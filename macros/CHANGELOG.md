# Changelog

## Unreleased

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
