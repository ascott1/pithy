import { describe, it, expect } from "vitest";
import { cleanSnippet } from "./snippets";

describe("cleanSnippet", () => {
  it("preserves <b> highlight tags", () => {
    expect(cleanSnippet("hello <b>world</b>")).toBe("hello <b>world</b>");
  });

  it("strips heading markers", () => {
    expect(cleanSnippet("## Another Heading")).toBe("Another Heading");
  });

  it("strips link syntax, keeps text", () => {
    expect(cleanSnippet("[Link](https://example.com)")).toBe("Link");
  });

  it("strips bold markers", () => {
    expect(cleanSnippet("this is **bold** text")).toBe("this is bold text");
  });

  it("strips italic markers", () => {
    expect(cleanSnippet("this is *italic* text")).toBe("this is italic text");
  });

  it("strips blockquote markers", () => {
    expect(cleanSnippet("> This is a quote")).toBe("This is a quote");
  });

  it("strips inline code backticks", () => {
    expect(cleanSnippet("use `console.log` here")).toBe("use console.log here");
  });

  it("strips image syntax, keeps alt text", () => {
    expect(cleanSnippet("![alt text](image.png)")).toBe("alt text");
  });

  it("handles combined markdown with highlights", () => {
    const input = "## Heading <b>match</b> and [link](url) > <b>quote</b>";
    const result = cleanSnippet(input);
    expect(result).toBe("Heading <b>match</b> and link <b>quote</b>");
  });

  it("collapses whitespace from stripped syntax", () => {
    expect(cleanSnippet("> > nested  quote")).toBe("nested quote");
  });
});
