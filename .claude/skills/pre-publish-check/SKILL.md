---
name: pre-publish-check
description: Runs formatting/lint/test/semver checks before publishing. This skill should be used when the user asks to "publish dashu", "publish a sub-crate", "do pre-publish checks", "prepare a release", "cut a release", "bump the version", "check before publishing", or mentions publishing/releasing a new version of `dashu` or any sub-crate (`dashu-base`, `dashu-int`, `dashu-float`, `dashu-ratio`, `dashu-macros`).
---

# Pre-Publish Check for dashu

Run a consistent set of checks before publishing `dashu` (the meta crate) or any sub-crate to crates.io. Fail loudly on any regression that would break downstream users or violate the project's release policy.

## Repository context (load-bearing facts)

- The workspace contains 6 publishable crates: `dashu` (meta, at repo root), `base/`, `integer/`, `float/`, `rational/`, `macros/`. `python/` is **not** published and is excluded from workspace commands with `--exclude dashu-python`.
- **Major versions are always aligned across all crates** (`0.4.x` today). Minor and patch may differ — `dashu-float` is at `0.4.4` while the others are at `0.4.2`.
- Every crate's `Cargo.toml` pins the *path* and the *version range* of its dashu-internal dependencies. When sub-crate X bumps its version, every crate that depends on X (including the meta crate) must have its version requirement updated to `>=` the new version of X.
- The MSRV is **1.68**, declared both in `rust-version = "1.68"` in every `Cargo.toml` and in the README badge (`MSRV 1.68`). MSRV bumps are *breaking changes* and require a major version bump or an explicit policy exception (see CHANGELOG history for the 1.61 → 1.68 bump pattern).
- Every crate has its own `CHANGELOG.md` with an `## Unreleased` section. The meta `dashu` crate does **not** have a CHANGELOG — it follows the sub-crates.
- Feature-flag policy: third-party integrations are gated behind versioned feature flags (`rand_v08`, `num-traits_v02`, `diesel_v2`, `postgres-types_v02`) with an unversioned alias that points to the latest. Adding a new major-version dependency means adding a new versioned feature.
- Doc examples (`# Examples` blocks) on every public API are mandatory and are exercised by `cargo test --doc`.

## Workflow

### Step 1 — Identify target crate(s) and version(s)

Inspect the user's prompt for both pieces of information:

- **Target crate**: one of `dashu`, `dashu-base`, `dashu-int`, `dashu-float`, `dashu-ratio`, `dashu-macros`, or `all`.
- **Target version**: a SemVer string (e.g., `0.4.5`, `0.5.0`).

If either is missing, use `AskUserQuestion` to ask. Ask at most two questions in a single call:

1. "Which crate(s) are being published?" — options: each sub-crate, `dashu` (meta), `all sub-crates + meta`.
2. "What is the target version?" — leave this as free-text via the user's "Other" option, since the next version depends on what changed (major/minor/patch). Pre-populate sensible options based on the current Cargo.toml version: e.g., if current is `0.4.2`, offer `0.4.3` (patch), `0.5.0` (minor — note pre-1.0 minor acts like major in SemVer), `0.4.x` (let user specify).

**Pre-1.0 SemVer rule for this project:** since the major version is `0`, a *minor* bump (`0.4.x → 0.5.0`) is the breaking-change release (acts like a major bump). A *patch* bump (`0.4.2 → 0.4.3`) only allows backward-compatible additions and fixes — no removals, no signature changes, no MSRV bumps.

### Step 2 — Determine the required bump from the diff

Run `git log <last-release-tag>..HEAD --oneline` and read each affected crate's `CHANGELOG.md` `## Unreleased` section. Cross-reference the changelog against the SemVer rule:

| Change in `## Unreleased` | Required bump |
|---|---|
| `### Remove` of a public item, or `### Change` of a public signature | minor (since major is `0`) |
| `### Add` (new public items, additive) | patch OK |
| `### Fix` (bug fix preserving API) | patch OK |
| `### Improve` (perf/internal) | patch OK |
| MSRV bump (any magnitude) | minor + explicit `## Unreleased` note |

