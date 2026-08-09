#!/usr/bin/env bash
# Build the English (guide/) and Simplified-Chinese (guide-zh/) user guides.
#
# Both books are stamped with the dashu version and the git commit they were
# built from: this script generates a `version.md` into each book's `src/`
# (gitignored) which the corresponding `index.md` pulls in via
# `{{#include version.md}}`. Run this script instead of `mdbook build guide`
# / `mdbook build guide-zh` directly — without it the `{{#include}}` has no
# file to resolve and `mdbook build` fails.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

# Version: the `dashu` meta-crate version in the root Cargo.toml.
VERSION="$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -n1)"
if [[ -z "$VERSION" ]]; then
    echo "error: could not read the dashu version from the root Cargo.toml" >&2
    exit 1
fi

# Commit: tag + commits-since + short sha when tags are present, else just the
# short sha. `--dirty` flags uncommitted changes in the build tree.
COMMIT="$(git describe --tags --always --dirty 2>/dev/null || git rev-parse --short HEAD)"

cat > guide/src/version.md <<EOF
> Built from commit \`$COMMIT\` — dashu v$VERSION.
EOF
cat > guide-zh/src/version.md <<EOF
> 构建自提交 \`$COMMIT\` — dashu v$VERSION。
EOF

mdbook build guide
mdbook build guide-zh
