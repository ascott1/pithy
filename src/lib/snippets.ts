/**
 * Strips markdown syntax from a Tantivy snippet while preserving
 * the <b> highlight tags that mark search matches.
 */
export function cleanSnippet(html: string): string {
  // Strip all HTML tags except <b> and </b>, then process text segments
  const sanitized = html.replace(/<(?!\/?b>)[^>]*>/gi, "");
  const result = sanitized.replace(
    /(<\/?b>)|([^<]+)/g,
    (_match, tag: string, text: string) => {
      if (tag) return tag;
      return stripMarkdown(text);
    },
  );
  return result.trim();
}

function stripMarkdown(text: string): string {
  return (
    text
      // headings
      .replace(/^#{1,6}\s+/gm, "")
      // bold/italic
      .replace(/\*\*\*(.+?)\*\*\*/g, "$1")
      .replace(/\*\*(.+?)\*\*/g, "$1")
      .replace(/__(.+?)__/g, "$1")
      .replace(/\*(.+?)\*/g, "$1")
      .replace(/_(.+?)_/g, "$1")
      // strikethrough
      .replace(/~~(.+?)~~/g, "$1")
      // inline code
      .replace(/`(.+?)`/g, "$1")
      // images (before links)
      .replace(/!\[([^\]]*)\]\([^)]+\)/g, "$1")
      // links
      .replace(/\[([^\]]+)\]\([^)]+\)/g, "$1")
      // blockquotes: one or more > markers (possibly nested) at start or after whitespace
      .replace(/(^|\s)(?:>\s*)+/g, "$1")
      // unordered lists
      .replace(/^[-*+]\s+/gm, "")
      // ordered lists
      .replace(/^\d+\.\s+/gm, "")
      // horizontal rules
      .replace(/^[-*_]{3,}$/gm, "")
      // collapse whitespace runs
      .replace(/\s{2,}/g, " ")
  );
}
