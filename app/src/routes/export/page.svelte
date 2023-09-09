<script lang="ts">
    import {params} from "../../params";
    import {invoke} from "@tauri-apps/api";
    import {appWindow} from '@tauri-apps/api/window';
    import Input from "../../components/input.svelte";
    import Button from "../../components/button.svelte";
    import {onDestroy, onMount} from "svelte";
    import {listen} from "@tauri-apps/api/event";

    let id = parseInt($params.exportid);

    let item = undefined;

    $: {
        invoke("get_item", {id}).catch((e) => console.log(e)).then((res) => {
            item = res;
            appWindow.show();
        })
    }

    let exportPath = "";
    let password;

    $: {
        if (!item?.user_password) {
            password = item?.password
        }
    }

    let cancel: Promise<() => void> | undefined;
    let callbackx = Date.now();

    onMount(() => {
        cancel = listen<{ callback?: string; path: string }>("file-picked", ({payload: x}) => {
            if (x.callback === "upload-file-" + callbackx) {
                exportPath = x.path;
            }
        });
    })

    onDestroy(async () => {
        (await cancel)?.();
    })

</script>

{#if item}
    <div class="page">
        <div class="form">
            <div class="label">Export item</div>

            <div>Name: {item.name}</div>
        </div>

        <div class="form">
            <div class="label">Waterfall</div>

            <div class="value">
                <span>Waterfall: </span>
                <Input bind:value={exportPath}/>
            </div>

            <div class="value">
                <div style="width: 100%"/>
                <Button on:click={() => invoke("save_file_picker", { cb: "upload-file-" + callbackx, extensions: ["waterfall"]})}>
                    Open file
                </Button>
            </div>
        </div>

        {#if item.user_password}
            <div class="form">
                <div class="label">Encryption settings</div>

                <div >
                    <input type="checkbox" on:input={e => password =  e.target.checked ? item.password : undefined}/> Export with
                    password
                </div>
            </div>
        {/if}

        <div class="submit" style="margin-top: auto">
            <Button disabled={!exportPath} on:click={() => invoke("export_waterfall", {
                waterfallPath: exportPath,
                password,
                itemId: id,
            }).then(() => appWindow.close())}>
                Export waterfall
            </Button>

            <Button on:click={()=>appWindow.close()}>
                Cancel
            </Button>
        </div>
    </div>
{/if}

<style>
    .page {
        padding: 16px 8px;

        height: calc(100vh - 32px);
    }

    .submit > :global(button) {
        white-space: nowrap;
        width: 100px;
    }
</style>