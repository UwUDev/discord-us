import {resolveResource} from "@tauri-apps/api/path";
import {invoke} from "@tauri-apps/api";
import {listen} from "@tauri-apps/api/event"
import {settings} from "../../../settings";

export const Columns = {
    name: "Name",
    size: "Size",
    progress: "Progress",
    status: "Status",
    down_speed: "Down speed",
    up_speed: "Up speed",
    eta: "Eta",
    uploaded: "Uploaded"
}

export const ColumnsSort = {
    default: (col: string) => (item1, item2) => item1[col]  - item2[col],
} as const;

Object.keys(Columns).forEach((key) => {
    listen("toggle_column_" + key, () => {
        console.log("toggle_column_" + key);
        settings.update((setting: any) => {
            let cols = setting.transfers.columns as [string, number][];

            let exist = cols.some(x => x[0] === key);

            if (exist) {
                cols = cols.filter(x => x[0] !== key);
            } else {
                cols.push([key, 80]);
            }

            return {
                ...setting,
                transfers: {
                    ...setting.transfers,
                    columns: cols
                }
            }
        })
    })
})

export async function displayColumnsSelector(pos: { x: number, y: number }, columns: [string, number][]) {
    const selected = columns.map(x => x[0]);
    const iconUrl = await resolveResource('assets/checked_16x16.png');

    const checked = {path: iconUrl};

    invoke("plugin:context_menu|show_context_menu", {
        pos,
        items: Object.entries(Columns).map(([name, col]) => ({
            icon: selected.includes(name) ? checked : undefined,
            label: col,
            event: "toggle_column_" + name,
        }))
    })
}

export default {}