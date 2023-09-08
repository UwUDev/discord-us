<script lang="ts">
    import {Link, TrashBin, AddCircle, Play, Pause, SettingsSharp, Search} from 'svelte-ionicons';
    import {settings} from "../../settings";
    import Input from "../../components/input.svelte";
    import {invoke} from "@tauri-apps/api"

</script>

<div class="bar">
    <div class="actions">
        <button>
            <Link tabindex="-1" color="#1e90ff"/>
        </button>
        <button on:click={() => {
        invoke("open_window", {url: `index.html?path=upload`, title: "Upload file"})
    }}>
            <AddCircle tabindex="-1" color="#1e90ff"/>
        </button>
        <button>
            <TrashBin tabindex="-1" color="#ff0000"/>
        </button>

        <div class="separator"/>

        <button>
            <Play tabindex="-1" color="#32cd32"/>
        </button>

        <button>
            <Pause tabindex="-1" color="#d2691e"/>
        </button>

        <div class="separator"/>

        <button on:click={() => {
        invoke("open_window", {url: `index.html?path=options`, title: "Options", label: "options" /* define label = 1 instance of window allowed*/})
    }}>
            <SettingsSharp tabindex="-1" color="#1e90ff" />
        </button>
    </div>

    <div class="filter">
        <Input bind:value={$settings.filter} placeholder="Filter waterfall names...">
            <Search size="20px" style="padding-left: 2px" color="#1e90ff" slot="before"/>
        </Input>
    </div>
</div>

<style>
    .bar {
        width: 100%;
        height: 36px;
        display: flex;
        padding-top: 2px;
        padding-bottom: 2px;
        align-items: center;
        justify-content: space-between;
    }

    .separator {
        width: 1px;
        margin-left: 4px;
        margin-right: 4px;
        height: calc(100% - 8px);
        background-color: #bbb;
    }

    .actions > button {
        border-radius: 2px;
        background-color: transparent;
        border: 1px solid transparent;
        width: 32px;
        height: 32px;
        display: flex;
        justify-content: center;
        align-items: center;
    }

    .actions > button > :global(svg) {
        transform: scale(1.5);
    }

    .actions > button:active > :global(svg) {
        transform: scale(1.5) translate3d(0.5px, 0.5px, 1px) !important;
    }

    .actions > button:hover {
        border-color: rgb(184, 240, 220);
        background-color: rgba(184, 255, 244, 0.4);
    }

    .actions {
        display: flex;
        align-items: center;
    }

    .filter {
        padding-right: 10px;
    }
</style>