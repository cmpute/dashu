# Changelog

## 0.3.1 (WIP)

- Fix the problem of `ibig` and `rbig` using incorrect crate names.

## 0.3.0

- Now only numbers that fit in `u32`s can be created in a const context. (Previously any numbers fit in `DoubleWord`s is permitted.)
- Add feature `embedded` to improve ergonomics when embedded in the `dashu` meta crate.

## 0.2.0 (Initial release)

- Support creating integers and floats from literals.
