# @badwords/languages

Optional language word lists for [badwords-wasm](https://www.npmjs.com/package/badwords-wasm).

## Usage

```bash
npm install badwords-wasm @badwords/languages
```

```javascript
import init, { ProfanityFilter } from 'badwords-wasm';
import de from '@badwords/languages/de';
import ua from '@badwords/languages/ua';

await init();
const filter = new ProfanityFilter();
filter.addWords(de);
filter.addWords(ua);

filter.isBad('some text');  // uses en+ru (built-in) + de + ua
```

## Available languages

br, cz, da, de, du, en, fi, fr, gr, hu, in, it, ja, ko, lt, no, pl, po, ro, ru, sp, sw, th, tu, ua
