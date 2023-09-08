<script lang="ts">
    import {params} from "../../params";
    import Input from "../../components/input.svelte"
    import {invoke} from "@tauri-apps/api"
    import {afterUpdate, onDestroy, onMount} from "svelte";
    import {listen} from "@tauri-apps/api/event";
    import Button from "../../components/button.svelte";
    import {appWindow} from '@tauri-apps/api/window';


    let value = $params.fpath as string || '';

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

    let u = false;
    afterUpdate(() => {
        if (!u) {
            u = true;
            appWindow.show();
        }
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
        <Button on:click={() => invoke("upload_file", {
            path: value,
        })}>
            Upload file
        </Button>

        <Button on:click={()=>appWindow.close()} class="submit">
            Cancel
        </Button>
    </div>
</div>

<style>
    .page {
        padding: 16px 8px;

        height: calc(100vh - 32px);
    }
</style>