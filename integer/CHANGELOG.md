# Changelog

## 0.4.1

- Implemented `AbsEq` and `AbsOrd` for `UBig` and `IBig`.
- Add `UBig::from_static_words` and `IBig::from_static_words` (both Rust 1.64+) to support the `static_ubig!` and `static_ibig!` macros.
- Add `is_multiple_of` and `is_multiple_of_const` (Rust 1.64+) for `UBig`/`IBig`
- Constify `trailing_zeros` and `trailing_ones` of `UBig`/`IBig` (Rust 1.64+).
- `IBig::trailing_ones` bug fixed.

## 0.4.0

### Add

- Add a `ConstDivisor` type that supports faster division when you have an invariant number as the divisor.
- Add `as_ibig` method to `UBig`.
- Implement `num_order::NumOrd` trait between `UBig` and `IBig`
- Implement `num_modular::Reducer` trait for `ConstDivisor`

### Change

- The serialization format with `serde` for `UBig` and `IBig` has been changed. Now both types will be serialize as a sequence of little-endian bytes.
- Now feature `num-traits` and `rand` are not enabled by default, feature `num-order` is enabled instead.
- The `IntoModule` trait is refactored into the `IntoRing` trait, which has an additional type parameter for the ring. This is used for potential Montgomery implementation in future.
- The `IntoRing` trait is no longer implemented for reference types `&UBig` and `&IBig` to make the copying explicit.
- The `Modulo` type is renamed to `Reduced` to prevent confusion.
- `From<&UBig>` implementation for `IBig` and `TryFrom<&IBig>` for `UBig` are removed to prevent implicit cloning.
- `BitAnd` for `UBig` and other primitive integer types now will always return the result with primitive integer type.

### Remove

- The comparison traits `PartialOrd` and `PartialEq` are no longer implemented between `UBig` and `IBig`. Use `num_order::NumOrd` instead.

## 0.3.1

- Add struct `crate::rand::UniformBits` for generating random integers with given bit lenght limit.
- Add `count_ones()` and `count_zeros()` for `UBig`
- Add `cubic()` for `UBig` and `IBig`
- Add `rand_v08` and `num-traits_v02` feature flags to prevent breaking changes due to dependency updates in future 

## 0.3.0

### Add

- Implement `Gcd::gcd` and `ExtendedGcd::gcd_ext` between `UBig` and `IBig`
- Implement `DivRem::div_rem` between `UBig` and `IBig`
- Implement `dashu_base::BitTest` for `IBig`
- Implement `Div` and `DivAssign` for `Modulo`
- Add `trailing_ones` for `UBig` and `IBig`
- Implement `TryFrom<f32>` and `TryFrom<f64>` for `UBig` and `IBig`
- Implement `num_order::{NumOrd<f32>, NumOrd<f64>` for `UBig` and `IBig`

### Change

- `sqrt_rem` is only exposed through the `dashu_base::RootRem` trait now.
- `abs_cmp` is only exposed throught the `dashu_base::AbsCmp` trait now.
- `abs_eq` is only exposed throught the `dashu_base::AbsEq` trait now.
- `bit_len` and `bit` are only exposed throught the `dashu_base::BitTest` trait now.
- `Modulo::inv` now takes the reference of a `Modulo`.
- `to_le_bytes` and `to_be_bytes` now return a boxed array `Box<[u8]>` instead of a `Vec<u8>`
- `IBig::square()` now returns `UBig` instead of `IBig`

### Remove

- `error::{OutOfBoundsError, ParseError}` are removed, related error types are added to `dashu-base`
- `PartialOrd` and `PartialEq` is not implemented for primitive integers any more. Please use `num_order::NumOrd`
  for comparison. (See [`num-bigint`#150](https://github.com/rust-num/num-bigint/issues/150))
- `num-integer` feature is not enabled by default now.

## 0.2.1

- Add `sqrt`, `sqrt_rem`, `nth_root` for `UBig` and `IBig`
- Implement `core::iter::{Sum, Product}` for `UBig` and `IBig`
- Implement `num_traits::{Euclid, ToPrimitive, FromPrimitive}` for `UBig` and `IBig`
- Implement `num_integer::{Integer, Roots}` for `UBig` and `IBig`
- Implement `num_order::{NumHash, NumOrd}` for `UBig` and `IBig`
- Implement `zeroize::Zeroize` for `UBig`, `IBig` and internal types
- `serde` se/derialization now supports the `is_human_readable()` flag

## 0.2.0

### Add

- Expose the `Sign` enum and related operations with `UBig` and `IBig`
- Expose `DoubleWord` for easier operation with `Word`
- Add a public API `as_words` and `as_sign_words` to access internal representation of `UBig` and `IBig` respectively.
- Add const constructors `from_word`, `from_dword` and a direct constructor `from_words` for `UBig`.
- Add a const constructor `from_parts_const` and a director constructor `from_parts` for `IBig`
- Add `split_bits` and `clear_high_bits` for `UBig`.
- Add `remove` for `UBig`
- Add `abs_cmp`, `abs_eq` for `IBig`.
- Implement `Mul` between `Sign` and `UBig`/`IBig`.
- Implement `DivRemAssign` for `UBig` and `IBig`, and `DivRemAssign` is re-exported in the `ops` module.
- Implement integer logarithm `ilog` and approximated bounds of base 2 logarithm `log2_bounds`.

### Remove
- Remove `ubig!` and `ibig!` macros from the crate, more powerful versions of them will be included in a separate `dashu-macro` crate.

### Change

- Function `zero()`, `one()`, `neg_one()` are changed to associated constants `ZERO`, `ONE`, `NEG_ONE`.
- Function `gcd()` and `extended_gcd()` of `UBig` and `IBig` are changed to be associated functions of `Gcd` and `ExtendedGCD`.
- Parsing a string with unsupported radix will now return an Err instead of `panic!`.
- `from_str_with_radix_prefix` now also return the radix.
- Due to the requirement of `dashu-float`, the MSRV is now 1.61. Rust versions from 1.57 to 1.60 are still working for `dashu-int` in this version, but it won't be ensured in future releases.

### Improve
- Parsing integers from string support underscore separater.
- Improve speed for power function `pow()`

## 0.1.1

- Implemented modular inverse for the `Modulo` type.
- Implemented `gcd` and `extended_gcd` for `UBig` and `IBig`.

## 0.1.0

The code for big integer is ported from `ibig @ 0.3.5` with modifications stated in the [NOTICE](./NOTICE.md), the current MSRV for `dashu-int` is 1.57.

# Todo

- Division: trim the trailing zero words before division.
- GCD: trim the trailing zero words, and give the number of zeros as input to low-level algorithms.
- GCD: An idea of fast gcd check for rational number: don't do gcd reduction after every operation.
  For small numerators or denominators, we can directly do a gcd, otherwise, we first do gcd with a primorial that
  fits in a word, and only remove these small divisors.
  Further improvement: store a const divisor for the prime factors in the primorial, thus supports a fast factorial of
  the gcd result, and then divide with these const divisor.
- Power: implement a k-ary pow when exponent is too large (after lifting to at least a full word), this will store pre-computed 2^1~2^k powers. Maybe move this implementation to a separate module folder, and use the window selection function from modular pow.
- Logarithm: for very large est value, the est error can be large and there can be many fixing steps,
  we should use a similar strategy as the non_power_two formatter, using power sequence,
  or call log again on the target / est_pow
