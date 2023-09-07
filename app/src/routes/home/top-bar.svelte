<script lang="ts">
    import {Link, TrashBin, AddCircle, Play, Pause, SettingsSharp} from 'svelte-ionicons';
    import {settings} from "../../settings";
    import Input from "../../components/input.svelte";

</script>

<div class="bar">
    <div class="actions">
    <button>
        <Link tabindex="-1" style="transform: scale(1.5)" color="#1e90ff"/>
    </button>
    <button>
        <AddCircle tabindex="-1" style="transform: scale(1.5)" color="#1e90ff"/>
    </button>
    <button>
        <TrashBin tabindex="-1" style="transform: scale(1.5)" color="#ff0000"/>
    </button>

    <div class="separator"/>

    <button>
        <Play tabindex="-1" style="transform: scale(1.5)" color="#32cd32"/>
    </button>

    <button>
        <Pause tabindex="-1" style="transform: scale(1.5)" color="#d2691e"/>
    </button>

    <div class="separator"/>

    <button>
        <SettingsSharp tabindex="-1" style="transform: scale(1.5)" color="#1e90ff"/>
    </button>
    </div>

    <div class="filter">
        <Input bind:value={$settings.filter} />
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