import { invoke } from "@tauri-apps/api/core";

export interface EditorConfigInfo {
	fontSize: number;
	fontFamily: string;
	lineHeight: number;
}

export interface ThemeConfigInfo {
	mode: string;
	lightCss: string;
	darkCss: string;
}

export interface DailyConfigInfo {
	dir: string;
	format: string;
}

export interface StatusBarConfigInfo {
	show: boolean;
	showBacklinks: boolean;
	showWordCount: boolean;
}

export interface ConfigInfo {
	configPath: string;
	vaultDir: string;
	vaultDirDisplay: string;
	warning: string | null;
	editor: EditorConfigInfo;
	theme: ThemeConfigInfo;
	daily: DailyConfigInfo;
	autoUpdateLinks: boolean;
	statusBar: StatusBarConfigInfo;
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
