<script lang="ts">

    import TopBar from "./top-bar.svelte";
    import {Shuffle, BookSharp} from "svelte-ionicons"
    import Transfers from "./transfers/transfers.svelte"
    import { handleDrop } from "./filedrop"
    import {onDestroy, onMount} from "svelte";
    import {appWindow} from "@tauri-apps/api/window";

    let view = 'transfers';

    let views = [["transfers", Shuffle], ["logs", BookSharp]] as const;

    let listenCancel: Promise<(() => void)> | undefined;

    onMount(() => {
        listenCancel = appWindow.onFileDropEvent(({payload})=> {
            if (payload.type === "drop") {
                handleDrop(payload.paths)
            }
        })
    })

    onDestroy(async () => {
        (await listenCancel)?.();
    })
</script>

<div>
    <TopBar/>
    <div style="margin: 2px 0; height: 1px;background-color: #bbb;width: 100% "/>

    <div class="v-selector">
        {#each views as [name, icon]}
            <div tabindex={0} aria-label={name} role="button" on:click={() => view = name} class="v-s"
                 class:selected={view === name}>
                <svelte:component tabindex="-1" this={icon} size="18" color="#1e90ff"/>
                <div style="text-align: center">{name}</div>
            </div>
        {/each}
    </div>

    <div class="container">
        {#if view === "transfers"}
            <Transfers/>
        {:else if view === "logs"}
            Logs
        {/if}
    </div>
</div>

<style>
    .v-selector {
        display: flex;
        flex-direction: row;
        padding: 0px 8px;
        align-items: flex-end;
    }

    .v-s:last-child {
        border-right: 1px solid rgba(176, 176, 176, 0.3);
    }

    .v-s {
        display: flex;
        gap: 4px;
        align-items: center;
        border-top: 1px solid rgba(176, 176, 176, 0.3);
        border-left: 1px solid rgba(176, 176, 176, 0.3);

        padding: 1px 2px;
        font-size: 11px;
        height: 20px;
        user-select: none;
    }

    .v-s:hover {
        background-color: rgba(176, 176, 176, 0.15);
    }

    .selected {
        height: 22px;
        border-right: 1px solid rgba(176, 176, 176, 0.3);
    }

    .selected + .v-s {
        border-left: 0;
    }

    .container {
        width: calc(100% - 16px);
        height: calc(100vh - 70px - 20px);
        margin: 0 8px 0 8px;
    }

</style>