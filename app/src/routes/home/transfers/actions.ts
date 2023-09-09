import {get, writable} from "svelte/store";
import {invoke} from "@tauri-apps/api";
import {listen} from "@tauri-apps/api/event";

export const selectedItems = writable([] as number[]);

listen("export_selection", async () => {
    const items = get(selectedItems);


    for (const item of items) {
       const r=  await invoke("open_window", {url: `index.html?path=export&exportid=${item}`, title: "Export to waterfall"})
        console.log(r, item, "open_window")
    }
})

export async function openActionContextMenu(pos: { x: number, y: number }) {
    const selected = get(selectedItems);

    await invoke("plugin:context_menu|show_context_menu", {
        pos,
        items: [
            {
                label: "Pause",
                event: "pause_selection",
                disabled: selected.length === 0
            },
            {
                label: "Resume",
                event: "resume_selection",
                disabled: selected.length === 0
            },
            {
                label: "Export to waterfall",
                event: "export_selection",
                disabled: selected.length === 0
            }
        ],
    })
}

export default {}