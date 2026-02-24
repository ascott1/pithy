import { describe, it, expect } from "vitest";
import { normalizeForMatch, resolveWikilink, type FileEntry } from "./wikilink";

describe("normalizeForMatch", () => {
	it("lowercases input", () => {
		expect(normalizeForMatch("Hello World")).toBe("hello-world");
	});

	it("collapses spaces to single dash", () => {
		expect(normalizeForMatch("foo  bar")).toBe("foo-bar");
	});

	it("collapses dashes to single dash", () => {
		expect(normalizeForMatch("foo--bar")).toBe("foo-bar");
	});

	it("collapses underscores to single dash", () => {
		expect(normalizeForMatch("foo__bar")).toBe("foo-bar");
	});

	it("collapses mixed separators", () => {
		expect(normalizeForMatch("foo - _ bar")).toBe("foo-bar");
	});

	it("handles already normalized input", () => {
		expect(normalizeForMatch("project-kickoff")).toBe("project-kickoff");
	});
});

describe("resolveWikilink", () => {
	const files: FileEntry[] = [
		{ path: "project-kickoff.md", stem: "project kickoff" },
		{ path: "daily-notes.md", stem: "daily notes" },
		{ path: "sub/readme.md", stem: "readme" },
	];

	it("resolves exact match", () => {
		expect(resolveWikilink("project kickoff", files)).toBe(
			"project-kickoff.md",
		);
	});

	it("resolves case-insensitive", () => {
		expect(resolveWikilink("Project Kickoff", files)).toBe(
			"project-kickoff.md",
		);
	});

	it("resolves dashes matching spaces", () => {
		expect(resolveWikilink("project-kickoff", files)).toBe(
			"project-kickoff.md",
		);
	});

	it("resolves underscores matching spaces", () => {
		expect(resolveWikilink("daily_notes", files)).toBe("daily-notes.md");
	});

	it("returns null for no match", () => {
		expect(resolveWikilink("nonexistent", files)).toBeNull();
	});

	it("returns null for empty target", () => {
		expect(resolveWikilink("", files)).toBeNull();
	});

	it("returns null for whitespace-only target", () => {
		expect(resolveWikilink("   ", files)).toBeNull();
	});
});
