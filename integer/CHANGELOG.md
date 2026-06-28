# Changelog

## Unreleased

### Add
- `UBig::to_digits` / `UBig::from_digits`: convert to and from a sequence of base-`B` digits (base `2..=Word::MAX`, digits stored as `Word`, most-significant first). Complements [`UBig::in_radix`] which is limited to base 2..=36.

### Fix
- `UBig::nth_root` / `IBig::nth_root` (and `cbrt`) of `0` returned `1` instead of `0`: the `bits <= n`
  shortcut fired for the zero input (bit length 0). Found by the new `fuzz/` `rug::Integer` oracle.

### Change
- **(breaking)** `IBig`'s serde non-human-readable format switched from the custom byte-length-parity encoding to standard two's complement little-endian bytes (matching [`IBig::to_le_bytes`]), for interop robustness. Previously serialized data is not compatible.
- **(breaking)** `UBig::in_radix` and `IBig::in_radix` now take `radix: u8` (was `u32`); the internal `Digit` type alias is now `u8`. `from_str_with_radix_prefix` / `from_str_with_radix_default` now expose the detected/default radix as `u8` (was `u32`). `from_str_radix` keeps its `u32` argument for `std` parity.

## 0.4.3

### Add
- NTT-based multiplication using Proth primes (`K·2^N + 1`) combined via Garner CRT, with 64-bit transform coefficients. Enabled on both 64-bit and 32-bit `Word` targets; activates above ~4 000 words (~256 kbits). Auto-selects `K_eff = 2` primes when headroom allows.
- Asymmetric NTT: when one operand is much larger than the other, the shorter operand is forward-transformed once and reused across chunks.
- `monty` module: Montgomery modular arithmetic for odd moduli. `MontgomeryRepr` (precomputed constants) and `Montgomery` (values in Montgomery form) support multiplication, squaring, addition, subtraction, negation, doubling, exponentiation, and inversion.
- Specialized squaring paths for Karatsuba, Toom-Cook-3, and NTT (recursive squarings instead of multiplications / a single forward transform), with separately tunable thresholds.
- `UBig::from_u64` and `IBig::from_i64` const constructors (on 32-bit and 64-bit targets).
- Optional `rand_v09` (rand 0.9, MSRV 1.63) and `rand_v010` (rand 0.10, MSRV 1.85) features mirroring `rand_v08`. The default `rand` feature still maps to `rand_v08`.
- The random-integer distributions (`UniformBits`, `UniformBelow`, `UniformUBig`, `UniformIBig`) and their sampling now live once in the version-agnostic `dashu_int::rand` module, which exposes a `BitRng` trait and `bridge_v08` / `bridge_v09` / `bridge_v010` constructors. The per-version modules are now private trait bindings.

### Improve
- Basecase (schoolbook) multiplication and squaring now use a double-word addmul inner kernel, roughly halving accumulator memory traffic; on x86-64 with `std` it dispatches to a BMI2 build lowered to `mulx`.
- Addition and subtraction carry/borrow propagation now uses `Word` instead of `bool`, eliminating conversions in the inner loops.
- Lowered the Karatsuba→Toom-3 multiplication threshold from 192 to 96 words (~6 000 bits instead of ~12 000 bits).
- `UBig::nth_root` for large composite `n` decomposes into a chain of smaller roots via small-prime factor reduction, avoiding the prohibitively expensive Newton term.
- Integer logarithm for very large values uses power-sequence decomposition instead of iterative single-step multiplication.
- Non-power-of-2 radix formatting preallocates its conversion buffers instead of growing them incrementally, and optimizes single-digit writes.
- Multiplication, squaring, and division thresholds can be overridden at runtime via `DASHU_THRESHOLD_*` environment variables (requires `tuning` feature): `DASHU_THRESHOLD_{SIMPLE,KARATSUBA,NTT}_MUL`, `DASHU_THRESHOLD_{SIMPLE,KARATSUBA,NTT}_SQR`, and `DASHU_THRESHOLD_SIMPLE_DIV`.

### Fix
- Fix modular exponentiation and `Reduced::sqr` under-allocating scratch for squaring, which could exhaust the scratch allocator mid-recursion for moduli in the Karatsuba band.
- Fix a panic in the extended GCD when one operand divides the other.
- Fix `IBig::{to_le_bytes, to_be_bytes}` for negative powers of two.
- Fix `IBig >> n` (shift by `n >= bit length`) on `DoubleWord`-magnitude values.
- Fix the 32-bit `Word` build: the NTT pack code, test literals, and unused-import warnings on targets where the NTT arm is compiled out.

## 0.4.2

- Add `UBig::ones`.
- Add `IBig::as_ubig`.
- Add `UBig::from_chunks` and `UBig::to_chunks`.
- Implement `TryFrom<UBig>` and `TryFrom<IBig>` for `f32`/`f64`.
- Implement `IBig::{from_le_bytes, from_be_bytes}` and `IBig::{to_le_bytes, to_be_bytes}`.
- The alterative `Debug` output of `UBig` and `IBig` will include `(digits: x, bits: y)` instead of `(x digits, y bits)`.
- Implement bit operations (`BitAnd`, `BitOr`, `BitXor`) between `UBig` and `IBig`, and between `IBig` and unsigned primitive integers.
- Fix a bug in `UBig::split_bits` and `UBig::clear_high_bits`.
- Reduce unsafe code ([#52](https://github.com/cmpute/dashu/pull/52) thanks to @eduardosm).
- Fix `words_to_chunks` panic on 32-bit `Word` targets. ([#63](https://github.com/cmpute/dashu/pull/63)).
- FIx bugs in `to_f64` and `sqrt_rem_large` ([#64](https://github.com/cmpute/dashu/pull/64)).
- Bump MSRV from 1.61 to 1.68.

## 0.4.1

- Implement `AbsEq` and `AbsOrd` for `UBig` and `IBig`.
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