If the user-provided target version is *smaller* than what the diff requires, halt and report. If it is *larger* (e.g., user wants `0.5.0` but only patch-level changes are listed), warn and ask for confirmation — they may know about an additional breaking change not yet in the changelog.

### Step 3 — Run the normal CI-equivalent checks

Run these from the repo root. All must pass:

```sh
# Format check (no fix — surface the diff so the user decides)
cargo fmt --all -- --check

# Clippy on 64-bit Word (warnings are errors)
cargo clippy --all-features --all-targets --workspace --exclude dashu-python -- -D warnings

# Clippy on 32-bit Word (the NTT module has 32-bit-specific paths)
RUSTFLAGS='--cfg force_bits="32"' cargo clippy --all-features --all-targets --workspace --exclude dashu-python -- -D warnings

# Check + tests with all features (matches CI)
cargo check --all-features --tests
cargo test --workspace --exclude dashu-python --all-features --no-fail-fast

# no_std path — ensures default code paths do not pull in std
cargo test --no-default-features --features rand --workspace --exclude dashu-python
```

Report any failure verbatim. Do not auto-fix unless the user asks — these checks gate the release, so the user must own the fix.

### Step 4 — Verify CHANGELOG.md is updated for every affected crate

For each crate being published (and any crate whose version is bumped as a dependency side-effect):

1. Open `<crate>/CHANGELOG.md`.
2. Confirm `## Unreleased` exists and is non-empty.
3. Confirm every notable commit since the last release of that crate is reflected. Use `git log <last-tag-for-crate>..HEAD -- <crate>/` to find commits that touched the crate.
4. Confirm an MSRV change is mentioned in the changelog *if and only if* `rust-version` changed (see Step 6).

If a crate's `## Unreleased` is empty or missing entries, halt with a list of uncovered commits.

### Step 5 — Backward-compatibility verification

Use `cargo-semver-checks` to compare the working tree against the currently-published version on crates.io:

```sh
# Install if missing
cargo install cargo-semver-checks --locked

# Run against the specific crate(s) being released
cargo semver-checks check-release --package <crate-name> --verbose
```

Interpret the output against the project's pre-1.0 SemVer rule:

- If target is a **patch** bump: **zero** semver violations are allowed. Any `major` or `minor` finding from semver-checks is a hard failure.
- If target is a **minor** bump (breaking for this project): `major`/`minor` findings are allowed but each must correspond to an entry under `### Remove` or `### Change` in the changelog. Unlisted breakage is a hard failure.

If `cargo-semver-checks` is unavailable (e.g., MSRV-incompatible), fall back to a manual public-API diff:

```sh
# Install if missing
cargo install cargo-public-api --locked

# Compare current against the published version
cargo public-api --package <crate-name> diff <published-version>
```

Review the diff manually using the same rules.

### Step 6 — MSRV verification

For each crate being published:

1. Diff `rust-version` in `<crate>/Cargo.toml` against the version on `master` (or the last release tag). Use `git show <last-tag>:<crate>/Cargo.toml | grep rust-version`.
2. If `rust-version` changed:
   - Verify the changelog `## Unreleased` section explicitly mentions the MSRV bump (e.g., "Bump MSRV from 1.61 to 1.68" — match the wording used in past entries).
   - Verify the README badge (`MSRV 1.68` in the root `README.md`) matches the new value.
   - Verify the target version is at least a **minor** bump (since MSRV bumps are breaking for this project).
   - Verify `.github/workflows/tests.yml` still tests the new MSRV (the `rust: [stable, "1.85", "1.68"]` matrix and the `drop_incompatible_deps_for_msrv.py` invocation).
   - Verify `.github/workflows/drop_incompatible_deps_for_msrv.py` is still correct for the new MSRV — new deps may need to be stripped for MSRV builds.
