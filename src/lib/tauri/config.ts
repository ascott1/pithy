import { invoke } from "@tauri-apps/api/core";

export interface EditorConfigInfo {
	fontSize: number;
	fontFamily: string;
	lineHeight: number;
}

export interface ConfigInfo {
	configPath: string;
	vaultDir: string;
	vaultDirDisplay: string;
	warning: string | null;
	editor: EditorConfigInfo;
}

export function getConfigInfo(): Promise<ConfigInfo> {
	return invoke<ConfigInfo>("get_config_info");
}

export function readConfigFile(): Promise<string> {
	return invoke<string>("read_config_file");
}

export function writeConfigFile(contents: string): Promise<void> {
	return invoke<void>("write_config_file", { contents });
}
