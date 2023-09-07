<script lang="ts">
    export let value = "";
</script>

<div class="d">
    <input bind:value/>
</div>

<style>
    .d {
        position: relative;
        border: 1px solid #000;
    }

    input {
        width: 100%;
        height: 100%;
        border: none;
        outline: none;
        background: transparent;
    }
</style>