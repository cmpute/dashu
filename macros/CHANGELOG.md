# Changelog

## Unreleased

### Change
- `static_fbig!` / `static_dbig!` produce a value with unlimited precision for every literal, as documented. Literals whose significand fits in 32 bits used to carry the precision of the literal.
- A zero literal of `fbig!` / `dbig!` has the precision of its digits, like every other literal and like `from_str` (`dbig!(0.000)` has precision 4 and `dbig!(0)` precision 1; both used to have unlimited precision).

### Fix
- `cbig!` / `static_cbig!`: decimal and octal integer coefficients are converted to base 2 exactly (`cbig!(9)` was 8, `cbig!(99)` was 98). A coefficient with `n` digits now gets as many bits of precision as the largest `n`-digit integer needs, so `cbig!(3 + 4i)` has precision 4 (was 3).
- `fbig!` / `dbig!` / `cbig!` and their static variants keep the sign of a negative zero literal (`dbig!(-0.0)` was `+0`).
- `rbig!` / `static_rbig!` accept `base N` without a denominator (`rbig!(a base 16)` failed to compile).
- `rbig!` / `static_rbig!` reject two numbers that are not separated by `/` (`rbig!(1 - 2)` was -1/2) and repeated signs.
- `ibig!` / `static_ibig!` reject repeated or mixed signs (`ibig!(--5)` was -5).
- `cbig!` accepts an exponent with a sign in the imaginary coefficient (`cbig!(1 + 2e-3i)`, `cbig!(0x1p-2i)`), as `CBig::from_str` does.
- `ubig!` / `ibig!` (and the static variants) reject a repeated `base` marker (`ubig!(5 base base 36)`).

## 0.6.0

### Change
- **(breaking) `cbig!` / `static_cbig!` coefficients are now decimal by default**, matching `ubig!` /
  `ibig!` — use `0x`/`0b`/`0o` prefixes for other bases (previously coefficients were parsed as
  binary floats, so `cbig!(3+4i)` was rejected and `cbig!(11+100i)` meant `3+4i`).
- Removed the `rustversion` dependency (MSRV is 1.68).
- The literal macros now surface `ParseError::InvalidSyntax` with a dedicated panic message.

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
