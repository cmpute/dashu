## What precision is

An `FBig<R, B>` value is stored as `significand × B^exponent`, together with a
`Context<R>` that carries two settings:

- **precision** — the maximum number of significant base-`B` digits the
  significand may keep;
- **rounding mode** — what happens to the digits that do not fit (see
  [Rounding](./rounding.md)).

`Context::new(p)` creates a context with precision `p`; `p == 0` means
**unlimited precision** (no cap at all). Read the current cap with
`FBig::precision()`:

```rust
use dashu_float::{Context, DBig, Repr};
use dashu_float::round::mode::HalfAway;

let context = Context::<HalfAway>::new(20);
let a = DBig::from_repr(Repr::new(1234.into(), -2), context);
assert_eq!(a.precision(), 20);
```

## Where the precision comes from at construction

Most constructors pick the precision for you, inferred from the input; only
the context-based ones let you set it explicitly:

| Constructor | Result precision |
|---|---|
| `from_parts(significand, exponent)` | Inferred — the exact number of significant base-`B` digits in `significand` (at least 1) |
| `from_parts_const(sign, significand, exponent, min_precision)` | Inferred, or `max(inferred, min_precision)` when `min_precision` is `Some` |
| `from_repr(repr, context)` | Taken directly from the given `Context` |
| `From<integer>` / `FromStr` | Inferred from the significant digits of the input |
| Constants (`ONE`, `ZERO`, …) / `from_repr_const` | Unlimited (`Context::new(0)`) |
| `Context::new(p)` + `Context::max(lhs, rhs)` | The explicit knobs; `max` picks the higher precision of two contexts |

```rust
use core::str::FromStr;
use dashu_base::Sign;
use dashu_float::{Context, DBig};
use dashu_float::round::mode::Zero;

// from_parts: precision inferred from the significand
let a = DBig::from_parts(1234.into(), -2);   // 12.34
assert_eq!(a.precision(), 4);                // "1234" has 4 digits

// from_parts_const: max(inferred, min_precision)
let b = DBig::from_parts_const(Sign::Negative, 1234, -2, Some(6));
assert_eq!(b.precision(), 6);

// parsing infers the precision from the significant digits
assert_eq!(DBig::from_str("2.3450")?.precision(), 5);

// constants are unlimited
assert_eq!(DBig::ONE.precision(), 0);

// Context::max picks the higher precision
let c1 = Context::<Zero>::new(10);
let c2 = Context::<Zero>::new(50);
assert_eq!(Context::max(c1, c2).precision(), 50);
# Ok::<(), dashu_base::ParseError>(())
```

## How operators propagate precision

Binary arithmetic (`+`, `-`, `*`, `/`) computes the result context as
`Context::max(lhs.context, rhs.context)` — the **higher** precision wins, and
the result is rounded to it. Mixing a 2-digit and a 30-digit value yields a
30-digit result:

```rust
# use core::str::FromStr;
# use dashu_float::DBig;
let lo = DBig::from_str("2.0")?;
let hi = DBig::from_str("1.23456789012345678901234567890")?; // 30 digits
assert_eq!(lo.precision(), 2);
assert_eq!(hi.precision(), 30);

let sum = lo + hi;
assert_eq!(sum.precision(), 30); // the higher precision wins
# Ok::<(), dashu_base::ParseError>(())
```

Other rules worth knowing:

- `Sum` over an iterator is **correctly rounded**: the addends are accumulated
  exactly and rounded once, at the `max` precision over the addends, instead
  of folding with `+` (which rounds at every step and can drop small
  addends). `Product` is a plain fold.
- An inexact addition or subtraction may carry a single **guard digit**: the
  result significand can briefly hold up to `precision + 1` digits before the
  next operation rounds it back. This is an internal detail; you normally
  don't observe it.
- `with_precision(p)` explicitly re-rounds to `p` digits. Widening is always
  exact; shrinking rounds per the context's rounding mode and returns a
  `Rounded` wrapper reporting whether the rounding was `Exact` or `Inexact`
  (with the direction):

```rust
# use core::str::FromStr;
# use dashu_base::Approximation::*;
# use dashu_float::DBig;
use dashu_float::round::Rounding::*;

let a = DBig::from_str("2.345")?;
assert_eq!(a.precision(), 4);
assert_eq!(a.clone().with_precision(3), Inexact(DBig::from_str("2.35")?, AddOne));
assert_eq!(a.clone().with_precision(5), Exact(DBig::from_str("2.345")?));
# Ok::<(), dashu_base::ParseError>(())
```

## Unlimited precision

`Context::new(0)` (equivalently `with_precision(0)`) sets **unlimited**
precision: no cap on the significand, so `+`, `-`, and `*` are exact whenever
the true result is finitely representable in the base — e.g. `0.1 + 0.2` in
base 10 keeps full precision, and a product never loses a digit:

```rust
# use core::str::FromStr;
# use dashu_float::DBig;
let a = DBig::from_str("0.1")?;
let b = DBig::from_str("0.2")?;
let c = a + b;                   // exact: 0.3
assert_eq!(c, DBig::from_str("0.3")?);
assert_eq!(c.precision(), 0);    // still unlimited
# Ok::<(), dashu_base::ParseError>(())
```

Caveats:

- **Not every operation works at unlimited precision.** Division (`/`,
  `inv`) and the transcendentals (`exp`, `ln`, `sin`, …, roots) need a finite
  target to round to, and **panic** on an unlimited-precision operand — there
  is no rounding mode that can produce an infinite significand. `ulp()` /
  `ulp_lb()` likewise panic (there is no fixed unit to report).
- The common pattern is to hold a value at unlimited precision (e.g. a parsed
  constant) and call `with_precision(p)` before a lossy operation:

```rust
# use core::str::FromStr;
# use dashu_float::DBig;
let x = DBig::from_str("3.1415926535897932384626433832795028841971")?; // unlimited
let r = x.with_precision(20).unwrap();   // round to 20 significant digits
let y = r / DBig::from(2u8);             // division is now allowed
# Ok::<(), dashu_base::ParseError>(())
```

## Complex numbers: shared precision

`CBig` mirrors `FBig` but stores the real and imaginary parts over a **single
shared `Context`** — so there is exactly one precision and one rounding mode,
and the two parts structurally cannot disagree (the uniform-precision
invariant).

- `CBig::from_parts(re, im)` takes the **higher** of the two parts' contexts:

```rust
# use core::str::FromStr;
# use dashu::complex::CBig;
# use dashu::float::DBig;
let re = DBig::from_str("1.234567890123456789")?; // 19 digits
let im = DBig::from_str("2.0")?;                  // 2 digits
let z = CBig::from_parts(re, im);
assert_eq!(z.precision(), 19);                    // the larger context wins
# Ok::<(), dashu_base::ParseError>(())
```

- `CBig::from_parts_const(...)` takes an explicit `precision` argument instead
  of inferring.
- Arithmetic between `CBig` values keeps the shared context; operations
  against plain `FBig` parts follow the same "higher wins" rule, and a
  `CBig` result is correctly rounded per component (see
  [Rounding](./rounding.md)).
