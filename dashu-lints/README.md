# dashu-lints

Custom [Dylint](https://github.com/trailofbits/dylint) lints enforcing dashu house conventions.

This crate is not a member of the main workspace: it builds against a pinned nightly toolchain
(see `rust-toolchain`) because lints link against rustc internals, exactly like Clippy. The main
dashu crates remain stable-Rust; nightly is needed only to build and run these lints.

Currently all lints are **warn-by-default** so the existing hits can be cleaned up incrementally;
they will be flipped to deny once the tree is clean.

## Lints

| Lint | Default | Description |
|---|---|---|
| `if_sign_mul` | warn | Flags `if sign == Sign::Negative { -v } else { v }` selections (including the `Sign::Positive` and `!=` mirrors), which can be written as `sign * v` |

## Running

One-time setup:

```sh
cargo install cargo-dylint dylint-link
```

Then, from the repository root (the library is registered in the root `Cargo.toml` under
`[workspace.metadata.dylint]`):

```sh
cargo +nightly-2026-04-16 dylint --all -- --all-targets
```

or scoped to one crate while iterating:

```sh
cargo +nightly-2026-04-16 dylint --all -- -p dashu-float --all-targets
```

The `+nightly-2026-04-16` on the command is what makes the *target* crates build under the pinned
nightly; the driver itself picks the same toolchain up from `rust-toolchain`. The pinned channel
and the `clippy_utils` revision in `Cargo.toml` must be bumped together (see
[malachite-lints](https://github.com/mhogrefe/malachite/tree/master/malachite-lints), which uses
the same scheme).

## Note on network-restricted machines

Building the `dylint_driver` executable, which `cargo-dylint` does on the fly, runs a build script
that clones rust-clippy from GitHub. On machines where that clone times out, redirect it to a
local mirror of the commit that matters (the one whose `rust-toolchain.toml` channel equals the
pinned nightly):

```sh
git clone --depth 1 --filter=blob:none --no-checkout https://github.com/rust-lang/rust-clippy /tmp/rust-clippy
git -C /tmp/rust-clippy fetch --depth 1 origin f6d310692116e9a527ce6d0b3526c965d9c5d7b9
git -C /tmp/rust-clippy checkout f6d310692116e9a527ce6d0b3526c965d9c5d7b9
git init ~/path/to/clippy-mini
echo "f6d310692116e9a527ce6d0b3526c965d9c5d7b9" | git -C /tmp/rust-clippy pack-objects --revs --stdout | git -C ~/path/to/clippy-mini unpack-objects -q
```

then rewrite the URL for the build script only (never commit this):

```sh
GIT_CONFIG_COUNT=1 \
GIT_CONFIG_KEY_0='url.file://$HOME/path/to/clippy-mini.insteadOf' \
GIT_CONFIG_VALUE_0='https://github.com/rust-lang/rust-clippy' \
cargo +nightly-2026-04-16 dylint --all -- --all-targets
```

Running on this repository currently reports a dozen hits in the `float` crate. Cleaning those up
goes together with adding `Mul<Sign>` for `Ball` and by-reference `IBig`/`FBig` impls, after which
the lint can be flipped to deny.

## Testing

Each lint has an example under `examples/` together with a `.stderr` snapshot:

```sh
cd dashu-lints && cargo test
```

## Using these lints in your own project

The crate is not published to crates.io (a compiler-plugin dylib pinned to a specific nightly has
no use for dependency resolution); point Dylint at this repository from your own workspace's
`Cargo.toml` instead:

```toml
[workspace.metadata.dylint]
libraries = [{ git = "https://github.com/cmpute/dashu", pattern = "dashu-lints" }]
```
