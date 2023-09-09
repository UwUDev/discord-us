<script lang="ts">

    import {getContext, onMount} from "svelte";
    import {invoke} from "@tauri-apps/api";
    import Input from "../../components/input.svelte";
    import type {Writable} from "svelte/store"

    let options: Promise<string[]> | undefined;

    const edit = getContext<Writable<any>>('edit');

    onMount(() => {
        options = invoke('get_options', {options: ["upload_token", "channel_id", "account_type"]}).then((v) => ({
            upload_token: v[0],
            channel_id: v[1],
            account_type: v[2]
        })).then(x => {
            console.log('rx',x)
            return x;
        });
    })

    async function set_v(n: string, v: string) {
        const o = (await options)[n];

        if (v === o) {
            $edit = delete $edit[n] && $edit;
        } else {
            $edit[n] = v;
        }
    }

</script>

{#if options}
    {#await options}
        ...
    {:then o}
        <div class="page">

            <div class="form">
                <div class="label">Discord credentials</div>

                <div class="value">
                    Here you can configure your discord credentials.
                </div>

                <div class="value">
                    <span>Discord token: </span>
                    <Input hide value={$edit['upload_token'] || o['upload_token'] || ''} on:input={(e) => set_v('upload_token', e.target.value)}
                           placeholder="Token"/>
                </div>

                <div class="value">
                    <span>Channel id: </span>
                    <Input value={$edit['channel_id'] || o['channel_id']||''} on:input={(e) => set_v('channel_id', e.target.value)} placeholder="Channel id"/>
                </div>

                <div class="value">
                    <span>Account type: </span>
                    <Input>
                    <select slot="input" on:change={(e) => set_v('account_type', e.target.value)}>
                        {#each ["","Free", "Basic", "Classic", "Boost"] as v}
                            <option value={v} selected={$edit['account_type'] === v || o['account_type'] === v}>{v}</option>
                        {/each}
                    </select>
                    </Input>
                </div>


            </div>


        </div>
    {/await}
{/if}

<style>
    .value > span {
        white-space: nowrap;
    }


</style>