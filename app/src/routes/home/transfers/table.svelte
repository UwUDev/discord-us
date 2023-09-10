<script lang="ts">
    import {ChevronDown} from "svelte-ionicons";
    import {invoke} from "@tauri-apps/api"

    import {settings} from "../../../settings";
    import Resizable from "../../../components/resizable/resizable.svelte";
    import ProgressBar from "../../../components/progressbar.svelte"

    import {Columns, ColumnsSort, displayColumnsSelector} from "./columns"
    import {onDestroy, onMount} from "svelte";
    import {listen} from "@tauri-apps/api/event";
    import prettyBytes from 'pretty-bytes';

    import {openActionContextMenu, selectedItems} from "./actions"


    $: filter = $settings.filter || undefined;

    $: get_items = invoke("get_items", {filter})

    let items: any[] = [];

    $: {
        if (get_items) {
            get_items.then((res: any[]) => {
                items = res;
            })
        }
    }

    let itemsProgression = {};

    $: {
        for (let k of Object.keys(itemsProgression)) {
            if (items.find((item) => item.id === k)) {
                itemsProgression[k] = undefined;
                delete itemsProgression[k];
            }
        }

        for (const i of items) {
            console.log(i);
            if (!i.progression_data || itemsProgression[i.id])
                continue;

            const data = JSON.parse(i.progression_data);

            itemsProgression[i.id] = {
                id: i.id,
                progress: data.progress,
                total: data.total,
                ranges: data.ranges
            }
        }
    }

    let unlistenFn: Promise<() => void>[] = [];

    onMount(() => {
        unlistenFn.push(listen<any>('push_item', (e) => {
            let index = items.findIndex((item) => item.id === e.payload.id);
            if (index !== -1) {
                items[index] = e.payload;
            } else {
                items = [...items, e.payload];
            }
        }));

        unlistenFn.push(listen<any>('remove_item', (e) => {
            items = items.filter((item) => item.id !== e.payload);
        }));

        unlistenFn.push(listen<{
            id: number;
            progress: number;
            total: number;
            ranges: [number, number][]
        }>('upload_progress', (e) => {
            console.log(e);
            itemsProgression[e.payload.id] = e.payload;
            console.log(itemsProgression);
        }));
    });

    onDestroy(() => {
        unlistenFn.forEach(async f => (await f)?.());
    });

    let drag = undefined;
    let dragStartX = undefined;
    let dragDeltaX = 0;
    let dragStart = false;
    $: dragColIndex = drag && $settings.transfers.columns.findIndex(x => x[0] === drag);
    $: dragStartV = dragColIndex >= 0 && $settings.transfers.columns.slice(0, dragColIndex).reduce((prev, v) => prev + v[1] + 2, 0);
    $: computedDragPosition = dragStartV + dragDeltaX;
    $: dragWidth = dragColIndex >= 0 && $settings.transfers.columns[dragColIndex][1] + 2;

    let itemAbove;

    $: {
        let acc = 0, i = 0;
        for (let [c, w] of $settings.transfers.columns) {
            let end = acc + w + 2;

            if (computedDragPosition > acc && computedDragPosition < end && c !== drag) {
                itemAbove = i;
                break;
            }

            acc = end;
            i++;
        }
    }

    $: sort = ColumnsSort[$settings.transfers.sort[0]] ?? ColumnsSort.default($settings.transfers.sort[0]);
    let sortedItems = [];
    $: {
        if (items) {
            let a = items.sort(sort).slice();
            sortedItems = $settings.transfers.sort[1] === "asc" ? a : a.reverse();
        }
    }


</script>

<svelte:window on:mousemove={(e) => {
    if(drag) {
        dragDeltaX = e.clientX - dragStartX;

        if(Math.abs(dragDeltaX) > 5) {
            dragStart = true;
        }
    }
}} on:mouseup={() => {
    if(dragStart && itemAbove>=0) {
        // swap dragColIndex and itemAbove
        let tmp = $settings.transfers.columns[dragColIndex];
        $settings.transfers.columns[dragColIndex] = $settings.transfers.columns[itemAbove];
        $settings.transfers.columns[itemAbove] = tmp;
    }

    drag= undefined;
    dragStart = false;
    itemAbove = undefined;
}} />

