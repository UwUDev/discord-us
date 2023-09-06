<script lang="ts">
    import {Router, Route} from "svelte-routing";
    import Home from "./routes/home/page.svelte"
    import {settingsLoaded} from "./settings"

    export let url = "";
</script>

{#if $settingsLoaded}
    <Router {url}>
        <Route path="/">
            <Home/>
        </Route>
    </Router>
{/if}
