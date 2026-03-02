import { invoke } from "@tauri-apps/api/core";

export function setTitlebarOpacity(opacity: number): Promise<void> {
	return invoke("set_titlebar_opacity", { opacity });
}
