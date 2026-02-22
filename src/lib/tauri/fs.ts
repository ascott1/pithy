import { invoke } from "@tauri-apps/api/core";

export function listFiles(): Promise<string[]> {
	return invoke<string[]>("list_files");
}

export function readFile(relPath: string): Promise<string> {
	return invoke<string>("read_file", { relPath });
}

export function saveFile(relPath: string, contents: string): Promise<void> {
	return invoke<void>("save_file", { relPath, contents });
}

export function renameFile(oldRelPath: string, newRelPath: string): Promise<void> {
	return invoke<void>("rename_file", { oldRelPath, newRelPath });
}

export function sanitizeFilename(name: string): Promise<string> {
	return invoke<string>("sanitize_filename", { name });
}
