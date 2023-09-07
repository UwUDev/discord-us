<script lang="ts">
    import {Router, Route} from "svelte-routing";
    import Home from "./routes/home/page.svelte"
    import Upload from "./routes/upload/page.svelte"
    import {settingsLoaded} from "./settings"
    import { params } from "./params"

    let url = $params.path || '/';
</script>

{#if $settingsLoaded}
    <Router {url}>
        <Route path="/">
            <Home/>
        </Route>

        <Route path="upload">
            <Upload />
        </Route>
    </Router>
{/if}
