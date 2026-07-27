`FBig`/`DBig` provide the exponential, logarithmic, power, and root families, plus the mathematical constants. `CBig` provides the complex analogs of each. Like all inexact operations, these come in [two layers](../types.md#two-layer-api) — a `Context` layer that returns the rounding result and a convenience layer that unwraps it.

## Real functions

- Exponential: `exp`, `exp_m1` ($e^x - 1$, accurate near zero).
- Logarithm: `ln`, `ln_1p` ($\ln(1+x)$, accurate near zero).
- Powers and roots: `powi(IBig)`, `powf(&FBig)`, `sqrt`, `cbrt`, `nth_root(&n)`, and `hypot(&other)` ($\sqrt{x^2+y^2}$, overflow-safe).
- Constants: `FBig::pi(precision)` for π and `FBig::e(precision)` for *e*. π benefits from reuse across calls via [`CachedFBig`](../cached.md); *e* is self-contained (it depends on no other constant) and is computed directly by binary splitting on `Σ 1/k!`, so it is not cached.

(`exp2`/`exp10`/`log2`/`log10` are deferred to a later 0.5.x release.)

## Complex functions

`CBig` mirrors the real set with `exp`, `ln`, `sqrt`, `powi`, and `powf`, built on the real implementations. The identities are

$$\exp(x+iy) = e^x(\cos y + i\sin y), \qquad \log z = \ln|z| + i\,\arg z,$$

with `ln`'s principal branch cut on $]-\infty, 0]$ — so the sign of an imaginary zero selects the side of the cut. See [Standards Compliance](../compliance.md) for the full C99 Annex G special-value and branch-cut tables.
