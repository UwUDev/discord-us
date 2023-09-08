<script lang="ts">
    export let disabled: boolean = false;
</script>

<button on:click {disabled}>
    <slot />
</button>

<style>
    button {
        background: #fff;
        border: 1px solid #ccc;
        border-radius: 4px;
        padding: 2px 1px;

        transition: all .1s;
    }
    button:hover:not(:disabled) {
        background-color: rgba(30, 144, 255, 0.1);
        border-color:  rgba(30, 144, 255, 1);
    }
</style>