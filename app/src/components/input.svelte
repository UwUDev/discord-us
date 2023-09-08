<script lang="ts">
    import { Eye } from "svelte-ionicons";

    export let value = "";
    export let hide = false;
    export let placeholder: string | undefined = undefined;

    let hidden = true;
</script>

<div class="d">
    <slot name="before"/>
    <div class="b">
        <input placeholder={placeholder} class:hide={hide} class:hidden={hide && hidden} bind:value on:input/>
        {#if hide}
            <div class="fake" class:h={!hidden}>{"•".repeat(value.length)}</div>
            <div class="view" on:click={()=>hidden=!hidden}><Eye /></div>
        {/if}
    </div>
    <slot name="after"/>
</div>

<style>
    .d {
        position: relative;
        border-bottom: 1px solid #bbb;
        width: 100%;
        background-color: white;
        display: flex;
        align-items: center;
        justify-content: center;
        flex-direction: row;
    }

    .b {
        width: 100%;
        height: 100%;
        border: none;
        outline: none;
        background: transparent;
        padding: 2px 4px;
        height: 20px;
        position: relative;
        display: flex;
        justify-content: center;
        align-items: center;
    }

    input {
        width: 100%;
        height: 100%;
        border: none;
        outline: none;
        background: transparent;
        caret-color: black;
    }

    .d:has( input:focus) {
        border-bottom: 1px solid rgba(30, 144, 255, 1);
    }

    .d:has(.hide) {
        position: relative;
    }

    .hide {
        font-family: monospace;
    }

    .h {
        display: none;
    }

    .fake {
        position: absolute;
        left: 6px;
        top: 1px;

        user-select: none;
        pointer-events: none;
        font-family: monospace;
    }

    .view {
        position: absolute;
        right: 4px;
        top: 0;
    }

    .hidden {
        color: transparent;
    }

    .hidden::selection {
        color: transparent;
        background-color: #1e90ff;
    }
</style>