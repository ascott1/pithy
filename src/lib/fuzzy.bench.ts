import { bench, describe } from "vitest";
import { fuzzyScore } from "./fuzzy";

// Generate 10k synthetic filename stems
const WORDS = [
  "project", "meeting", "notes", "daily", "ideas", "research",
  "design", "review", "planning", "kickoff", "summary", "draft",
  "final", "archive", "backup", "todo", "journal", "thoughts",
  "brainstorm", "outline", "proposal", "report", "analysis",
  "feedback", "retrospective", "standup", "sprint", "roadmap",
];

function generateStems(count: number): string[] {
  const stems: string[] = [];
  for (let i = 0; i < count; i++) {
    const w1 = WORDS[i % WORDS.length];
    const w2 = WORDS[(i * 7 + 3) % WORDS.length];
    const w3 = WORDS[(i * 13 + 5) % WORDS.length];
    stems.push(`${w1} ${w2} ${w3} ${i}`);
  }
  return stems;
}

const stems10k = generateStems(10_000);

describe("fuzzyScore benchmarks", () => {
  // Target: < 50ms for 10k stems
  bench("fuzzy search 10k stems (3-char query)", () => {
    for (const stem of stems10k) {
      fuzzyScore("pro", stem);
    }
  });

  bench("fuzzy search 10k stems (full word query)", () => {
    for (const stem of stems10k) {
      fuzzyScore("meeting", stem);
    }
  });

  bench("fuzzy search 10k stems (multi-word query)", () => {
    for (const stem of stems10k) {
      fuzzyScore("project notes", stem);
    }
  });

  bench("fuzzy search 10k stems (1-char query)", () => {
    for (const stem of stems10k) {
      fuzzyScore("r", stem);
    }
  });

  bench("fuzzy search 10k stems (no match query)", () => {
    for (const stem of stems10k) {
      fuzzyScore("zzz", stem);
    }
  });
});
