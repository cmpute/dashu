# Changelog

## Unreleased

### Add
- NTT-based multiplication using Proth primes (`K·2^N + 1`), combined via Garner CRT.  Supports 64-bit and 32-bit Word targets.  Threshold at 4 000 words (~256 kbits).
- Asymmetric NTT chunking: when one operand is much larger than the other, the shorter operand is forward-transformed once and reused across chunks.
- `UBig::from_u64` and `IBig::from_i64`, const on 32-bit and 64-bit targets.
- `monty` module: Montgomery modular arithmetic for odd moduli. New [`MontgomeryRepr`](integer::monty::MontgomeryRepr) (precomputed Montgomery constants) and [`Montgomery`](integer::monty::Montgomery) (values in Montgomery form) with multiplication, squaring, addition, subtraction, negation, doubling, exponentiation, and inversion. Single/double-word moduli delegate to `num-modular`; multi-word moduli use word-by-word REDC with a double-word "addmul_2" kernel (the operand product reuses the crate's fast multiply), beating the Barrett division path of `modular::Reduced` for multiplication/squaring/exponentiation at roughly 256–4096 bits. For inverse-heavy computation, `modular::Reduced` remains faster (a Montgomery inverse must convert out of and back into Montgomery form).

### Improve
- Basecase (schoolbook) multiplication now uses an dword mult inner kernel (two multiplier words per sweep over the accumulator, mirroring GMP's `mpn_addmul_2` and `mpn_submul_2`), roughly halving accumulator memory traffic.
- Addition and subtraction carry/borrow propagation now uses `Word` (u64/u32) instead of `bool` throughout the architecture-specific `add_with_carry` and `sub_with_borrow` functions, eliminating `bool`↔Word conversions in the inner loops.
- Lowered the Karatsuba→Toom-3 multiplication threshold from 192 to 96 words, giving Toom-Cook-3 at ~6000 bits instead of ~12000 bits — closes the gap with malachite at ~10000-bit sizes.
- NTT coefficient width increased from 16 to 64 bits (K_eff=3 for 64-bit, K_eff=2 otherwise), roughly halving the transform length at each step.
- NTT multiplication auto-selects `K_eff = 2` primes when headroom allows, skipping the third prime.
- Multiplication thresholds can be overridden at runtime via `DASHU_THRESHOLD_SIMPLE`, `DASHU_THRESHOLD_KARATSUBA`, and `DASHU_THRESHOLD_NTT` environment variables (requires `tuning` feature).
- Division threshold (schoolbook ↔ divide-and-conquer crossover) can be overridden at runtime via the `DASHU_THRESHOLD_SIMPLE_DIV` environment variable (requires `tuning` feature). Values below 3 are clamped to 3 to uphold the divide-and-conquer algorithm's `n_lo >= 2` invariant.
- Montgomery multiplication: the `sqr()` method and `pow_nontrivial` entry point now avoid a redundant clone+overwrite of the multi-word value, saving one `Box<[Word]>` allocation and an `s`-word copy per squaring.
- Montgomery multiplication: `mul_in_place_large` uses pointer-identity instead of full element comparison to detect self-multiplication (`a *= a`), avoiding an `O(s)` scan on the common distinct-operands path.
- Montgomery multiplication: `mul_normalized_large` and `sqr_normalized_large` share a common `finish_monty_product` helper for the REDC+canonicalize pipeline tail.

### Change
- NTT multiplication now uses Proth primes (`K·2^N + 1`) instead of Solinas primes, improving modular reduction speed.
- NTT threshold lowered from 40 000 to 4 000 words.
- NTT enabled for 32-bit Word targets.
- Arch-specific NTT prime definitions under `arch/generic_{32,64}_bit/ntt.rs`.

### Fix
- `pack.rs` test used 64-bit literals that overflowed `Word` (`u32`) on 32-bit targets, breaking the test build.
- `pack.rs` now uses native `Word`/`Lane` types throughout instead of `u64`/`u32`, fixing clippy `unnecessary_cast` warnings on 64-bit.
- `test_unpack_carry_propagation` had a hardcoded 64-bit shift assumption; now derived from `Word::BITS` so it works on 32-bit.
- Various clippy warnings (`let_and_return`, `too_many_arguments`, `needless_range_loop`, `type_complexity`) resolved across the NTT module.

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
