import {writable, get} from 'svelte/store';
import {invoke} from '@tauri-apps/api/tauri'
import deepmerge from "deepmerge";

const Settings = {
    statusWitdh: "15%",

    filter: '',

    leftBar: {
        statusOpen: true
    },

    transfers: {
        columns: [
            ["name", 140],
            ["size", 80],
        ],
        sort: ["name", "asc"]
    }
} as const

export const settings = writable(Settings);

export const settingsLoaded = writable(false);


export async function loadSettings() {
    const str = await invoke('get_settings') as string | undefined;

    if (str) {
        const json = JSON.parse(str);
        settings.update((s) => deepmerge(s, json, {
            arrayMerge(target: any[], source: any[], options?: deepmerge.ArrayMergeOptions): any[] {
                return source;
            }
        }));
    }
}

loadSettings().then(() => {
    settingsLoaded.set(true)
});

function createDebounce(d: number) {
    let lastCallback;
    let timeout;

    return (cb: () => void) => {
        lastCallback = cb;
        if (!timeout) {
            timeout = setTimeout(() => {
                lastCallback();
                timeout = null;
            }, d);
        }
    }
}

const settingsDebounce = createDebounce(1000);

settings.subscribe((value) => {
    // send update to rust
    // to save settings
    const jsonValue = JSON.stringify(value);

    settingsDebounce(() => {
        invoke('save_settings', {settings: jsonValue});
    });
});

window.addEventListener("beforeunload", () => {
    invoke('save_settings', {settings: JSON.stringify(get(settings))});
})

export default {};