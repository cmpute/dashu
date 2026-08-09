# Changelog

## Unreleased

## 0.6.0-rc.4

### Change
- **Removed the `rustversion` dependency** — the `static_*` macros are no longer gated at Rust
  1.64 (`#[rustversion::since(1.64)]`); the MSRV is 1.68.
- **(breaking)** `cbig!` / `static_cbig!` coefficients are now **decimal by default**, matching
  `ubig!` / `ibig!`: use the `0x` / `0b` / `0o` prefixes for hexadecimal / binary / octal.
  Previously coefficients were parsed as binary floats, so `cbig!(3+4i)` was rejected and
  `cbig!(11+100i)` meant `3+4i`; now `cbig!(3+4i)` means `3+4i` and binary literals must be
  written with an explicit `0b` prefix (e.g. `cbig!(0b11 + 0b100i)`). Decimal and octal
  coefficients are converted to base 2 (exact for integers).

## 0.6.0-rc.3

Version aligned with the coordinated dashu 0.6.0-rc.3 release; no functional changes
(skipped 0.6.0-rc.2).

## 0.6.0-rc.1

### Change
- The literal macros now surface the new `ParseError::InvalidSyntax` (from `dashu-base`) with a
  dedicated panic message; the `InvalidDigit` panic message no longer says "or syntax".

## 0.5.1

### Add
- `cbig!` now works in `const` position when both coefficients fit in a `DoubleWord`: it expands to
  `CBig::from_parts_const` (built on `Repr::new_const`). Larger coefficients still fall back to the
  runtime `CBig::from_parts` path — use `static_cbig!` for const values with large coefficients.

### Doc
- Mention the `cbig!` / `static_cbig!` macros in the crate-level docs and the crate README, and
  note the `dashu-int` / `dashu-float` / `dashu-cmplx` dependency requirement.

## 0.5.0

### Add
- `cbig!` / `static_cbig!` (and the `cbig_embedded` / `static_cbig_embedded` building blocks) for
  creating [`dashu-cmplx`]'s `CBig` from a complex literal in algebraic `a+bi` form or a `re, im`
  pair. Each coefficient reuses the `fbig!` base-2 literal parser; `static_cbig!` builds the value
  via the new `CBig::from_repr_parts` const constructor (gated on Rust 1.64+, like the other static
  variants).

### Improve
- Enabled `#![deny(missing_docs)]` together with `clippy::dbg_macro`,
  `clippy::undocumented_unsafe_blocks`, and `clippy::let_underscore_must_use` as crate-level denies.

## 0.4.2

- Replace `paste` dependency with `pastey` ([#58](https://github.com/cmpute/dashu/pull/58)).
- Bump MSRV from 1.61 to 1.68.

## 0.4.1

- Add `static_ubig!` and `static_ibig!` macros to support static integer creation ([#38](https://github.com/cmpute/dashu/issues/38)).
- Add `static_fbig!` macro to support static float numbers creation.
- Add `static_rbig!` macro to support static rational numbers creation.

## 0.4.0

- Remove the `embedded` feature ([#18](https://github.com/cmpute/dashu/pull/18)).

## 0.3.1

- Fix the problem of `ibig` and `rbig` using incorrect crate names.

## 0.3.0

- Now only numbers that fit in `u32`s can be created in a const context. (Previously any numbers fit in `DoubleWord`s is permitted.)
- Add feature `embedded` to improve ergonomics when embedded in the `dashu` meta crate.

## 0.2.0 (Initial release)

- Support creating integers and floats from literals.
