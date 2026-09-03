# BadWords WASM examples

WebAssembly builds for the browser and for Node.js.

## Build

From the repository root:

```bash
make wasm         # browser: ES modules, async init, output in pkg-web/
make wasm-nodejs  # Node.js: CommonJS, synchronous, output in pkg-node/
```

The two targets write to separate directories, so one does not overwrite the
other. Requires `cargo install wasm-pack`.

## Browser

```bash
make wasm
npx serve . -p 3000          # from the repository root
# or: cd examples/wasm/browser && npm start
```

Open <http://localhost:3000/examples/wasm/browser/>.

## Node.js

```bash
make wasm-nodejs
node examples/wasm/node/index.js
```

## Node.js with TypeScript

```bash
make wasm-nodejs
cd examples/wasm/node && npm install
npx tsx index.ts        # run
npx tsc --noEmit        # typecheck against the generated declarations
```

`make wasm-typecheck` does the install and the typecheck in one step. The types
come from the `.d.ts` wasm-pack generates, so a renamed method in the Rust layer
becomes a compile error here rather than a runtime surprise.

## API

```javascript
const filter = new ProfanityFilter();

filter.isProfane(text);              // boolean
filter.censor(text, '*');            // string, punctuation preserved
filter.find(text);                   // Match[] with spans, language and kind

filter.addWords(['spam_link']);      // project-specific words
filter.addWordsFromText(wordList);   // a whole word list in one call
filter.addWhitelist(['assessment']); // never flag these

filter.loadedLanguages();            // ['en', 'ru'] - what is compiled in
filter.availableLanguages();
```

Evasion detection is opt-in, because each detector trades false negatives for
false positives. Options are a plain object and can be reused:

```javascript
const strict = { splitOnPunctuation: true, collapseRepeats: true, leetspeak: true };
filter.isProfane('shiiit', strict);   // true
filter.isProfane('shiiit');           // false
filter.setOptions(strict);            // or set them once for every call
```

Unknown option names are rejected rather than ignored, so a typo is an error.

## Other languages

English and Russian are compiled in. The rest come from
[`@badwords/languages`](../../js/languages):

```javascript
import de from '@badwords/languages/de';
filter.addWords(de);
```

`filterText`, `isBad` and `getLanguages` still work but are deprecated; see the
[changelog](../../CHANGELOG.md).
