import { describe, it, expect } from "vitest";
import { fuzzyScore } from "./fuzzy";

describe("fuzzyScore", () => {
  it("returns 0 for empty needle", () => {
    expect(fuzzyScore("", "hello world")).toBe(0);
  });

  it("returns null when needle has no subsequence match", () => {
    expect(fuzzyScore("xyz", "hello world")).toBeNull();
  });

  it("matches exact prefix", () => {
    const score = fuzzyScore("hel", "hello world");
    expect(score).not.toBeNull();
    expect(score!).toBeGreaterThan(0);
  });

  it("matches subsequence across words", () => {
    const score = fuzzyScore("hw", "hello world");
    expect(score).not.toBeNull();
    expect(score!).toBeGreaterThan(0);
  });

  it("scores exact prefix higher than mid-word match", () => {
    const prefix = fuzzyScore("pro", "project kickoff")!;
    const mid = fuzzyScore("pro", "a-problem-note")!;
    expect(prefix).toBeGreaterThan(mid);
  });

  it("scores consecutive matches higher than scattered", () => {
    const consecutive = fuzzyScore("note", "my-notes")!;
    const scattered = fuzzyScore("note", "a-new-open-test-entry")!;
    expect(consecutive).toBeGreaterThan(scattered);
  });

  it("is case-insensitive", () => {
    const a = fuzzyScore("hello", "Hello World");
    const b = fuzzyScore("hello", "hello world");
    expect(a).toBe(b);
  });

  it("treats dashes and underscores as word separators", () => {
    const score = fuzzyScore("pk", "project-kickoff");
    expect(score).not.toBeNull();
    expect(score!).toBeGreaterThan(0);
  });
});
