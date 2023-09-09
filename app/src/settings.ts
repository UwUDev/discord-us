import {writable, get} from 'svelte/store';
import {invoke} from '@tauri-apps/api/tauri'
import { listen} from "@tauri-apps/api/event";
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
        ] as [string, number][],
        sort: ["name", "asc"]
    },

    options: {
        tabOpen: 'Credentials',
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

let noUpdate = false;

listen<string>('settings-updated', (event) => {
    const json = JSON.parse(event.payload);
    noUpdate = true;
    settings.set(json);
});

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

const settingsDebounce = createDebounce(10);

settings.subscribe((value) => {
    if (noUpdate) {
        noUpdate = false;
        return;
    }
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