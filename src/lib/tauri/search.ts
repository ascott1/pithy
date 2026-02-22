import { invoke } from "@tauri-apps/api/core";

export interface SearchHit {
	path: string;
	filenameStem: string;
	snippet: string | null;
	tags: string[];
	modifiedDate: number;
	score: number;
}

export interface SearchResponse {
	hits: SearchHit[];
	query: string;
}

export function searchQuery(queryText: string, limit?: number, offset?: number): Promise<SearchResponse> {
	return invoke<SearchResponse>("search_query", { queryText, limit, offset });
}

export function searchStatus(): Promise<string> {
	return invoke<string>("search_status");
}

export function searchRebuild(): Promise<void> {
	return invoke<void>("search_rebuild");
}
