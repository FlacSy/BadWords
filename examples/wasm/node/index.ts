/**
 * BadWords WASM - Node.js example, in TypeScript.
 *
 * Build first: make wasm-nodejs
 * Run:         npx tsx examples/wasm/node/index.ts
 * Typecheck:   npx tsc --noEmit -p examples/wasm/node
 *
 * The types come from the .d.ts wasm-pack generates, so a rename in the Rust
 * layer becomes a compile error here.
 */

import type { Match, MatchOptions } from '../../../rust/badwords-wasm/pkg-node/badwords_wasm';
import { ProfanityFilter } from '../../../rust/badwords-wasm/pkg-node/badwords_wasm';

function main(): void {
  const filter: ProfanityFilter = new ProfanityFilter();

  console.log('languages:', filter.loadedLanguages().join(', '));
  console.log('entries:  ', filter.wordCount());
  console.log('');

  const messages: string[] = [
    'Hello, nice day!',
    'sonofabitch',
    'hey shit, ok',
    'a clean sentence',
  ];

  for (const message of messages) {
    const status = filter.isProfane(message) ? 'BLOCKED' : 'OK';
    console.log(`[${status.padEnd(7)}] ${filter.censor(message, '*')}`);
  }

  // find() reports what matched, where, and in which language.
  const matches: Match[] = filter.find('what a shitty, damn mess');
  for (const match of matches) {
    console.log(
      `  ${match.matchedText} at ${match.start}..${match.end} ` +
        `(${match.word}, ${match.language ?? 'custom'}, ${match.kind})`,
    );
  }

  // Project-specific words, and words that must never be flagged.
  filter.addWords(['spam_link']);
  filter.addWhitelist(['assessment']);
  console.log('\ncustom word:', filter.isProfane('visit spam_link now'));
  console.log('whitelisted:', filter.isProfane('your assessment was wrong'));

  // Evasion detection is opt-in. Options are a plain object, so the same one
  // can be reused across calls.
  const strict: MatchOptions = {
    collapseRepeats: true,
    leetspeak: true,
    splitOnPunctuation: true,
  };
  for (const text of ['shiiit', 'you.shit']) {
    console.log(`  ${text}: default=${filter.isProfane(text)} strict=${filter.isProfane(text, strict)}`);
  }
}

main();
