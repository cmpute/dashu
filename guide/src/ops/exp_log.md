# Exponential and Logarithm

`FBig`/`DBig` provide the exponential, logarithmic, power, and root families, plus the mathematical constants. `CBig` provides the complex analogs of each.

## Two-layer API

Like all inexact operations, transcendentals come in two layers (see [types](../types.md)):

- **Context layer** — `Context` methods take a `&Repr` and return `FpResult<Rounded<FBig>>` (a correctly-rounded result or an `FpError`), carrying the rounding direction. They accept an optional `&mut ConstCache` for constant reuse.
- **Convenience layer** — methods on `FBig` (`.exp()`, `.ln()`, …) unwrap to a plain `FBig`, panicking on `Indeterminate`/`OutOfDomain`/`InfiniteInput` and saturating overflow/underflow to `±∞`/`±0`.

## Real functions

- Exponential: `exp`, `exp_m1` ($e^x - 1$, accurate near zero).
- Logarithm: `ln`, `ln_1p` ($\ln(1+x)$, accurate near zero).
- Powers and roots: `powi(IBig)`, `powf(&FBig)`, `sqrt`, `cbrt`, `nth_root(&n)`, and `hypot(&other)` ($\sqrt{x^2+y^2}$, overflow-safe).
- Constants: `FBig::pi(precision)` computes π; use [`CachedFBig`](../construct.md#cached-arithmetic-for-fbig) to reuse it across calls.

(`exp2`/`exp10`/`log2`/`log10` are deferred to a later 0.5.x release.)

## Complex functions

`CBig` mirrors the real set with `exp`, `ln`, `sqrt`, `powi`, and `powf`, built on the real implementations. The identities are

$$\exp(x+iy) = e^x(\cos y + i\sin y), \qquad \log z = \ln|z| + i\,\arg z,$$

with `ln`'s principal branch cut on $]-\infty, 0]$ — so the sign of an imaginary zero selects the side of the cut. See [Standards Compliance](../compliance.md) for the full C99 Annex G special-value and branch-cut tables.
