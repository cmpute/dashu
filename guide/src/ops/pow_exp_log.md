`dashu` provides the power, exponential, logarithmic, and root families for the integer, float, and complex types. For `FBig`/`DBig` and `CBig`, like all inexact operations, these come in [two layers](../types.md#two-layer-api) — a `Context` layer that returns the rounding result and a convenience layer that unwraps it.

## Integer powers and roots

The integer types provide the full power/root family (the root methods come from the `SquareRoot` / `SquareRootRem` / `CubicRoot` / `CubicRootRem` traits in `dashu-base`):

- `pow(exp)` — exponentiation (`exp` is a `usize`).
- `sqr()` / `cubic()` — square / cube, cheaper than `pow(2)` / `pow(3)`.
- `sqrt()` / `sqrt_rem()`, `cbrt()` / `cbrt_rem()` — square / cube root, with the remainder variant
  returning `(root, remainder)` in one pass (for `sqrt_rem`, `root² ≤ self < (root+1)²`).
- `nth_root(n)` — n-th root, truncated toward zero.
- `ilog(&base)` — the truncated base-`base` logarithm.

`IBig::sqrt` returns a `UBig` and panics on a negative value; an even `nth_root` of a negative
`IBig` also panics. `ilog` panics on zero or a base of 0 or 1.

```rust
use dashu::base::{CubicRoot, SquareRoot, SquareRootRem};
use dashu::integer::UBig;

let n = UBig::from(10u32);
assert_eq!(n.pow(3), UBig::from(1000u32));
assert_eq!(n.sqr(), UBig::from(100u32));
assert_eq!(n.cubic(), UBig::from(1000u32));
assert_eq!(UBig::from(1000u32).sqrt_rem(), (UBig::from(31u8), UBig::from(39u8)));
assert_eq!(UBig::from(27u8).cbrt(), UBig::from(3u8));
```

## Real functions

- Exponential: `exp`, `exp_m1` ($e^x - 1$, accurate near zero).
- Logarithm: `ln`, `ln_1p` ($\ln(1+x)$, accurate near zero), plus the base-2 and base-10 logarithms `log2` and `log10` (correctly rounded; exact for exact powers of the base).
- Powers and roots: `powi(IBig)`, `powf(&FBig)`, `sqr`, `cubic`, `sqrt`, `cbrt`, `nth_root(&n)`, and `hypot(&other)` ($\sqrt{x^2+y^2}$, overflow-safe).
- Constants: `FBig::pi(precision)` for π and `FBig::e(precision)` for *e*. π benefits from reuse across calls via [`CachedFBig`](../cached.md); *e* is self-contained (it depends on no other constant) and is computed directly by binary splitting on `Σ 1/k!`, so it is not cached.

(`exp2`/`exp10` are deferred to a later release.)

## Complex functions

`CBig` mirrors the real set with `exp`, `ln`, `sqrt`, `powi`, and `powf`, built on the real implementations. The identities are

$$\exp(x+iy) = e^x(\cos y + i\sin y), \qquad \log z = \ln|z| + i\,\arg z,$$

with `ln`'s principal branch cut on $]-\infty, 0]$ — so the sign of an imaginary zero selects the side of the cut. See [Standards Compliance](../compliance.md) for the full C99 Annex G special-value and branch-cut tables.
