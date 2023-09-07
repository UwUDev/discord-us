<script lang="ts">
    import {params} from "../../params";
    import Input from "../../components/input.svelte"
    import {invoke} from "@tauri-apps/api"
    import {onDestroy, onMount} from "svelte";
    import {listen} from "@tauri-apps/api/event";
    import Button from "../../components/button.svelte"

    let value = $params.fpath as string||'';

    let cancel: Promise<() => void> | undefined;
    let callbackx = Date.now();

    onMount(() => {
        cancel = listen<{ callback?: string; path: string }>("file-picked", ({payload: x}) => {
            console.log(x)
            if (x.callback === "upload-file-" + callbackx) {
                value = x.path;
            }
        });
    })

    onDestroy(async () => {
        (await cancel)?.();
    })
</script>

<div class="page">
    <div class="form">
        <div class="label">Select file to upload</div>

        <div class="value">
            <span>File: </span>
            <Input bind:value/>
        </div>

        <div class="value">
            <div style="width: 100%"/>
            <Button on:click={() => invoke("pick_file", { cb: "upload-file-" + callbackx})}>Open file</Button>
        </div>
    </div>

    <div class="submit">
        <Button>
            Upload file
        </Button>

        <Button class="submit">
            Cancel
        </Button>
    </div>
</div>

<style>
    .page {
        padding: 16px 8px;
        background-color: rgb(230, 230, 230);
        height: calc(100vh - 32px);
        display: flex;
        flex-direction: column;
        justify-content: space-between;
    }

    .form {
        position: relative;
        border: 1px solid #bbb;
        padding: 8px;
        display: flex;
        padding-top: 12px;
        font-size: 13px;
        flex-direction: column;
        gap: 8px;
    }

    .value {
        display: flex;
        width: 100%;
        justify-content: space-between;
        gap: 8px;
    }

    .value > :global(button) {
        width: 100px;
    }

    .label {
        position: absolute;
        top: -12px;
        left: 5px;
        background-color: rgb(230, 230, 230);
        font-size: 12px;
    }

    .submit {
        display: flex;
        justify-content: center;
        align-items: center;
        margin-top: 8px;
        gap: 8px;
    }

    .submit > :global(button) {
        width: 80px;
    }
</style>