{#if dragColIndex >= 0 && dragStart}
    <div style="width: {dragWidth}px; height: 22px; left: {computedDragPosition}px"
         class="drag">
        {Columns[drag]}
    </div>
{/if}

<table>
    <thead>
    <tr on:contextmenu={(e) => {
          displayColumnsSelector({ x: e.clientX, y: e.clientY}, $settings.transfers.columns);
    }}>
        {#each $settings.transfers.columns as [column, width],i}
            {@const is_above =dragStart && i === itemAbove}

            <th style="width: {width}px;" class:above={is_above} on:mousedown={(e) => {
                drag = column;
                dragStartX = e.clientX;
                dragDeltaX = 0;
            }} on:mouseup={(e) => {
                if(dragStart || e.button == 2) {
                    return;
                }
                if($settings.transfers.sort[0] !== column) {
                    $settings.transfers.sort = [column, "asc"];
                } else {
                    $settings.transfers.sort[1] = $settings.transfers.sort[1] === "asc" ? "desc" : "asc";
                }
            }} class:dr={true}>
                <Resizable limits={[30, 1500]} bind:width={width} height="20px" resize_cords={["right"]}>
                    {#if $settings.transfers.sort[0] === column}
                        <div class="sort">
                            <ChevronDown
                                    style="transform: rotate({$settings.transfers.sort[1] === 'asc' ? '0deg' : '180deg'}); opacity: 75%"
                                    size="12px" color="#000"/>
                        </div>
                    {/if}
                    {Columns[column]}
                </Resizable>
            </th>
        {/each}
    </tr>
    </thead>

    <tbody>
    {#each sortedItems as item, i}
        {@const selected = $selectedItems.includes(item.id)}

        <tr class="item" on:click|stopPropagation={(e) => {
            console.log(e, e.ctrlKey)
            if (e.ctrlKey) {
                if(!selected) {
                    $selectedItems= [...$selectedItems, item.id];
                } else {
                    $selectedItems = $selectedItems.filter((i) => i !== item.id);
                }
            } else if (e.shiftKey && $selectedItems.length > 0) {
                let lastClicked = sortedItems.findIndex(item => item.id === $selectedItems[$selectedItems.length - 1]);
                let index = i;
                if (index > lastClicked) {
                    let tmp = lastClicked;
                    lastClicked = index;
                    index = tmp;
                }

                let push = [];
                let remove = [];
                for(let j = index; j <= lastClicked; j++) {
                    if(!$selectedItems.includes(sortedItems[j].id))
                        push.push(sortedItems[j].id);
                    else {
                        remove.push(sortedItems[j].id);
                    }
                };

                if (push.length === 0) {
                    // remove all items bet
                    $selectedItems = $selectedItems.filter((x) => !remove.includes(x));
                } else {
                    $selectedItems = [...$selectedItems, ...push]
                }

            } else {
                $selectedItems = [item.id];
            }
        }} class:selected on:contextmenu|preventDefault={(e) => {
            if(!selected)
                $selectedItems = [item.id];

            openActionContextMenu({
                x: e.clientX,
                y: e.clientY
            });
        }}>
            {#each $settings.transfers.columns as [column, width]}
                <td style="max-width: {width}px;">
                    <div class="v" style="max-width: {width-2}px;">
                        {#if column === "progress"}
                            <div style="height: 25px" class="progress">
                                <ProgressBar total={itemsProgression[item.id]?.total || 1}
                                             ranges={itemsProgression[item.id]?.ranges||[]}/>

                                <div class="p">
                                    {((itemsProgression[item.id]?.progress || 0) / (itemsProgression[item.id]?.total || 1) * 100).toFixed(2)}
                                    %
                                </div>
                            </div>
                        {:else if column === "size"}
                            {prettyBytes(itemsProgression[item.id]?.total || 0, {
                                space: true,
                                binary: true
                            })}
                        {:else if column === "uploaded"}
                            {prettyBytes(itemsProgression[item.id]?.progress || 0, {
                                space: true,
                                binary: true
                            })}
                        {:else}
                            {item[column]}
                        {/if}
                    </div>
                </td>
            {/each}
        </tr>
    {/each}
    </tbody>
</table>
<style>
    .sort {
        position: absolute;
        top: -8px;
        left: 50%;
        transform: translateX(-50%);
    }

    table {
        border-collapse: collapse;
        user-select: none;
    }

    th {
        border-left: 1px solid #bbb;
        border-right: 1px solid #bbb;

        font-size: 13px;
        font-weight: normal;
        color: #000;
    }

    td {
        padding-right: 2px;
    }

    .v {
        text-overflow: ellipsis;
        overflow: hidden;
        white-space: nowrap;
    }


    th:first-child {
        border-left: none;
    }


    th:hover {
        background-color: rgba(30, 144, 255, 0.2);
    }

    .progress {
        display: flex;
        align-items: center;
        justify-content: center;
        position: relative;
    }

    .progress > :global(canvas) {
        width: 100%;
        height: 100%;
    }

    .p {
        position: absolute;
        left: 50%;
        top: 50%;
        transform: translate(-50%, -50%);
        color: #fff;
    }

    .drag {
        position: absolute;
        background-color: rgba(30, 144, 255, 0.5);
    }

    table {
        border-collapse: collapse;
    }

    .item {
        border: 1px dashed transparent;
    }

    .selected {
        background-color: rgba(30, 144, 255, 0.2);
        border: 1px dashed black;
    }

    .item:has(+ .selected) {
        border-bottom: none;
    }

    .above {
        background-color: rgba(30, 144, 255, 0.5);
    }

</style>