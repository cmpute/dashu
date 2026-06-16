---
name: support-3rdparty-traits
description: This skill should be used when the user asks to "support a trait from another crate", "implement a third-party trait", "add an integration for crate X", "add num-traits / serde / rand / num-integer / num-order / zeroize support", "add a feature flag for a new dependency", "wire up crate X behind a feature", "expose type X for crate Y", or "support a new major version of rand / serde / num-traits / diesel / postgres-types" (e.g. "add rand_v09 alongside rand_v08", "support diesel v3"). It provides the SOP for adding an optional third-party dependency behind the project's versioned-feature-flag policy, placing the trait impls in the right module, forwarding the feature across crates and the meta crate, and keeping the no_std/MSRV/changelog invariants intact.
---

# Supporting Third-Party Traits in dashu

Standard operating procedure for two related changes:

- **Branch A** — implement traits from a *new* third-party crate (e.g. add `borsh`, `arbitrary`, `bytemuck` support, or a trait crate the repo doesn't depend on yet).
- **Branch B** — support a *new major version* of a crate that is already integrated (e.g. add `rand_v09` alongside `rand_v08`, or `diesel_v3` alongside `diesel_v2`).

Both share the same invariants; Branch B reuses Branch A's steps plus a versioning layer.

## Repository context (load-bearing facts)

Read these before touching anything — they are the conventions every existing integration follows.

- **Impls live in `<crate>/src/third_party/`**, one file per external crate. `<crate>/src/third_party/mod.rs` gates each submodule with `#[cfg(feature = "...")]`. `<crate>/src/lib.rs` declares `mod third_party;` (private) then `pub use third_party::*;` to expose any public items.
- **Two visibility flavors in `mod.rs`:**
  - `pub mod rand;` — when the file defines *new public types* (e.g. `UniformBits`, `UniformUBig`) that users import from `crate::rand`.
  - `mod num_traits;` / `mod serde;` — when the file only `impl`s an external trait for a local type; the impl is globally reachable once compiled, so nothing public needs re-exporting.
- **"Stable" vs "unstable" dependencies** (this split is explicit in every `Cargo.toml`, marked `# stable dependencies` / `# unstable dependencies`):
  - *Stable* = the crate's API is essentially frozen across versions (serde, zeroize, num-order). One feature, named after the crate. Example: `serde = ["dep:serde", "dashu-int/serde"]`.
  - *Unstable* = breaking changes between major versions (rand, num-traits, num-integer, diesel, postgres-types). Always a **versioned** feature/dep with a `_vXX` suffix, plus an **unversioned alias** feature pointing at the current default.
- **Versioned dependency = renamed optional dep.** The dep *key* carries the suffix and `package =` resolves it to the real crate:
  ```toml
  rand_v08 = { optional = true, version = "0.8.3", package = "rand", default-features = false }
  ```
  In Rust code you import by the suffixed key: `use rand_v08::{Rng, ...};`. Inside a file you may alias it back to the natural name for readability: `use num_traits_v02 as num_traits;`.
- **Versioned feature enables its own dep AND forwards downstream.** Because float/rational re-export int's types, their feature must also turn on the matching feature in the lower crate:
  ```toml
  num-traits_v02 = ["dep:num-traits_v02", "dashu-int/num-traits_v02"]
  ```
- **Unversioned alias** points at the latest version: `rand = ["rand_v08"]`, `num-traits = ["num-traits_v02"]`. Users write `features = ["rand"]`; the alias decides which real version that means.
- **Multiple major versions coexist as peers** (see diesel): `diesel_v1` and `diesel_v2` are both real, each with its own versioned dep + feature + module file (`third_party/postgres/diesel_v1.rs`, `diesel_v2.rs`). Shared conversion logic lives in `third_party/postgres/mod.rs`; each version file only wires that logic to its own diesel traits.
- **Private helper deps** use a leading-underscore key and are never given their own user-facing feature. Enabled as a side effect: `num-order = ["dep:num-order", "dep:_num-modular"]` (rational).
- **no_std:** every optional dep is declared `default-features = false` so it cannot drag `std` into the default (no_std) build. If the dep itself needs std, the feature must pull in `std` (e.g. `postgres-types_v02 = ["dep:postgres-types_v02", "dep:_bytes", "std"]`).
- **Meta crate `dashu` (root `Cargo.toml`) forwards every feature** to all sub-crates that have it — both the versioned form and the unversioned alias: `rand_v08 = ["dashu-int/rand_v08", "dashu-float/rand_v08", "dashu-ratio/rand_v08"]` and `rand = ["dashu-int/rand", ...]`.
- **MSRV is 1.68.** A new dep must build on 1.68; if it doesn't, it must be stripped from MSRV builds via `.github/workflows/drop_incompatible_deps_for_msrv.py` (see pre-publish-check skill, Step 6).
- **Two `Cargo.toml` styles coexist in the repo:** `integer` uses the *legacy* style where `optional = true` deps auto-create an implicit feature (so `serde`/`zeroize`/`num-order` aren't listed in `[features]`). `float`/`rational` use the *modern* explicit style with `dep:` syntax. **Prefer the `dep:` (modern) style** for any new work; match the surrounding crate's existing style when editing an existing feature.

## Decision: which branch?

- The crate is **not yet** in any `Cargo.toml` → **Branch A**.
- The crate **is** already integrated and you're adding a second major version (or replacing the supported one) → **Branch B**.

---

## Branch A — Support a trait from a new third-party crate

### Step A1 — Classify the dependency: stable or unstable?

If the upstream crate has a history of breaking changes between major versions (rand, num-traits, num-integer) or is pre-1.0, treat it as **unstable** → you need the versioned suffix machinery. If its API is effectively frozen at 1.x (serde, zeroize, num-order) → **stable** → a single unversioned feature suffices. When in doubt, choose unstable; the cost is one extra alias line and you avoid a painful migration later.

### Step A2 — Declare the optional dependency in the target crate's `Cargo.toml`

Under the matching `# stable dependencies` or `# unstable dependencies` comment. Always `default-features = false`.

Stable:
```toml
borsh = { optional = true, version = "1.x", default-features = false }
```
Unstable (note the `_vXX` key + `package =`):
```toml
arbitrary_v1 = { optional = true, version = "1.x", package = "arbitrary", default-features = false }
```

### Step A3 — Define the feature flag(s)

Stable (modern `dep:` style):
```toml
borsh = ["dep:borsh"]
```
Unstable — two entries: the versioned feature (with cross-crate forwarding if a lower crate also exposes the type) plus the unversioned alias:
```toml
arbitrary_v1 = ["dep:arbitrary_v1"]
arbitrary = ["arbitrary_v1"]
```
If the trait must also be enabled on a crate this one depends on (because it re-exports that crate's types), add the downstream feature to the versioned feature list — see load-bearing fact above.

### Step A4 — Create the implementation module

Add `<crate>/src/third_party/<crate_name>.rs`. Import the external crate by its dep key (alias back to the natural name if it reads better). `impl` the external trait(s) for the local types (`UBig`/`IBig`/`FBig`/`RBig`). Follow existing files as a template: `integer/src/third_party/num_traits.rs` (pure trait impls), `integer/src/third_party/rand.rs` (defines new public types + a module-level `# Examples` doc), `float/src/third_party/postgres/` (multi-version, shared logic).

If the file defines new public items the user will name, declare it `pub mod` in `mod.rs`; otherwise plain `mod`.

### Step A5 — Wire it into `third_party/mod.rs`

```rust
#[cfg(feature = "borsh")]
mod borsh;
// or, if it exposes public types:
#[cfg(feature = "arbitrary_v1")]
pub mod arbitrary;
```
No change to `lib.rs` is needed beyond the existing `pub use third_party::*;` — that line already re-exports anything `pub` from these modules.

### Step A6 — Forward the feature across crates (if applicable)

If the same trait makes sense on types from more than one sub-crate, repeat Steps A2–A5 in each crate, and make each higher crate's versioned feature turn on the lower crate's matching feature (load-bearing fact). Keep the feature *name* identical across crates (`arbitrary_v1` everywhere).

### Step A7 — Forward the feature in the meta crate

In the root `Cargo.toml` `[features]`, add both forms (versioned + unversioned alias), each forwarding to every sub-crate that implements it:
```toml
arbitrary_v1 = ["dashu-int/arbitrary_v1", "dashu-float/arbitrary_v1", "dashu-ratio/arbitrary_v1"]
arbitrary = ["dashu-int/arbitrary", "dashu-float/arbitrary", "dashu-ratio/arbitrary"]
```
List only the sub-crates that actually have the feature — do not forward to a crate that doesn't implement it (that's a build error).

### Step A8 — Tests, doc examples, dev-dependencies

- Add a `[[test]]` with `required-features = ["<feature>"]` (see the `random` / `serde` tests in each `Cargo.toml`), or put unit tests in a `#[cfg(test)] mod tests` at the bottom of the new module file.
- Re-declare the dep (un-suffixed or suffixed to match) under `[dev-dependencies]` *without* `optional`, so doctests/examples resolve.
- Any public item documented at module level (like `rand.rs`) needs a `# Examples` block; in examples `use` the dep by its dep key (`use rand_v08::{...}`).

### Step A9 — MSRV check

Confirm the new dep's `rust-version` (or last-known-compatible release) is ≤ 1.68. If not, add it to `drop_incompatible_deps_for_msrv.py` so MSRV CI builds strip it.

### Step A10 — Changelog

Add an entry under `### Add` in the `## Unreleased` section of *every* crate whose `Cargo.toml` you changed (target crate, any crate that got cross-crate forwarding, and note the meta crate has no changelog). New optional deps are additive → patch bump.

---

## Branch B — Support a new major version of an existing crate

Example below uses adding `rand_v09` alongside `rand_v08`; substitute names as needed.

### Step B1 — Add the new versioned dependency

Keep the old one; add the new one beside it under `# unstable dependencies`:
```toml
rand_v08 = { optional = true, version = "0.8.3", package = "rand", default-features = false }
rand_v09 = { optional = true, version = "0.9.0", package = "rand", default-features = false }
```
Both keys resolve to the real `rand` crate via `package =`. They coexist fine because the keys differ.

### Step B2 — Add the versioned feature

In every crate that supports it, mirroring the old version's feature line and its cross-crate forwarding:
```toml
rand_v09 = ["dep:rand_v09", "dashu-int/rand_v09"]   # in float/rational
rand_v09 = ["dep:rand_v09"]                          # in integer
```

### Step B3 — Decide the unversioned alias target

The alias `rand = [...]` selects the default version users get. Two options:

- **Keep the alias on the old version** (`rand = ["rand_v08"]`) — purely additive, no behavior change for existing `features = ["rand"]` users. **Recommended default.**
- **Move the alias to the new version** (`rand = ["rand_v09"]`) — existing users silently upgrade. This is a *breaking* change in effect; only do it deliberately, call it out in the changelog under `### Change`, and require at least a minor bump (pre-1.0 rule).

If you move the alias, the old versioned feature still exists for users who need to pin it explicitly.

### Step B4 — Create/adapt the implementation module

Add `<crate>/src/third_party/rand_v09.rs` (or whatever splits cleanly). Often you can share logic with the old-version file by extracting the version-independent core into a helper (the `postgres/mod.rs` `Numeric` struct + conversion fns are the model: shared core, version-specific trait wiring in `diesel_v1.rs`/`diesel_v2.rs`). Gate the new module in `mod.rs`:
```rust
#[cfg(feature = "rand_v08")]
pub mod rand;      // existing
#[cfg(feature = "rand_v09")]
pub mod rand_v09;  // new
```
Port every trait impl from the old file, adjusting for API differences in the new major version.

### Step B5 — Cross-crate + meta forwarding

Repeat Step B2 in every implementing crate, and add `rand_v09 = [...]` (and, if you moved the alias, the updated `rand = [...]`) to the root `Cargo.toml` forwarding to all sub-crates that have it.

### Step B6 — Tests, doc examples, dev-dependencies

Add a dev-dependency for the new version, and either a new `[[test]]` gated on `rand_v09` or extend the existing one to cover both versions. Mirror the old version's doctest coverage using the new dep key.

### Step B7 — Changelog

Under `### Add` in each affected crate's `## Unreleased`: "Add support for rand 0.9 (behind the `rand_v09` feature)." If you moved the unversioned alias (Step B3 option 2), *also* add a `### Change` entry and treat the release as breaking.

---

## Verification

After either branch, confirm the feature compiles in isolation and that the no_std path still holds:

```sh
# feature on, 64-bit Word
cargo check -p <crate> --features <feature>
# feature on, 32-bit Word (NTT and some paths are 32-bit-specific)
RUSTFLAGS='--cfg force_bits="32"' cargo check -p <crate> --features <feature>
# no_std still builds (the new dep must not have pulled in std)
cargo build -p <crate> --no-default-features --features <feature>
# clippy clean with the feature on
cargo clippy -p <crate> --features <feature> -- -D warnings
# doctests/examples for the new module
cargo test -p <crate> --features <feature> --doc
# meta crate forwards correctly
cargo check -p dashu --features <feature>
```

If the feature exists in multiple sub-crates, run the `--features` check on each and on `-p dashu`.

## Common failure modes

- **Forgot the `package =` rename on a versioned dep** — `rand_v09` without `package = "rand"` makes cargo look for a (nonexistent) crate literally named `rand_v09`. Every versioned dep needs `package = "<real crate name>"`.
- **Forgot cross-crate forwarding** — float/rational's `num-traits_v02` feature must list `dashu-int/num-traits_v02`; without it, enabling the feature on the higher crate doesn't give the lower crate's types the impl.
- **Forwarded a feature to a crate that doesn't have it** — `dashu-base` has no `rand` feature; listing it in the meta crate's `rand` forwarding is a build error. Only forward to crates that actually define the feature.
- **Dep without `default-features = false`** — silently breaks the no_std build (serde/zeroize pull std by default). Every optional dep in this repo sets it.
- **`#[cfg]` feature name ≠ Cargo feature name** — `mod.rs` gates on the *feature* name (`rand_v08`), not the dep key's package. They usually match the key, but if you renamed via `package =`, the cfg still uses the key/feature name, not `rand`.
- **Added an unstable dep without the unversioned alias** — users expect `features = ["rand"]` to work; a versioned-only feature breaks that contract.
- **New dep exceeds MSRV** — builds fine on stable CI, fails the `1.68` matrix job. Must be stripped in `drop_incompatible_deps_for_msrv.py`.
- **Visibility mismatch** — declared `mod foo` in `mod.rs` but the module defines public types the user needs (`crate::foo::UniformBits`). If users name items from the module, it must be `pub mod`.
- **Missed a changelog** — every `Cargo.toml` you edited (target crate + forwarded crates) needs an `## Unreleased` entry; the meta crate has none.
