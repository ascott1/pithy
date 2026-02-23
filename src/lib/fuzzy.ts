/**
 * Fuzzy-match a needle against a haystack string.
 * Returns a numeric score (higher = better match) or null if no subsequence match.
 * Optimized for filename stems: word-start matches score higher, consecutive matches get bonuses.
 */
export function fuzzyScore(needle: string, haystack: string): number | null {
  const n = needle.toLowerCase();
  const h = haystack.toLowerCase();

  if (n.length === 0) return 0;

  let score = 0;
  let j = 0;
  let consecutive = 0;

  for (let i = 0; i < h.length && j < n.length; i++) {
    if (h[i] === n[j]) {
      score += 10;
      consecutive++;
      score += consecutive * 5;
      if (i === 0) {
        score += 20;
      } else if (isSeparator(h[i - 1])) {
        score += 15;
      }
      j++;
    } else {
      consecutive = 0;
      if (j > 0) score -= 1;
    }
  }

  return j === n.length ? score : null;
}

function isSeparator(ch: string): boolean {
  return ch === " " || ch === "-" || ch === "_" || ch === "/";
}
