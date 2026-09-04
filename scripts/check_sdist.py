#!/usr/bin/env python3
"""Verify an sdist before it is offered to PyPI.

PyPI rejects a distribution whose metadata names a `License-File` the archive
does not actually contain, and it does so at upload time - after the wheels
for that version have already been accepted. `twine check` does not catch it:
it passed on the archive PyPI refused.

Run: python scripts/check_sdist.py dist/*.tar.gz
"""

from __future__ import annotations

import sys
import tarfile
from pathlib import Path


def check(path: Path) -> list[str]:
    """Return the problems found in one sdist."""
    problems: list[str] = []
    with tarfile.open(path, "r:gz") as archive:
        names = set(archive.getnames())
        roots = {name.split("/", 1)[0] for name in names}
        if len(roots) != 1:
            return [f"expected a single top-level directory, found {sorted(roots)}"]
        root = roots.pop()

        metadata_name = f"{root}/PKG-INFO"
        if metadata_name not in names:
            return [f"no {metadata_name}"]

        handle = archive.extractfile(metadata_name)
        if handle is None:
            return [f"cannot read {metadata_name}"]
        metadata = handle.read().decode("utf-8", "replace")

    for line in metadata.splitlines():
        if line.startswith("License-File:"):
            declared = line.split(":", 1)[1].strip()
            if f"{root}/{declared}" not in names:
                problems.append(f"License-File {declared} is declared but not in the archive")
        if not line.strip():
            break  # headers end at the first blank line; the body is the README

    return problems


def main() -> int:
    paths = [Path(argument) for argument in sys.argv[1:]]
    if not paths:
        print("usage: check_sdist.py <sdist.tar.gz> [...]", file=sys.stderr)
        return 2

    failed = False
    for path in paths:
        problems = check(path)
        if problems:
            failed = True
            print(f"{path.name}: FAILED")
            for problem in problems:
                print(f"  {problem}")
        else:
            print(f"{path.name}: ok")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())
