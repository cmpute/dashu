# Complex Numbers

`dashu-cmplx` provides the arbitrary-precision complex type [`CBig`](https://docs.rs/dashu-cmplx/latest/dashu_cmplx/struct.CBig.html),
the Rust-native alternative to GNU MPC. A `CBig` is a pair of real parts (`re`, `im`) sharing one
precision and one rounding mode, built on top of `dashu-float`'s `FBig`.

```rust,ignore
use dashu::{cbig, Complex};

let z = cbig!(11+100i); // 3 + 4i (coefficients are base-2 literals, like fbig!)
let w: Complex = Complex::from_parts(3.into(), 4.into());

let sum = &z + &w;          // field arithmetic: + - * / (and sqr, inv)
let mag = z.abs();          // |z| = hypot(re, im)  (a real FBig)
let phase = z.arg();        // atan2(im, re) ∈ ]-π, π]
let ez = z.exp();           // transcendentals: sqrt, exp, log, sin, cos, tan, asin, acos, atan
```

## Construction and conversion

Construct from two `FBig` parts, or embed a real value (imaginary part `+0`):

```rust,ignore
let z = Complex::from_parts(re, im);
let real_only: Complex = an_fbig.into();
```

The `cbig!` macro takes an algebraic `a+bi` literal or a `re, im` pair (coefficients use the `fbig!`
base-2 grammar):

```rust,ignore
use dashu::cbig;
let z = cbig!(1.01+1.1i); // base-2 coefficients
let p = cbig!(11, -100);  // pair form: 3 - 4i
```

## Two-layer API (like FBig)

Each operation has a **convenience** layer (returns a `CBig`, panics on domain/indeterminate errors)
and a **context** layer (`Context::op` returns `CfpResult = Result<CRounded<CBig>, FpError>`, carrying
per-axis inexactness `(Rounding, Rounding)`):

```rust,ignore
use dashu::complex::Context;
let ctx = Context::new(53);
let r = ctx.mul(&z, &w)?;      // CRounded<CBig> with per-part inexact flags
```

## No-NaN special-value model

`CBig` has **no NaN**: C99 complex cases that would produce NaN (`0/0`, `∞−∞`, `0·∞`, `log(0)`,
…) map to `FpError` at the context layer (and panic at the convenience layer), exactly mirroring
`FBig`. Infinities are **terminal values**; the Riemann point at infinity is the single `+∞ + i·0`
produced by `proj`. IEEE signed zero (`-0`) is first-class and selects the side of a branch cut
(e.g. `log(-r ± i0) = ln r ± iπ`, `sqrt(conj z) == conj(sqrt z)`).

## Rounding

Components are rounded independently with the single mode `R`, after each op feeds them enough guard
precision — the same *near-correctly-rounded* guarantee class `dashu-float`'s transcendentals carry.
A guaranteed-correct Ziv retry loop is deferred to 0.5.x. See the
[IEEE 754 compliance](./ieee754.md) chapter for the signed-zero / infinity details (which `CBig`
inherits from `FBig`).
