## Why rounding

Precision caps how many significant digits a significand may keep. When an
operation produces more digits than fit — the rule rather than the exception:
`1/3`, `sqrt(2)`, `exp(1)`, or simply adding numbers with different exponents
— the excess digits must be discarded, and *how* they are discarded is the
rounding mode.

## The rounding mode is a type parameter

`FBig<R, B>` carries its rounding mode as a type parameter `R: Round`. `R` is
a zero-sized marker type, so it costs nothing at runtime — but it is part of
the type, so two `FBig`s with different rounding modes are different types and
cannot be mixed in one operation (a compile error by design).

dashu provides six modes in `dashu_float::round::mode`:

| Mode | Behavior | Notes |
|------|----------|-------|
| `Zero` | toward zero (truncate) | default for `Real` (`FBig`) |
| `HalfEven` | nearest, ties to even | |
| `HalfAway` | nearest, ties away from zero | default for `Decimal` (`DBig`) |
| `Down` | toward −∞ | directed |
| `Up` | toward +∞ | directed |
| `Away` | away from zero | |

So `dashu::Real` is `FBig<Zero, 2>` and `dashu::Decimal` is
`FBig<HalfAway, 10>` (a.k.a. `DBig`).

## Switching modes

`with_rounding::<NewR>()` changes the type parameter; the value is unchanged
and the precision stays the same:

```rust
use dashu_float::{DBig, FBig};
use dashu_float::round::mode::Zero;

let a = DBig::from_parts(1234.into(), -2); // FBig<HalfAway, 10>
let b = a.with_rounding::<Zero>();         // FBig<Zero, 10>, same value
assert_eq!(a, b);
```

## Reporting the rounding direction

At the context layer, an inexact operation returns a `Rounded<T>` wrapper — an
`Approximation` carrying a `Rounding` flag. `Rounding` has three variants,
`NoOp` / `AddOne` / `SubOne`, describing the adjustment applied to the
truncated significand (not the direction of the error). The convenience layer
(operators, `.exp()`, …) unwraps this and returns a plain value, so you only
observe the direction when you call the context layer yourself.

## Precision and rounding are coupled

Both settings live in the same `Context<R>`: `Context::new(p)` fixes the
precision, and the `R` type parameter fixes the rounding. `with_precision(p)`
re-rounds using the *current* rounding mode; `with_rounding` changes the mode
without touching the value. To control both, chain the two methods or
construct a new context.

## Every operation is correctly rounded

dashu-float (and dashu-cmplx) guarantee that every operation returns the
**correctly rounded** result: the unique representable value closest to the
infinitely-precise real result under the current rounding mode — never a loose
tolerance, never 1-ulp-wrong. This is enforced by fuzz differentials against
MPFR/MPC that assert bit-exact agreement under every rounding mode.

The rounding modes that support this guarantee are those implementing
`ErrorBounds` — a bound on how far the true result can be from a computed
approximation. The float Ziv layer requires `R: ErrorBounds` for the
transcendentals, and all six modes implement it.

## How correct rounding is achieved: the Ziv retry loop

Transcendentals (`exp`, `ln`, `sin`, `cos`, `sqrt`, …) cannot be computed
exactly in finite time, so dashu uses a **Ziv retry loop**:

1. Evaluate the operation at `precision + guard` digits.
2. Compute a rigorous error bound around the result (via Ball arithmetic).
3. Round the interval to `precision` digits. If the rounded value is
   unambiguous — the error interval does not straddle a rounding boundary —
   certify it and return.
4. Otherwise double the guard and retry.

The loop almost always converges on the first retry; hitting the retry cap
indicates a bug in the radius-bound estimate, and the context layer reports
`FpError::ZivRetryLimitExceeded` explicitly instead of returning a value that
might be 1-ulp wrong.

The float Ziv driver (`float/src/ziv.rs`) certifies the real transcendentals;
the complex Ziv driver (`complex/src/ziv.rs`) wraps it to certify both parts
of a `CBig` result against the same preimage, so the complex transcendentals
are correctly rounded per component.
