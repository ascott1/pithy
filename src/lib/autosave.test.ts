import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";

vi.mock("$lib/tauri/fs", () => ({
	saveFile: vi.fn(),
}));

import { saveFile } from "$lib/tauri/fs";
import { AutoSaveController } from "./autosave";

describe("AutoSaveController", () => {
	beforeEach(() => {
		vi.useFakeTimers();
		vi.mocked(saveFile).mockResolvedValue(undefined);
	});

	afterEach(() => {
		vi.useRealTimers();
	});

	it("debounces and saves only the latest doc", async () => {
		const c = new AutoSaveController(100);
		c.setOpenedFile("note.md", "one");

		c.setDoc("two");
		c.setDoc("three");

		expect(saveFile).not.toHaveBeenCalled();
		expect(c.state).toBe("debouncing");

		await vi.advanceTimersByTimeAsync(100);
		await c.flushAndWait();

		expect(saveFile).toHaveBeenCalledTimes(1);
		expect(saveFile).toHaveBeenCalledWith("note.md", "three");
		expect(c.state).toBe("idle");
		expect(c.dirty).toBe(false);
	});

	it("does not save when doc matches baseline", () => {
		const c = new AutoSaveController(100);
		c.setOpenedFile("note.md", "hello");

		c.setDoc("hello");

		expect(c.state).toBe("idle");
		expect(saveFile).not.toHaveBeenCalled();
	});

	it("flushAndWait resolves after save completes", async () => {
		const c = new AutoSaveController(100);
		c.setOpenedFile("note.md", "");
		c.setDoc("changed");

		const flushed = c.flushAndWait();
		await vi.advanceTimersByTimeAsync(0);
		await flushed;

		expect(saveFile).toHaveBeenCalledWith("note.md", "changed");
		expect(c.state).toBe("idle");
	});

	it("reports error state on save failure", async () => {
		vi.mocked(saveFile).mockRejectedValueOnce(new Error("disk full"));

		const c = new AutoSaveController(100);
		c.setOpenedFile("note.md", "");
		c.setDoc("fail");

		await vi.advanceTimersByTimeAsync(100);
		await vi.advanceTimersByTimeAsync(0);

		expect(c.state).toBe("error");
		expect(c.error).toBe("Error: disk full");
	});

	it("resets generation on file switch to discard stale saves", async () => {
		let resolveFirst!: () => void;
		vi.mocked(saveFile).mockImplementationOnce(
			() => new Promise<void>((r) => (resolveFirst = r)),
		);

		const c = new AutoSaveController(100);
		c.setOpenedFile("a.md", "");
		c.setDoc("change-a");

		await vi.advanceTimersByTimeAsync(100);

		// Switch to a new file while save is in-flight
		c.setOpenedFile("b.md", "fresh");
		resolveFirst();
		await vi.advanceTimersByTimeAsync(0);

		// The stale save should not update lastSavedDoc for the new file
		expect(c.state).toBe("idle");
		expect(c.dirty).toBe(false);
	});
});
