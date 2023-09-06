<script lang="ts">
    import {ChevronDown} from "svelte-ionicons"

    export let open = false;
</script>

<div>
    <div class="trigger" on:click={() => open = !open}>
        <ChevronDown size="16" style="transform: rotate({open ? '0deg' : '-90deg'})"/>
        <slot name="trigger"/>
    </div>

    <div>
        {#if open}
            <slot/>
        {/if}
    </div>
</div>

<style>
    .trigger {
        display: flex;
        align-items: center;
        cursor: pointer;
        gap: 4px;
        width: 100%;
        user-select: none;
    }
</style>