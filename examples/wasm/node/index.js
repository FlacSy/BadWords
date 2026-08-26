/**
 * BadWords WASM - Node.js example.
 *
 * Build first: make wasm-nodejs
 * Run:         node examples/wasm/node/index.js
 *
 * The Node.js build is CommonJS: it loads synchronously, with no init() call.
 */

const { ProfanityFilter } = require('../../../rust/badwords-wasm/pkg-node/badwords_wasm.js');

function main() {
  const filter = new ProfanityFilter();

  console.log('languages:', filter.loadedLanguages().join(', '));
  console.log('entries:  ', filter.wordCount());
  console.log('');

  const messages = [
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
  for (const match of filter.find('what a shitty, damn mess')) {
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

  // Evasion detection is opt-in. Options are a plain object.
  const strict = { collapseRepeats: true, leetspeak: true, splitOnPunctuation: true };
  for (const text of ['shiiit', 'you.shit']) {
    console.log(`  ${text}: default=${filter.isProfane(text)} strict=${filter.isProfane(text, strict)}`);
  }
}

main();
