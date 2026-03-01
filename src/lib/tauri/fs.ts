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

export function deleteFile(relPath: string): Promise<void> {
	return invoke<void>("delete_file", { relPath });
}

export function renameFile(oldRelPath: string, newRelPath: string): Promise<void> {
	return invoke<void>("rename_file", { oldRelPath, newRelPath });
}

export function sanitizeFilename(name: string): Promise<string> {
	return invoke<string>("sanitize_filename", { name });
}

export interface WikilinkReference {
	relPath: string;
	count: number;
}

export function findWikilinkReferences(oldStem: string): Promise<WikilinkReference[]> {
	return invoke<WikilinkReference[]>("find_wikilink_references", { oldStem });
}

export function updateWikilinkReferences(oldStem: string, newStem: string): Promise<string[]> {
	return invoke<string[]>("update_wikilink_references", { oldStem, newStem });
}

export function copyImageToAssets(sourcePath: string, filename: string): Promise<string> {
	return invoke<string>("copy_image_to_assets", { sourcePath, filename });
}
