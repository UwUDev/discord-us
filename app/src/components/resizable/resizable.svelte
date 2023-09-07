<script lang="ts">

    type Cord = "right" | "top";

    export let resize_cords: Cord[];
    export let width: string | number;
    export let height: string | number;

    export let limits: [number, number] = [20, 400];

    let drag: Cord = null;
    let pos = null;
    let s: [number, number] = null;

    let elem: HTMLDivElement;

    function captureSize() {
        const wPx = typeof width === "string" ? parseInt(width.slice(0, -1)) * elem.parentElement.clientWidth / 100 : width;
        const hPx = typeof height === "string" ? parseInt(height.slice(0, -1)) * elem.parentElement.clientHeight / 100 : height;

        return [wPx, hPx];
    }

    function handleMove(e) {
        if (!drag || !pos) return;
        const deltaX = e.clientX - pos[0];
        const deltaY = e.clientY - pos[1];

        if (drag === "right") {
            const fWidth = elem.parentElement.clientWidth;

            const p = Math.min(limits[1], Math.max(limits[0], s[0] + deltaX));

            if (typeof width === "string") {
                width = `${p / fWidth * 100}%`
            } else {
                width = p;
            }

        } else if (drag === "top") {
            height = `${parseInt(height as string) + deltaY}px`;
        }
    }
</script>

<svelte:window on:mousemove={handleMove} on:mouseup={(e) => {
    if (drag) {e.stopImmediatePropagation(); e.preventDefault()}
    drag = null}}/>

<div bind:this={elem} class="resizable"
     style="width: {width}; height: {height}; user-select: {drag ? 'none':'auto'}">
    <div class="content">
        <slot/>
    </div>
    {#each resize_cords as cord}
        <div on:mousedown|stopPropagation|preventDefault={(e) => {drag = cord; pos = [e.clientX, e.clientY]; s = captureSize()}}

             class="handle"
             style="right:-6px; top:0px; width: {cord === 'right' ? '10px' : width}; height:{cord === 'top' ? '10px': height}; cursor: {cord === 'right' ? 'col-resize' : 'row-resize'}">

        </div>
    {/each}
</div>

<style>
    .resizable {
        position: relative;
        height: 100%;
        width: 100%;
    }

    .content {
        position: absolute;
        height: 100%;
        width: 100%;
    }

    .handle {
        position: absolute;
        z-index: 10;
        /*background-color: rgba(0, 0, 0, 0.2);*/
    }
</style>