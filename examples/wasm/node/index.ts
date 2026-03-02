/**
 * BadWords WASM - Node.js TypeScript example
 *
 * Build: make wasm-nodejs
 * Run: npx tsx examples/wasm/node/index.ts
 */

// @ts-ignore - CommonJS wasm module
const { ProfanityFilter } = require('../../../rust/badwords-wasm/pkg/badwords_wasm.js');

function main(): void {
  const filter = new ProfanityFilter();

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

  filter.addWords(['custom_bad_word']);
  console.log('After adding "custom_bad_word":', filter.isBad('This has custom_bad_word'));
}

main();
