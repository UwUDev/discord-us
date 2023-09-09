<script lang="ts">
    import {ChevronDown} from "svelte-ionicons";
    import {invoke} from "@tauri-apps/api"

    import {settings} from "../../../settings";
    import Resizable from "../../../components/resizable/resizable.svelte";

    import {Columns, displayColumnsSelector} from "./columns"
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
            console.log(e)
            items.push(e.payload);
        }));

        unlistenFn.push(listen<{ id: number; progress: number; total: number; ranges: [number, number][] }>('upload_progress', (e) => {
            console.log(e);
            itemsProgression[e.payload.id] = e.payload;
            console.log(itemsProgression);
        }));
    });

    onDestroy(() => {
        unlistenFn.forEach(async f => (await f)?.());
    })

</script>

<table>
    <thead>
    <tr on:contextmenu|preventDefault={(e) => {
          displayColumnsSelector({ x: e.clientX, y: e.clientY}, $settings.transfers.columns);
    }}>
        {#each $settings.transfers.columns as [column, width]}
            <th style="width: {width}px;" on:mouseup={() => {
                if($settings.transfers.sort[0] !== column) {
                    $settings.transfers.sort = [column, "asc"];
                } else {
                    $settings.transfers.sort[1] = $settings.transfers.sort[1] === "asc" ? "desc" : "asc";
                }
            }}>
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
    {#each items as item}
        <tr on:click={() => {
            $selectedItems = [item.id];
        }} on:contextmenu|preventDefault={(e) => {
            if(!$selectedItems.includes(item.id))
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
                            {(itemsProgression[item.id]?.progress || 0) / (itemsProgression[item.id]?.total || 1) * 100}
                            %
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
</style>