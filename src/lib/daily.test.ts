import { describe, it, expect } from "vitest";
import { formatDailyName } from "./daily";

describe("formatDailyName", () => {
	const date = new Date(2026, 1, 24); // Feb 24, 2026

	it("formats default YYYY-MM-DD pattern", () => {
		expect(formatDailyName("YYYY-MM-DD", date)).toBe("2026-02-24");
	});

	it("formats DD-MM-YYYY pattern", () => {
		expect(formatDailyName("DD-MM-YYYY", date)).toBe("24-02-2026");
	});

	it("formats YYYY_MM_DD pattern", () => {
		expect(formatDailyName("YYYY_MM_DD", date)).toBe("2026_02_24");
	});

	it("pads single-digit month", () => {
		const jan = new Date(2026, 0, 5); // Jan 5
		expect(formatDailyName("YYYY-MM-DD", jan)).toBe("2026-01-05");
	});

	it("pads single-digit day", () => {
		const first = new Date(2026, 11, 1); // Dec 1
		expect(formatDailyName("YYYY-MM-DD", first)).toBe("2026-12-01");
	});

	it("handles format with no tokens", () => {
		expect(formatDailyName("daily-note", date)).toBe("daily-note");
	});

	it("handles format with prefix", () => {
		expect(formatDailyName("note-YYYY-MM-DD", date)).toBe("note-2026-02-24");
	});

	it("uses current date when none provided", () => {
		const result = formatDailyName("YYYY-MM-DD");
		expect(result).toMatch(/^\d{4}-\d{2}-\d{2}$/);
	});
});
