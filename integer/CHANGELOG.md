# Changelog

## Unreleased

### Add
- NTT-based multiplication using Proth primes (`K·2^N + 1`), combined via Garner CRT.  Supports 64-bit and 32-bit Word targets.  Threshold at 4 000 words (~256 kbits).
- Asymmetric NTT chunking: when one operand is much larger than the other, the shorter operand is forward-transformed once and reused across chunks.
- `UBig::from_u64` and `IBig::from_i64`, const on 32-bit and 64-bit targets.
- Specialized Karatsuba squaring: uses 3 recursive squarings instead of 3 multiplications, with simplified diff handling.
- Specialized Toom-Cook-3 squaring: evaluates a single polynomial instead of two, 5 recursive squarings instead of multiplications.
- Specialized NTT squaring: single forward transform instead of two, pointwise square instead of multiply.
- Squaring thresholds can be overridden at runtime via `DASHU_THRESHOLD_SIMPLE_SQR`, `DASHU_THRESHOLD_KARATSUBA_SQR`, and `DASHU_THRESHOLD_NTT_SQR` environment variables (requires `tuning` feature).

### Improve
- Basecase (schoolbook) multiplication now uses an dword mult inner kernel (two multiplier words per sweep over the accumulator, via the `add_mul_dword_same_len_in_place` and `sub_mul_dword_same_len_in_place` kernels), roughly halving accumulator memory traffic.
- Basecase (schoolbook) squaring's off-diagonal phase now uses the same two-word kernel as multiplication, pairing consecutive limbs `(a[i], a[i+1])` against their shared suffix `a[i+2..]`. This halves the accumulator traffic of the basecase, speeding up squaring ~25% in the schoolbook range (≤30 words) and ~12-17% through the Karatsuba/Toom-3 bands that recurse into it; `ubig_pow` improves likewise since exponentiation is squaring-dominated.
- Addition and subtraction carry/borrow propagation now uses `Word` (u64/u32) instead of `bool` throughout the architecture-specific `add_with_carry` and `sub_with_borrow` functions, eliminating `bool`↔Word conversions in the inner loops.
- Lowered the Karatsuba→Toom-3 multiplication threshold from 192 to 96 words, giving Toom-Cook-3 at ~6000 bits instead of ~12000 bits — closes the gap with malachite at ~10000-bit sizes.
- NTT coefficient width increased from 16 to 64 bits (K_eff=3 for 64-bit, K_eff=2 otherwise), roughly halving the transform length at each step.
- NTT multiplication auto-selects `K_eff = 2` primes when headroom allows, skipping the third prime.
- Multiplication thresholds can be overridden at runtime via `DASHU_THRESHOLD_SIMPLE_MUL`, `DASHU_THRESHOLD_KARATSUBA_MUL`, and `DASHU_THRESHOLD_NTT_MUL` environment variables (requires `tuning` feature).
- Non-power-of-2 radix formatting now preallocates the `radix_powers`/`big_chunks` vectors (capacity estimated from the number's length) instead of growing them push-by-push.
- The basecase addmul/submul-2 kernels (`add_mul_dword_same_len_in_place`, `sub_mul_dword_same_len_in_place`) now dispatch at runtime to a BMI2 build of identical arithmetic on x86-64 (when the `std` feature is enabled and the CPU supports `bmi2`); LLVM lowers the widening multiplies to the flag-free `mulx` instruction and unrolls the loop. This speeds the kernel ~4-5% in isolation. Other targets and `no_std` builds keep the portable path.

### Change
- Multiplication threshold env vars renamed with `_MUL` suffix: `DASHU_THRESHOLD_SIMPLE_MUL`, `DASHU_THRESHOLD_KARATSUBA_MUL`, `DASHU_THRESHOLD_NTT_MUL` (was without suffix).
- NTT multiplication now uses Proth primes (`K·2^N + 1`) instead of Solinas primes, improving modular reduction speed.
- NTT threshold lowered from 40 000 to 4 000 words.
- NTT enabled for 32-bit Word targets.
- Arch-specific NTT prime definitions under `arch/generic_{32,64}_bit/ntt.rs`.

### Fix
- Modular exponentiation and `Reduced::sqr` under-allocated scratch memory for squaring (they sized it using the multiplication budget), which could exhaust the scratch allocator mid-recursion for moduli in the Karatsuba band (e.g. the Mersenne-prime `test_pow` case). The pow path now reserves `max(mul, sqr)` scratch and `Reduced::sqr` uses the dedicated squaring budget.
- Unused imports (`Sign`, `debug_assert_zero`) in `sqr/mod.rs` on 16-bit Word targets, where the NTT arm is compiled out.
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
