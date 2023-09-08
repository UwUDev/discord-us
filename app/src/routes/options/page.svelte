<script lang="ts">

    import {afterUpdate, setContext} from "svelte";
    import {appWindow} from "@tauri-apps/api/window";
    import {settings} from "../../settings";
    import {LockClosed,Information} from "svelte-ionicons"
    import Cred from "./cred.svelte";
    import Button from "../../components/button.svelte";
    import {writable} from "svelte/store";
    import {invoke} from "@tauri-apps/api";

    let u = false;
    afterUpdate(() => {
        if (!u) {
            u = true;
            appWindow.show();
        }
    })

    const Tabs = [
        ["Credentials", LockClosed, Cred],
        ["About", Information, LockClosed],
    ] as const

    let edited = writable({});

    setContext("edit", edited);

    $: isEdited = Object.keys($edited).length > 0;

    async function save () {
        if(!isEdited) return;

        let options = Object.entries($edited).flat();
        await invoke('set_options', { options });
        // reload window ?
        window.location.reload();
    }
</script>

<div class="w">
    <div class="w2">
        <div class="tabs">
            {#each Tabs as [name, icon,]}
                <button class:selected={$settings.options.tabOpen === name}
                        on:click={() => $settings.options.tabOpen = name}>
                    <svelte:component tabindex={-1} this={icon}/>
                    <span>{name}</span>
                </button>
            {/each}

        </div>

        <div class="c">
            {#each Tabs as [name, icon, cmp]}

                {#if $settings.options.tabOpen === name}

                    <svelte:component this={cmp}/>

                {/if}
            {/each}
        </div>
    </div>

    <div class="footer">
        <Button on:click={() => save().then(() => appWindow.close())}>Ok</Button>
        <Button on:click={() => appWindow.close()}>Cancel</Button>
        <Button disabled={!isEdited} on:click={save}>Apply</Button>
    </div>
</div>


<style>
    .w {
        width: calc(100vw - 16px);
        height: calc(100vh - 32px);
        background-color: rgb(230, 230, 230);
        padding: 16px 8px;
        display: flex;
        flex-direction: column;
    }

    .w2 {
        display: flex;
        gap: 12px;
        width: 100%;
        height: 100%;
    }

    .footer {
        margin-left: auto;
        padding-top: 4px;
    }

    .footer > :global(button) {
        width: 80px;
    }

    .c {
        width: calc(100% - 16px);
        padding: 16px 8px;
        flex: 1 auto;
        border: 1px solid #bbb;
    }


    .c > :global(div) {
        width: 100%;
        height: 100%;
    }

    .tabs {
        width: 200px;
        display: flex;
        flex-direction: column;
        align-items: center;
        flex-grow: 0;
        position: relative;
        gap: 16px;
        color: #1e90ff;
        background-color: #fff;
        border: 1px solid #bbb;
        padding: 1px 0;
    }

    .tabs > button {
        padding: 16px 8px;
        display: flex;
        flex-direction: column;
        align-items: center;
        gap: 8px;

        /*width: 60%;*/
        border: 1px solid transparent;
        background-color: transparent;
        border-radius: 4px;
    }

    .tabs > button > :global(svg) {
        color: #1e90ff;
    }

    .tabs > button.selected {
        background-color: rgba(187, 187, 187, 0.2);
    }

    .tabs > button.selected:hover {
        border-color: rgba(30, 144, 255, 0.5);
    }

    .tabs > button:hover {
        background-color: rgba(30, 144, 255, 0.1);
    }
</style>