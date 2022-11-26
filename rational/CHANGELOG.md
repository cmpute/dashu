# Changelog

## 0.3.1 (WIP)

- Impl `Sum` and `Product` traits for `RBig` and `Relaxed`.
- Impl `dashu_base::Inverse` trait for `RBig` and `Relaxed`.
- Add `cubic()` for `RBig` and `Relaxed`.
- Add support of random rational numbers generation through `Uniform01` and `UniformRBig`.
- Implement `rand::distributions::uniform::SampleUniform` for `RBig`.
- Implement `serde::{Serialize, Deserialize}` for `RBig` and `Relaxed`
- Fix the bug in number comparison.

## 0.3.0 (Initial release)

- Support basic arithmetic operations and numeric conversion.
- Support Diophatine approximations.
