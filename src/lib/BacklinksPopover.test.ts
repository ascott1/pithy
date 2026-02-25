import { describe, it, expect, vi, afterEach } from "vitest";
import { render, screen, cleanup, fireEvent } from "@testing-library/svelte";
import BacklinksPopover from "./BacklinksPopover.svelte";
import type { WikilinkReference } from "$lib/tauri/fs";

afterEach(cleanup);

const refs: WikilinkReference[] = [
	{ relPath: "project-kickoff.md", count: 2 },
	{ relPath: "notes/meeting-notes.md", count: 1 },
	{ relPath: "ideas.md", count: 3 },
];

describe("BacklinksPopover", () => {
	it("renders list of backlink files with display names", () => {
		render(BacklinksPopover, {
			references: refs,
			onselect: vi.fn(),
			onclose: vi.fn(),
		});
		expect(screen.getByText("project kickoff")).toBeTruthy();
		expect(screen.getByText("meeting notes")).toBeTruthy();
		expect(screen.getByText("ideas")).toBeTruthy();
	});

	it("shows correct pluralized header", () => {
		render(BacklinksPopover, {
			references: refs,
			onselect: vi.fn(),
			onclose: vi.fn(),
		});
		expect(screen.getByText("3 backlinks")).toBeTruthy();
	});

	it("shows singular header for 1 backlink", () => {
		render(BacklinksPopover, {
			references: [{ relPath: "solo.md", count: 1 }],
			onselect: vi.fn(),
			onclose: vi.fn(),
		});
		expect(screen.getByText("1 backlink")).toBeTruthy();
	});

	it("shows reference count when > 1", () => {
		render(BacklinksPopover, {
			references: refs,
			onselect: vi.fn(),
			onclose: vi.fn(),
		});
		expect(screen.getByText("2 references")).toBeTruthy();
		expect(screen.getByText("3 references")).toBeTruthy();
	});

	it("shows directory hint for nested files", () => {
		render(BacklinksPopover, {
			references: refs,
			onselect: vi.fn(),
			onclose: vi.fn(),
		});
		expect(screen.getByText("notes")).toBeTruthy();
	});

	it("arrow key navigation cycles through items", async () => {
		render(BacklinksPopover, {
			references: refs,
			onselect: vi.fn(),
			onclose: vi.fn(),
		});
		const popover = screen.getByRole("listbox");
		await fireEvent.keyDown(popover, { key: "ArrowDown" });
		const options = screen.getAllByRole("option");
		expect(options[1].getAttribute("aria-selected")).toBe("true");

		await fireEvent.keyDown(popover, { key: "ArrowDown" });
		expect(options[2].getAttribute("aria-selected")).toBe("true");

		// Wraps around
		await fireEvent.keyDown(popover, { key: "ArrowDown" });
		expect(options[0].getAttribute("aria-selected")).toBe("true");
	});

	it("ArrowUp wraps from first to last", async () => {
		render(BacklinksPopover, {
			references: refs,
			onselect: vi.fn(),
			onclose: vi.fn(),
		});
		const popover = screen.getByRole("listbox");
		await fireEvent.keyDown(popover, { key: "ArrowUp" });
		const options = screen.getAllByRole("option");
		expect(options[2].getAttribute("aria-selected")).toBe("true");
	});

	it("Enter selects the focused item", async () => {
		const onselect = vi.fn();
		render(BacklinksPopover, {
			references: refs,
			onselect,
			onclose: vi.fn(),
		});
		const popover = screen.getByRole("listbox");
		await fireEvent.keyDown(popover, { key: "ArrowDown" });
		await fireEvent.keyDown(popover, { key: "Enter" });
		expect(onselect).toHaveBeenCalledWith("notes/meeting-notes.md");
	});

	it("Escape calls onclose", async () => {
		const onclose = vi.fn();
		render(BacklinksPopover, {
			references: refs,
			onselect: vi.fn(),
			onclose,
		});
		const popover = screen.getByRole("listbox");
		await fireEvent.keyDown(popover, { key: "Escape" });
		expect(onclose).toHaveBeenCalled();
	});

	it("clicking an item calls onselect", async () => {
		const onselect = vi.fn();
		render(BacklinksPopover, {
			references: refs,
			onselect,
			onclose: vi.fn(),
		});
		await fireEvent.click(screen.getByText("ideas"));
		expect(onselect).toHaveBeenCalledWith("ideas.md");
	});

	it("shows empty state defensively", () => {
		render(BacklinksPopover, {
			references: [],
			onselect: vi.fn(),
			onclose: vi.fn(),
		});
		expect(screen.getByText("No backlinks")).toBeTruthy();
		expect(screen.getByText("0 backlinks")).toBeTruthy();
	});
});
