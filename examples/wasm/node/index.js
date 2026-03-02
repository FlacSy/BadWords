/**
 * BadWords WASM - Node.js example
 *
 * Build for Node.js first: make wasm-nodejs
 * Run: node examples/wasm/backend/index.js
 *
 * Note: Node.js build uses CommonJS (synchronous load, no init needed)
 */

const { ProfanityFilter } = require('../../../crates/badwords-wasm/pkg/badwords_wasm.js');

function main() {
  const filter = new ProfanityFilter();

  // Check for profanity
  console.log('Languages:', filter.getLanguages());
  console.log('');

  const tests = [
    'Hello, nice day!',
    'Some bad word here',
    'h3ll0 with numbers',
    'привет мир',
  ];

  for (const text of tests) {
    const isBad = filter.isBad(text);
    const censored = filter.censor(text, '*');
    console.log(`"${text}"`);
    console.log(`  isBad: ${isBad}, censored: "${censored}"`);
    console.log('');
  }

  // Add custom words
  filter.addWords(['custom_bad_word']);
  console.log('After adding "custom_bad_word":', filter.isBad('This has custom_bad_word'));
}

main();
