# Changelog

## Unreleased

### Add
- `cbig!` / `static_cbig!` (and the `cbig_embedded` / `static_cbig_embedded` building blocks) for
  creating [`dashu-cmplx`]'s `CBig` from a complex literal in algebraic `a+bi` form or a `re, im`
  pair. Each coefficient reuses the `fbig!` base-2 literal parser; `static_cbig!` builds the value
  via the new `CBig::from_repr_parts` const constructor (gated on Rust 1.64+, like the other static
  variants).

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
