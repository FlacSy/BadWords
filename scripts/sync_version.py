#!/usr/bin/env python3
"""Propagate the workspace version to every package that is not a Cargo crate.

The version lives in [workspace.package] in the root Cargo.toml; the Rust
crates inherit it with `version.workspace = true`. This copies it to
pyproject.toml and the npm packages, and with --check verifies they agree
instead of writing.

Run: python scripts/sync_version.py [--check]
"""

from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
CARGO = ROOT / "Cargo.toml"
PYPROJECT = ROOT / "pyproject.toml"
NPM_PACKAGES = [ROOT / "js" / "languages" / "package.json"]


def workspace_version() -> str:
    """Read the version from [workspace.package]."""
    text = CARGO.read_text(encoding="utf-8")
    section = text.split("[workspace.package]", 1)
    if len(section) != 2:
        message = f"no [workspace.package] section in {CARGO}"
        raise RuntimeError(message)
    match = re.search(r'^\s*version\s*=\s*"([^"]+)"', section[1], re.MULTILINE)
    if match is None:
        message = f"no version in [workspace.package] of {CARGO}"
        raise RuntimeError(message)
    return match.group(1)


def sync_pyproject(version: str, *, check: bool) -> bool:
    """Set (or verify) the version in pyproject.toml."""
    text = PYPROJECT.read_text(encoding="utf-8")
    match = re.search(r'^version\s*=\s*"([^"]+)"', text, re.MULTILINE)
    if match is None:
        message = f"no version in {PYPROJECT}"
        raise RuntimeError(message)
    current = match.group(1)
    if current == version:
        return True
    if check:
        print(f"  pyproject.toml: {current} != {version}")
        return False
    PYPROJECT.write_text(
        text[: match.start(1)] + version + text[match.end(1) :],
        encoding="utf-8",
    )
    print(f"  pyproject.toml: {current} -> {version}")
    return True


def sync_npm(path: Path, version: str, *, check: bool) -> bool:
    """Set (or verify) the version in an npm package.json."""
    if not path.exists():
        return True
    data = json.loads(path.read_text(encoding="utf-8"))
    current = data.get("version")
    if current == version:
        return True
    if check:
        print(f"  {path.relative_to(ROOT)}: {current} != {version}")
        return False
    data["version"] = version
    path.write_text(json.dumps(data, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    print(f"  {path.relative_to(ROOT)}: {current} -> {version}")
    return True


def main() -> int:
    """Sync or check versions."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true", help="verify instead of writing")
    args = parser.parse_args()

    version = workspace_version()
    print(f"workspace version: {version}")

    ok = sync_pyproject(version, check=args.check)
    for package in NPM_PACKAGES:
        ok = sync_npm(package, version, check=args.check) and ok

    if args.check and not ok:
        print("versions are out of sync; run: python scripts/sync_version.py")
        return 1
    if not args.check:
        print("in sync")
    return 0


if __name__ == "__main__":
    sys.exit(main())
