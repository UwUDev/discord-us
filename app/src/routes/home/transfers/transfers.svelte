<script lang="ts">
    import LeftBar from "./left-bar.svelte";
    import Table from "./table.svelte";
</script>

<div class="container">
    <LeftBar />

    <div class="transfers">
        <Table />
    </div>
</div>

<style>
    .container {
        width: 100%;
        height: 100%;
        display: flex;
    }

    .transfers {
        width: 100%;
        height: 100%;
        border: 1px solid #000;
        margin-left: 1px;
        position: relative;
    }
</style>