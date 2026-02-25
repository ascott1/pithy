import { describe, it, expect, afterEach } from "vitest";
import { render, screen, cleanup } from "@testing-library/svelte";
import InfoBar from "./InfoBar.svelte";

afterEach(cleanup);

describe("InfoBar", () => {
	it("renders word count and backlinks when both enabled", () => {
		render(InfoBar, {
			wordCount: 42,
			backlinkCount: 3,
			showBacklinks: true,
			showWordCount: true,
		});
		expect(screen.getByText("42 words")).toBeTruthy();
		expect(screen.getByText("3 backlinks")).toBeTruthy();
	});

	it("hides word count when showWordCount is false", () => {
		render(InfoBar, {
			wordCount: 10,
			backlinkCount: 1,
			showBacklinks: true,
			showWordCount: false,
		});
		expect(screen.queryByText(/\d+ words?$/)).toBeNull();
		expect(screen.getByText("1 backlink")).toBeTruthy();
	});

	it("hides backlinks when showBacklinks is false", () => {
		render(InfoBar, {
			wordCount: 10,
			backlinkCount: 5,
			showBacklinks: false,
			showWordCount: true,
		});
		expect(screen.queryByText(/\d+ backlinks?$/)).toBeNull();
		expect(screen.getByText("10 words")).toBeTruthy();
	});

	it("pluralizes correctly for singular values", () => {
		render(InfoBar, {
			wordCount: 1,
			backlinkCount: 1,
			showBacklinks: true,
			showWordCount: true,
		});
		expect(screen.getByText("1 word")).toBeTruthy();
		expect(screen.getByText("1 backlink")).toBeTruthy();
	});

	it("pluralizes correctly for zero", () => {
		render(InfoBar, {
			wordCount: 0,
			backlinkCount: 0,
			showBacklinks: true,
			showWordCount: true,
		});
		expect(screen.getByText("0 words")).toBeTruthy();
		expect(screen.getByText("0 backlinks")).toBeTruthy();
	});
});
