# The `postgres-types` / `diesel` optional features in `dashu-float`
# pull `postgres-types-0.2.13`, `diesel-2.3.9`, `digest-0.11.3`, etc.
# via `postgres-protocol`. The latest versions of those all bump
# their MSRV above ours (some require edition = "2024" -> Rust 1.85,
# `diesel-2.3.9` requires 1.86). They aren't core surface, so the
# MSRV check drops them. Stable still exercises them via `--all-features`.
#
# Also removes `criterion` and `postgres` dev-deps, whose transitive
# dependencies are incompatible with older Cargo's dependency resolver.

import re
import shutil

# Strip criterion from dev-dependencies and postgres from float's dev-deps.
for manifest in ['base/Cargo.toml', 'integer/Cargo.toml', 'rational/Cargo.toml']:
    text = open(manifest).read()
    text = re.sub(r'\n*criterion = \{.*\n', '\n', text)
    open(manifest, 'w').write(text)

text = open('float/Cargo.toml').read()
text = re.sub(r'\n*criterion = \{.*\n', '\n', text)
text = re.sub(r'\n*postgres = \{.*\n', '\n', text)
open('float/Cargo.toml', 'w').write(text)

# Strip the postgres/diesel feature flags + optional deps from
# float's manifest, and the `[[test]] name = "postgres"` entry.
text = open('float/Cargo.toml').read()
for pat in [
    r'^diesel = \["diesel_v2"\]\n',
    r'^postgres-types = \["postgres-types_v02"\]\n',
    r'^postgres-types_v02 = \["dep:postgres-types_v02".*\n',
    r'^diesel_v1 = \{ optional = true,.*\n',
    r'^diesel_v2 = \{ optional = true,.*\n',
    r'^_bytes = \{ optional = true,.*\n',
    r'^postgres-types_v02 = \{ optional = true,.*\n',
]:
    text = re.sub(pat, '', text, flags=re.MULTILINE)
text = re.sub(
    r'\[\[test\]\]\nname = "postgres"\nrequired-features = .*\n',
    '', text)
open('float/Cargo.toml', 'w').write(text)

# Strip the `decimal-extras` meta feature.
text = open('Cargo.toml').read()
text = re.sub(
    r'\n# this feature enables.*\ndecimal-extras = .*\n',
    '', text)
open('Cargo.toml', 'w').write(text)

# Drop the postgres submodule (its cfg-gated children reference
# the features we just removed).
text = open('float/src/third_party/mod.rs').read()
text = re.sub(
    r'\n#\[cfg\(any\(\n[^)]+\bdiesel_v1[^)]+\)\)\]\nmod postgres;\n',
    '', text)
open('float/src/third_party/mod.rs', 'w').write(text)

shutil.rmtree('float/tests/postgres.rs', ignore_errors=True)
shutil.rmtree('float/src/third_party/postgres', ignore_errors=True)
