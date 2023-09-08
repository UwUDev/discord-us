<script lang="ts">
    import {params} from "../../params";
    import Input from "../../components/input.svelte"
    import {invoke} from "@tauri-apps/api"
    import {afterUpdate, onDestroy, onMount} from "svelte";
    import {listen} from "@tauri-apps/api/event";
    import Button from "../../components/button.svelte";
    import {appWindow} from '@tauri-apps/api/window';


    let fpath = $params.fpath as string || '';

    let cancel: Promise<() => void> | undefined;
    let callbackx = Date.now();

    onMount(() => {
        cancel = listen<{ callback?: string; path: string }>("file-picked", ({payload: x}) => {
            console.log(x)
            if (x.callback === "upload-file-" + callbackx) {
                fpath = x.path;
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

    let threads = 2;

    let password = undefined;
</script>

<div class="page">
    <div class="form">
        <div class="label">Select file to upload</div>

        <div class="value">
            <span>File: </span>
            <Input bind:value={fpath}/>
        </div>

        <div class="value">
            <div style="width: 100%"/>
            <Button on:click={() => invoke("pick_file", { cb: "upload-file-" + callbackx})}>Open file</Button>
        </div>
    </div>

    <div class="form">
        <div class="label">Upload parameters</div>

        <div class="value">
            <span>Threads count: </span>
            <Input>
                <input type="number" bind:value={threads} slot="input"/>
            </Input>
        </div>
    </div>

    <div class="form">
        <div class="label">Encryption parameters</div>

        <div class="form" class:disabled={password === undefined} style="margin-top: 16px">
            <div class="label"> <input type="checkbox" on:input={e => password =  e.target.checked ? '': undefined} > Protect with a password</div>
            <div class="value">
                <span>Password: </span>
                <Input>
                    <input type="password" bind:value={password} slot="input"/>
                </Input>
            </div>
        </div>
    </div>

    <div class="submit" style="margin-top: auto">
        <Button disabled={!fpath} on:click={() => invoke("upload_file", {
            payload: {
                file_path: fpath,
                thread_count: threads,
                password,
            },
        }).then(() => appWindow.close())}>
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