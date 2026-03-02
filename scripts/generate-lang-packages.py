#!/usr/bin/env python3
"""Generate @badwords/languages package from badwords/resource/words/*.txt"""

import json
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
WORDS_DIR = ROOT / "python" / "badwords" / "resource" / "words"
PKG_DIR = ROOT / "js" / "languages"


def main() -> None:
    PKG_DIR.mkdir(parents=True, exist_ok=True)
    lang_dir = PKG_DIR / "lang"
    lang_dir.mkdir(exist_ok=True)

    exports: dict[str, str] = {}
    for txt in sorted(WORDS_DIR.glob("*.txt")):
        lang = txt.stem
        lines = [
            line.strip()
            for line in txt.read_text(encoding="utf-8-sig").splitlines()
            if line.strip()
        ]
        out = lang_dir / f"{lang}.json"
        out.write_text(
            json.dumps(lines, ensure_ascii=False, indent=0), encoding="utf-8"
        )
        exports[f"./{lang}"] = f"./lang/{lang}.json"

    # index.js re-exports all for convenience
    index_lines = [
        "// Auto-generated. Use @badwords/languages/de etc. for tree-shaking.",
        "module.exports = {",
    ]
    for lang in sorted(exports.keys(), key=lambda x: x.replace("./", "")):
        l = lang.replace("./", "")
        index_lines.append(f'  {l}: require("./lang/{l}.json"),')
    index_lines.append("};")
    (PKG_DIR / "index.js").write_text("\n".join(index_lines), encoding="utf-8")

    exports["."] = "./index.js"
    exports["./package.json"] = "./package.json"

    pkg = {
        "name": "@badwords/languages",
        "version": "2.2.0",
        "description": "Optional language word lists for badwords-wasm",
        "license": "MIT",
        "repository": {"type": "git", "url": "https://github.com/FlacSy/badwords.git"},
        "keywords": ["badwords", "profanity", "filter", "languages"],
        "exports": exports,
        "files": ["lang", "index.js"],
    }
    (PKG_DIR / "package.json").write_text(
        json.dumps(pkg, indent=2, ensure_ascii=False), encoding="utf-8"
    )

    # README for the package
    readme = """# @badwords/languages

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

"""
    lang_keys = sorted(
        k.replace("./", "") for k in exports if k not in (".", "./package.json")
    )
    readme += ", ".join(lang_keys)
    readme += "\n"
    (PKG_DIR / "README.md").write_text(readme, encoding="utf-8")

    print(f"Generated {len(exports) - 2} language files in {PKG_DIR}")


if __name__ == "__main__":
    main()
