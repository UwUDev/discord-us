import {writable} from 'svelte/store';
import {invoke} from '@tauri-apps/api/tauri'
import deepmerge from "deepmerge";

const Settings = {
    statusWitdh: "15%",

    leftBar: {
        statusOpen: true
    }
} as const

export const settings = writable(Settings);

export const settingsLoaded = writable(false);


export async function loadSettings() {
    const str = await invoke('get_settings') as string | undefined;

    if (str) {
        const json = JSON.parse(str);
        settings.update((s) => deepmerge(s, json));
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

const settingsDebounce = createDebounce(250);

settings.subscribe((value) => {
    // send update to rust
    // to save settings
    const jsonValue = JSON.stringify(value);

    settingsDebounce(() => {
        invoke('save_settings', {settings: jsonValue});
    });
});

export default {};