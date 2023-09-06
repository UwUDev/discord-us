<script lang="ts">
    import {settings} from "../../../settings";

    import Resizable from "../../../components/resizable/resizable.svelte";
    import Collapse from "../../../components/collapse.svelte";

</script>

<Resizable limits={[100, 400]} resize_cords={["right"]} height="100%" bind:width={$settings.statusWitdh} >
    <div class="bar">
        <Collapse bind:open={$settings.leftBar.statusOpen}>
            <span slot="trigger">status</span>

            <div>AA</div>
        </Collapse>
    </div>
</Resizable>

<style>
    .bar {
        background-color: #f5f5f5;
        height: 100%;
        width: 100%;
    }
</style>