3. If the user explicitly requested an MSRV change in their prompt but it's not in the changelog, halt and ask them to add the entry first.

### Step 7 — Cross-crate version sync

When publishing sub-crate X with new version V:

1. Every other crate Y whose `Cargo.toml` depends on X must be updated so that `version = "..."` accepts V. Use `grep -rn 'dashu-X = ' */Cargo.toml Cargo.toml` to find all references.
2. If Y is also being released in this cycle, update Y's `version = "..."` for X to the *exact* new version (e.g., `dashu-float = { version = "0.4.5", ... }`).
3. If Y is *not* being released, the version requirement still needs to be widened (e.g., `0.4.2` → `0.4.5`), but Y's own version stays put. **This is a real scenario** — for example, when `dashu-float` shipped `0.4.4` while siblings stayed at `0.4.2`.
4. The meta `dashu` crate at the repo root depends on every sub-crate. Its `Cargo.toml` must list version requirements that accept each new sub-crate version.
5. Run `cargo update -p <crate-name>` and `cargo build --workspace --exclude dashu-python` to confirm everything resolves.

### Step 8 — dashu-specific sanity checks

Verify project invariants documented in `AGENTS.md`:

1. **no_std**: default code paths must not use `std`. Confirm with `cargo build --no-default-features --workspace --exclude dashu-python` for each crate.
2. **Feature flags**: if the diff adds a new third-party dependency, confirm there's a versioned feature flag (e.g., `rand_v08`) plus an unversioned alias. Adding `rand` (unversioned) without a versioned variant is a violation.
3. **Doc examples**: every new public function or method must have a `# Examples` section. Run `cargo test --doc --workspace --exclude dashu-python --all-features` to confirm examples compile and pass.
4. **rustfmt config**: only `fn_call_width = 80` is set in `rustfmt.toml`. Long call sites that exceed this should be split — Step 3's `cargo fmt --check` already enforces this.
5. **Workspace integrity**: `dashu-python` is excluded from workspace tests and clippy. Never remove the `--exclude dashu-python` flag from any workspace command.

### Step 9 — Pre-publish packaging dry-run

For each crate being published, run `cargo publish --dry-run --allow-dirty --features <all-features-for-this-crate> -p <crate-name>` from the repo root. Resolve any errors before the actual publish.

### Step 10 — Report

Produce a checklist summary with one section per crate:

```
## dashu-float 0.4.5

- [x] fmt clean
- [x] clippy clean (64-bit and 32-bit)
- [x] tests pass (all-features and no-std)
- [x] CHANGELOG Unreleased matches commits since 0.4.4
- [x] semver-checks: 0 violations (patch-level changes only)
- [x] MSRV unchanged at 1.68
- [x] downstream deps updated: dashu-ratio/Cargo.toml, dashu-macros/Cargo.toml, dashu (meta)
- [x] cargo publish --dry-run succeeded
```

Halt the release and surface the failure if any box is unchecked. Do **not** run `cargo publish` for real — that is a human action. After the report, suggest the user run `cargo publish -p <crate>` themselves (or via `! cargo publish -p <crate>` in this session).

## Common failure modes

- **Forgot `--exclude dashu-python`**: `dashu-python` is in early development; workspace commands fail without the exclude.
- **32-bit clippy not run**: the NTT module has 32-bit-specific code paths. Always run clippy with both `--cfg force_bits="32"` (the default) and without.
- **`dashu-base` version pin drift**: `dashu-int` pins `dashu-base = "0.4.1"` (older than the others' `0.4.2`). This is intentional but easy to miss when bumping versions. Re-read every cross-crate pin in Step 7.
- **MSRV bump without CHANGELOG entry**: 1.61 → 1.68 was a coordinated bump that touched every crate's CHANGELOG. Any future MSRV change must do the same.
- **Semver-checks reports a violation but the changelog doesn't list it**: this is a hidden breaking change. Halt and ask the user whether to (a) add it to the changelog and bump minor, or (b) revert the change to keep the patch release.
