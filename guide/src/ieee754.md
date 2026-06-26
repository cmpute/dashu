# IEEE 754-2008 Compliance of dashu-float

This document describes where `dashu-float`'s `FBig` type is compliant and where it deviates
from IEEE 754-2008. The reference is IEEE Std 754™-2008 (ISO/IEC/IEEE 60559:2011).

dashu-float is an **arbitrary-precision** floating-point library. Many IEEE 754 concepts
(e.g. fixed-width encoding, subnormals, NaN payloads) have no direct equivalent here.
Where infinite precision makes the standard's rules natural to satisfy, they are satisfied;
where they conflict with the arbitrary-precision model, the deviation is noted.

## Data Model

### Section 3 — Floating-point formats

| IEEE 754 requirement | Compliance | Notes |
|---------------------|-----------|-------|
| Binary and decimal formats | ✅ Supported | `FBig<Rounding, 2>` (binary) and `DBig` = `FBig<HalfAway, 10>` (decimal). Other bases are supported via the `const BASE: Word` parameter. |
| Finite non-zero numbers | ✅ | Represented as `significand × BASE^exponent` with unbounded significand. |
| Signed zero (`±0`) | ✅ | Encoded via exponent sentinels: `+0` ↔ exponent `0`, `-0` ↔ exponent `-1`. Produced by arithmetic, rounding, and cancellations per IEEE 754. |
| Signed infinity (`±∞`) | ✅ | Encoded via exponent sentinels: `+∞` ↔ `isize::MAX`, `-∞` ↔ `isize::MIN`. |
| NaN | ❌ Deviates | No NaN. Invalid operations panic (at the `FBig` convenience layer) or return `Err(FpError)` (at the `Context` layer). |
| Subnormals | N/A | Arbitrary-precision significands eliminate the need for subnormals. Any non-zero number is normalized. |
| Fixed-width encoding | N/A | No fixed bit widths; significands are unbounded `IBig` integers. |

## Arithmetic Operations

### Section 5 — Operations

| IEEE 754 requirement | Compliance | Notes |
|---------------------|-----------|-------|
| `±0` compare equal | ✅ | `+0 == -0` in `PartialEq`, `Ord`, `NumOrd`. |
| `±∞` compare equal to same sign, ordered vs finite | ✅ | `+∞ == +∞`, `+∞ > finite`, `-∞ < finite`. |
| Overflow → `±∞` (with rounding-mode-dependent sign) | ✅ | Detected at the Repr level, returned as `Err(FpError::Overflow(sign))` at `Context`, converted to signed infinity at `FBig`. |
| Underflow → `±0` | ✅ | Same mechanism as overflow. |
| `finite / ±0` → `±∞` | ✅ | Produced as a value (not an error). Sign = XOR of operand signs. |
| `0 / 0` → NaN / error | ⚠️ Partial | Returns `Err(FpError::Indeterminate)`. Panics at the `FBig` layer. |
| `∞ ± finite`, `∞ × finite`, etc. → `±∞` | ❌ Deviates | Infinities are terminal: feeding them into arithmetic returns `Err(FpError::InfiniteInput)`. Operations on infinities require explicit handling at the `Context` layer or use special-case methods. |
| `exp(+∞)` = `+∞` | ✅ | `Context::exp` accepts infinite input. |
| `exp(-∞)` = `+0` | ✅ | Same. |
| `exp_m1(+∞)` = `+∞` | ✅ | |
| `exp_m1(-∞)` = `-1` | ✅ | |
| `ln(±0)` = `-∞` | ✅ | Produced as a value. |
| `sqrt(-0)` = `-0` | ✅ | |
| `sin(-0)` = `-0` | ✅ | |
| Cancellation under roundTowardNegative → `-0` | ✅ | `cancel_zero` in add.rs produces `-0` when `R::IS_ROUND_TOWARD_NEGATIVE`. |
| Exact subtraction cancels to `-0` only under directed rounding | ✅ | IEEE 754 §6.3: `(-3) + 3` = `+0` under roundTiesToEven/Up, `-0` under Down. |

### Section 5.3 — Rounding

| IEEE 754 requirement | Compliance | Notes |
|---------------------|-----------|-------|
| Rounding modes: roundTiesToEven, roundTiesToAway, roundTowardPositive, roundTowardNegative, roundTowardZero | ✅ | All five modes implemented as `HalfEven`, `HalfAway`, `Up`, `Down`, `Zero`. |
| Correct rounding to within 1 ulp | ✅ | All operations guarantee `|error| < 1 ulp`. The `Rounded` type distinguishes exact from inexact results. |
| Round-to-nearest preserves sign of zero | ✅ | `rounded_to_repr` preserves input sign when rounding collapses a non-zero to zero. |

### Section 5.6 — Sign bit operations

| IEEE 754 requirement | Compliance | Notes |
|---------------------|-----------|-------|
| `abs(x)` always non-negative | ✅ | `FBig::abs()` converts `-0` to `+0`. |
| `neg(x)` toggles sign of `±0` and `±∞` | ✅ | Correctly flips exponent sentinels via `negate_special_exponent`. |
| `signum(±0)` = `±0` | ✅ | Returns `+0` for both `+0` and `-0`. |
| `sign()` distinguishes `+0` from `-0` | ✅ | `Repr::sign()` returns `Negative` for `-0`. |

## Conversions

| IEEE 754 requirement | Compliance | Notes |
|---------------------|-----------|-------|
| `f32`/`f64` round-trip preserves `-0` | ✅ | `FBig::try_from(-0.0f64)` produces `-0`. |
| `f32`/`f64` round-trip preserves infinity | ✅ | |
| Overflow in conversion to `f32`/`f64` produces `±∞` | ✅ | `into_f32_internal` / `into_f64_internal` check exponent bounds. |
| Underflow in conversion to `f32`/`f64` produces `±0` | ✅ | Same. |
| Int-to-float conversion exact for representable integers | ✅ | |
| Float-to-int overflows saturate (per Rust convention) | N/A | Rust's `TryFrom` returns an error on overflow; `ToPrimitive` returns `None`. |

## Exceptional Conditions

| IEEE 754 requirement | Compliance | Notes |
|---------------------|-----------|-------|
| Invalid operation → NaN | ❌ | Panics (`FBig`) or returns `Err(FpError)` (`Context`). |
| Divide by zero → `±∞` (no trap) | ✅ | |
| Overflow → `±∞` (no trap) | ✅ | Detected and propagated. |
| Underflow → `±0` (no trap) | ✅ | Same. |
| Inexact flag | ⚠️ Partial | The `Rounded<T>` type carries `Exact`/`Inexact(T, Rounding)` to signal whether rounding occurred, but there is no sticky flag mechanism. |

## Summary

| Category | Status |
|----------|--------|
| Signed zeros | ✅ Fully compliant |
| Signed infinities | ✅ Fully compliant |
| Overflow/underflow | ✅ Fully compliant |
| Directed rounding | ✅ Fully compliant |
| NaN handling | ❌ Panics (by design) |
| Infinite operands in arithmetic | ❌ Error (by design — infinities are terminal) |
| Subnormals | N/A (unbounded precision) |
| Exception flags | ⚠️ Rounded type signals exact/inexact, no sticky flags |
