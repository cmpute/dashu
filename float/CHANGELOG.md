# Changelog

## 0.3.1 (WIP)

- Implement `num_traits::{Zero, One, FromPrimitive, ToPrimitive, Num, Signed, Euclid, Pow}` for `FBig` (#19)
- Implement `rand::distributions::uniform::UniformSampler` for `FBig` through `crate::rand::UniformFBig`
- Implement `rand::distributions::{Open01, OpenClosed01, Standard}` for `FBig`
- Implement `dashu_base::Inverse` for `FBig`
- Implement `rand::distributions::uniform::SampleUniform` for `FBig`.
- Implement `serde::{Serialize, Deserialize}` for `FBig` and `Repr`
- Implement `Rem` trait for `FBig`
- Add support of random floating point numbers generation through `crate::rand::Uniform01` and `crate::rand::UniformFBig`.
- Add support for serialization from/to PostgreSQL arguments through `diesel::{deserialize::FromSql, serialize::ToSql}` and `postgres_types::{FromSql, ToSql}`.
- Add `from_str_native()` for `Repr`
- Add `to_f32()`, `to_f64()` for `Repr`, and these two methods supports all bases for both `Repr` and `FBig`.
- Add `to_int()` for `Repr`, which is equivalent to `FBig::trunc()`
- Add `TryFrom<FBig>` for `UBig` and `IBig`
- Add `round()` for `FBig`
- Add `rand_v08` and `num-traits_v02` feature flags to prevent breaking changes due to dependency updates in future 
- Re-export operation traits through the `ops` module.

## 0.3.0

### Add

- Conversion from FBig to `f32`/`f64` support subnormal values now.
- Add a `split_at_point()` function to `FBig`

## 0.2.1

- Implement `core::iter::{Sum, Product}` for `FBig`
- Implement `powf`, `sqrt` for `FBig`

## 0.2.0 (Initial release)

- Support basic arithmetic operations (`add`/`sub`/`mul`/`div`/`exp`/`ln`) and base conversion.

# Todo

## Roadmap to next version
- Support generating base math constants (E, Pi, SQRT2, etc.)
- Support operations with inf
- Implement Random generator
- Implement Serde serialization
- Implememt cbrt, nth_root
- Implement log
- Create operations benchmark
- Benchmark against crates: rug, twofloat, num-bigfloat, rust_decimal, bigdecimal, scientific
- Implement more formatting traits

## Roadmap to v1.0
- Determine if caches for constants (especially ln2, pi) should be stored in the context (using RC)

## Not in plan for v1.0
- Other math functions: sin/cos/tan/etc.
- Support more rounding modes
- Faster base conversion (references: [dragonbox](https://github.com/jk-jeon/dragonbox), [ryu](https://lib.rs/crates/ryu-js), [Articles by Lemire](https://arxiv.org/search/cs?searchtype=author&query=Lemire%2C+D), [Fast number parsing by Lemire](https://arxiv.org/pdf/2101.11408.pdf)
- Specialize algorithms in the range where IBig is inlined

