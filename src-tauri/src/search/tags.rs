/// Extract `#tag` tokens from markdown text.
///
/// Returns deduplicated, lowercased, sorted tag names (without the `#` prefix).
/// Skips tags inside fenced code blocks, inline code, URLs, and markdown link
/// destinations.
pub fn extract_tags(text: &str) -> Vec<String> {
    let mut tags: Vec<String> = Vec::new();
    let mut in_fenced_block = false;
    let mut fence_char: char = '`';
    let mut fence_len: usize = 0;

    for line in text.lines() {
        // Check for fenced code block toggle (``` or ~~~)
        let trimmed = line.trim_start();
        if !in_fenced_block {
            if let Some(fc) = fence_start(trimmed) {
                in_fenced_block = true;
                fence_char = fc.0;
                fence_len = fc.1;
                continue;
            }
        } else {
            // Check for closing fence
            let count = trimmed.chars().take_while(|&c| c == fence_char).count();
            if count >= fence_len && trimmed[fence_char.len_utf8() * count..].trim().is_empty() {
                in_fenced_block = false;
            }
            continue;
        }

        extract_tags_from_line(line, &mut tags);
    }

    tags.sort();
    tags.dedup();
    tags
}

/// Returns (fence_char, fence_len) if the trimmed line starts a fenced code block.
fn fence_start(trimmed: &str) -> Option<(char, usize)> {
    let first = trimmed.chars().next()?;
    if first != '`' && first != '~' {
        return None;
    }
    let count = trimmed.chars().take_while(|&c| c == first).count();
    if count >= 3 {
        Some((first, count))
    } else {
        None
    }
}

fn extract_tags_from_line(line: &str, tags: &mut Vec<String>) {
    let chars: Vec<char> = line.chars().collect();
    let len = chars.len();
    let mut i = 0;

    while i < len {
        // Skip inline code spans
        if chars[i] == '`' {
            i += 1;
            while i < len && chars[i] != '`' {
                i += 1;
            }
            if i < len {
                i += 1; // skip closing `
            }
            continue;
        }

        // Skip URLs: http:// or https://
        if i + 7 < len && starts_with_at(&chars, i, "http://")
            || i + 8 < len && starts_with_at(&chars, i, "https://")
        {
            while i < len && !chars[i].is_whitespace() && chars[i] != ')' {
                i += 1;
            }
            continue;
        }

        // Skip markdown link destinations: ](...)
        if chars[i] == ']' && i + 1 < len && chars[i + 1] == '(' {
            i += 2; // skip ](
            while i < len && chars[i] != ')' {
                i += 1;
            }
            if i < len {
                i += 1; // skip )
            }
            continue;
        }

        // Check for tag
        if chars[i] == '#' {
            // Check boundary: must be at start or preceded by whitespace/open bracket
            let is_boundary = i == 0 || {
                let prev = chars[i - 1];
                prev.is_whitespace() || prev == '(' || prev == '[' || prev == '{'
            };

            if is_boundary {
                // Next char must be alphanumeric or _
                if i + 1 < len && (chars[i + 1].is_alphanumeric() || chars[i + 1] == '_') {
                    // Not a heading (# followed by space or another #)
                    let tag_start = i + 1;
                    let mut j = tag_start;
                    while j < len && is_tag_char(chars[j]) {
                        j += 1;
                    }
                    // Trim trailing hyphens/slashes from the tag
                    let mut end = j;
                    while end > tag_start && (chars[end - 1] == '-' || chars[end - 1] == '/') {
                        end -= 1;
                    }
                    if end > tag_start {
                        let tag: String = chars[tag_start..end].iter().collect();
                        tags.push(tag.to_lowercase());
                    }
                    i = j;
                    continue;
                }
            }
        }

        i += 1;
    }
}

fn starts_with_at(chars: &[char], start: usize, prefix: &str) -> bool {
    let prefix_chars: Vec<char> = prefix.chars().collect();
    if start + prefix_chars.len() > chars.len() {
        return false;
    }
    for (k, &pc) in prefix_chars.iter().enumerate() {
        if chars[start + k] != pc {
            return false;
        }
    }
    true
}

fn is_tag_char(c: char) -> bool {
    c.is_alphanumeric() || c == '-' || c == '_' || c == '/'
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_tags() {
        let input = "Hello #world and #rust are great";
        let tags = extract_tags(input);
        assert_eq!(tags, vec!["rust", "world"]);
    }

    #[test]
    fn tag_with_hyphens_and_underscores() {
        let input = "#my-tag and #my_tag";
        let tags = extract_tags(input);
        assert_eq!(tags, vec!["my-tag", "my_tag"]);
    }

    #[test]
    fn tag_with_slashes() {
        let input = "#project/backend/api";
        let tags = extract_tags(input);
        assert_eq!(tags, vec!["project/backend/api"]);
    }

    #[test]
    fn ignores_headings() {
        let input = "# Heading\n## Subheading\n### Another\nSome #real-tag here";
        let tags = extract_tags(input);
        assert_eq!(tags, vec!["real-tag"]);
    }

    #[test]
    fn ignores_fenced_code_block() {
        let input = "Before #visible\n```\n#hidden inside code\n```\nAfter #also-visible";
        let tags = extract_tags(input);
        assert_eq!(tags, vec!["also-visible", "visible"]);
    }

    #[test]
    fn ignores_tilde_fenced_code_block() {
        let input = "~~~\n#hidden\n~~~\n#shown";
        let tags = extract_tags(input);
        assert_eq!(tags, vec!["shown"]);
    }

    #[test]
    fn ignores_inline_code() {
        let input = "Use `#not-a-tag` but #real-tag is fine";
        let tags = extract_tags(input);
        assert_eq!(tags, vec!["real-tag"]);
    }

    #[test]
    fn ignores_url_fragments() {
        let input = "Visit https://example.com/page#section and #real";
        let tags = extract_tags(input);
        assert_eq!(tags, vec!["real"]);
    }

    #[test]
    fn ignores_markdown_link_url() {
        let input = "See [link](https://example.com#anchor) and #tag";
        let tags = extract_tags(input);
        assert_eq!(tags, vec!["tag"]);
    }

    #[test]
    fn deduplicates() {
        let input = "#rust #rust #rust";
        let tags = extract_tags(input);
        assert_eq!(tags, vec!["rust"]);
    }

    #[test]
    fn lowercases() {
        let input = "#Rust #PYTHON #GoLang";
        let tags = extract_tags(input);
        assert_eq!(tags, vec!["golang", "python", "rust"]);
    }

    #[test]
    fn empty_input() {
        let tags = extract_tags("");
        assert!(tags.is_empty());
    }

    #[test]
    fn hash_only_not_a_tag() {
        let input = "# ";
        let tags = extract_tags(input);
        assert!(tags.is_empty());

        let tags2 = extract_tags("#");
        assert!(tags2.is_empty());
    }

    #[test]
    fn tag_at_start_of_line() {
        let input = "#first\nsome text\n#second";
        let tags = extract_tags(input);
        assert_eq!(tags, vec!["first", "second"]);
    }

    #[test]
    fn tag_after_punctuation() {
        let input = "(#parens) [#brackets] {#braces}";
        let tags = extract_tags(input);
        assert_eq!(tags, vec!["braces", "brackets", "parens"]);
    }
}
