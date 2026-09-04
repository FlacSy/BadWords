#!/bin/sh
# Publish one workspace crate to crates.io, unless that version is already there.
#
# Publishing is not atomic across crates: badwords-core can succeed and
# badwords-wasm fail, and re-running then dies on "crate version already
# exists" before it reaches the crate that still needs publishing. Checking
# the index first makes the job resumable, which is what a release needs when
# something goes wrong halfway.
set -eu

name=$1
version=$(sed -n '/\[workspace.package\]/,/^\[/p' Cargo.toml | grep -m1 '^version' | cut -d'"' -f2)

# Sparse index layout: names of four characters or more live under two
# two-character directories.
prefix=$(printf '%s' "$name" | cut -c1-2)/$(printf '%s' "$name" | cut -c3-4)
url="https://index.crates.io/$prefix/$name"

if curl -sf "$url" | grep -q "\"vers\":\"$version\""; then
    echo "$name $version is already on crates.io, skipping"
    exit 0
fi

echo "publishing $name $version"
cargo publish -p "$name"
