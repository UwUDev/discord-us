<script lang="ts">
    export let value = "";
    export let placeholder: string | undefined = undefined;
</script>

<div class="d">
    <slot name="before"/>
    <input placeholder={placeholder} bind:value/>
    <slot name="after"/>
</div>

<style>
    .d {
        position: relative;
        border-bottom: 1px solid #bbb;
        width: 100%;
        background-color: white;
        display: flex;
        align-items: center;
        justify-content: center;
        flex-direction: row;
    }

    input {
        width: 100%;
        height: 100%;
        border: none;
        outline: none;
        background: transparent;
        padding: 2px 4px;
        height: 20px;
    }

    .d:has( input:focus) {
        border-bottom: 1px solid rgba(30, 144, 255, 1);
    }

</style>