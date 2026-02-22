import { describe, it, expect, vi } from "vitest";

vi.mock("@tauri-apps/api/core", () => ({
	invoke: vi.fn(),
}));

import { invoke } from "@tauri-apps/api/core";
import { listFiles, readFile, saveFile, renameFile, sanitizeFilename } from "./fs";

describe("Tauri fs wrappers", () => {
	it("listFiles invokes list_files", async () => {
		vi.mocked(invoke).mockResolvedValueOnce(["welcome.md"]);
		await expect(listFiles()).resolves.toEqual(["welcome.md"]);
		expect(invoke).toHaveBeenCalledWith("list_files");
	});

	it("readFile invokes read_file with relPath", async () => {
		vi.mocked(invoke).mockResolvedValueOnce("# hi");
		await expect(readFile("a.md")).resolves.toBe("# hi");
		expect(invoke).toHaveBeenCalledWith("read_file", { relPath: "a.md" });
	});

	it("saveFile invokes save_file with relPath and contents", async () => {
		vi.mocked(invoke).mockResolvedValueOnce(undefined);
		await expect(saveFile("a.md", "body")).resolves.toBeUndefined();
		expect(invoke).toHaveBeenCalledWith("save_file", { relPath: "a.md", contents: "body" });
	});

	it("renameFile invokes rename_file with old and new paths", async () => {
		vi.mocked(invoke).mockResolvedValueOnce(undefined);
		await expect(renameFile("a.md", "b.md")).resolves.toBeUndefined();
		expect(invoke).toHaveBeenCalledWith("rename_file", { oldRelPath: "a.md", newRelPath: "b.md" });
	});

	it("sanitizeFilename invokes sanitize_filename with name", async () => {
		vi.mocked(invoke).mockResolvedValueOnce("hello-world");
		await expect(sanitizeFilename("Hello World.md")).resolves.toBe("hello-world");
		expect(invoke).toHaveBeenCalledWith("sanitize_filename", { name: "Hello World.md" });
	});
});
