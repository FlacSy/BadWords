#!/usr/bin/env python3
"""Sharpen the .d.ts wasm-pack generates.

wasm-bindgen types an optional `Option<js_sys::Object>` parameter as
`options?: object | null`. Its `unchecked_param_type` attribute replaces the
type but also drops the `?`, so neither alone gives an optional, well-typed
parameter. This rewrites the parameter type and keeps the optionality.

The rewrite is verified by `npx tsc --noEmit -p examples/wasm/node`, which
fails if these signatures stop matching reality.

Run: make wasm / make wasm-nodejs (called automatically)
"""

from __future__ import annotations

import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
PACKAGES = [
    ROOT / "rust" / "badwords-wasm" / "pkg-web",
    ROOT / "rust" / "badwords-wasm" / "pkg-node",
]
REPLACEMENTS = [
    ("options?: object | null", "options?: MatchOptions"),
]


def refine(package: Path) -> bool:
    """Rewrite one package's declarations. True if it was changed."""
    declarations = package / "badwords_wasm.d.ts"
    if not declarations.exists():
        return False

    text = declarations.read_text(encoding="utf-8")
    original = text
    for old, new in REPLACEMENTS:
        text = text.replace(old, new)

    if text == original:
        return False
    declarations.write_text(text, encoding="utf-8")
    print(f"  refined {declarations.relative_to(ROOT)}")
    return True


def main() -> int:
    """Refine every built package."""
    built = [p for p in PACKAGES if (p / "badwords_wasm.d.ts").exists()]
    if not built:
        print("no wasm package built yet; run `make wasm` or `make wasm-nodejs`")
        return 1
    for package in built:
        refine(package)
    return 0


if __name__ == "__main__":
    sys.exit(main())
