import { invoke } from "@tauri-apps/api"

export function handleDrop (paths: string[]) {
    for (const path of paths) {
        invoke("handle_file_drop", { path });
    }
}

export default {}