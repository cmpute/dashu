# Changelog

## Unreleased

### Add

- Add `is_int` to `RBig`
- Implement `num-order::NumOrd` between `RBig`/`Relaxed` and `UBig`/`IBig`/`FBig`.
- Implement `num-order::NumHash` for `RBig` and `Relaxed`

### Change

- Now feature `num-traits` and `rand` are not enabled by default, feature `num-order` is enabled instead.
- Fix the bug in `is_one` of `RBig` and `Relaxed`.
- `RBig::square` and `Relaxed::square` are renamed to `sqr`

### Remove

- `PartialOrd` is no longer implemented between `RBig` and `Relaxed`. Please use `num-order::NumOrd` instead.

## 0.3.2

Fix the Bug in multiplication between `RBig` and `IBig`.

## 0.3.1

- Implement `Sum` and `Product` traits for `RBig` and `Relaxed`.
- Implement `Rem` trait for `RBig` and `Relaxed`.
- Implement `dashu_base::{Abs, Inverse, DivEuclid, RemEuclid, DivRemEuclid, EstimatedLog2}` traits for `RBig` and `Relaxed`.
- Implement `rand::distributions::uniform::SampleUniform` for `RBig`.
- Implement `serde::{Serialize, Deserialize}` for `RBig` and `Relaxed`
- Implement `num_traits::{Zero, One, Num, Signed, FromPrimitive, ToPrimitive, Pow, Euclid}` for `RBig` and `Relaxed`
- Add `cubic()`, `pow()` for `RBig` and `Relaxed`.
- Add `round()` for `RBig` and `Relaxed`.
- Add support of random rational numbers generation through `Uniform01` and `UniformRBig`.
- Add `rand_v08` and `num-traits_v02` feature flags to prevent breaking changes due to dependency updates in future 
- Fix the bug in number comparison.
- Re-export operation traits through the `ops` module.

## 0.3.0 (Initial release)

- Support basic arithmetic operations and numeric conversion.
- Support Diophatine approximations.
