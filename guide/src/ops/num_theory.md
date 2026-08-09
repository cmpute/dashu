`dashu-int` provides greatest-common-divisor and modular-arithmetic primitives.

## Greatest common divisor

The `Gcd` trait (from `dashu-base`) gives `gcd`, and `ExtendedGcd` gives `gcd_ext`, which returns `(gcd, x, y)` with $a\cdot x + b\cdot y = \gcd(a,b)$.

```rust
use dashu::base::Gcd;
use dashu::integer::UBig;

let a = UBig::from(12u8);
let b = UBig::from(8u8);
assert_eq!((&a).gcd(&b), UBig::from(4u8));
```

## Exact division

The `DivExact` / `DivExactAssign` traits (re-exported through `dashu-base` from `num-modular`, with the empty precomputation `()`) compute an exact quotient: `div_exact` consumes the dividend and returns `Some(self / rhs)` when `rhs` divides exactly, `None` otherwise; `div_exact_assign` replaces the dividend in place, returning `true` on success (leaving it unchanged otherwise). Both take the precomputation as an extra `&()` argument. On `UBig`, dividing by a single-word factor uses Hensel (2-adic) division and runs in place with no scratch allocation.

```rust
use dashu::base::{DivExact, DivExactAssign};
use dashu::integer::UBig;

let a = UBig::from(10u32).pow(8) * 7u32; // 700000000
assert_eq!(a.clone().div_exact(UBig::from(10u32).pow(8), &()), Some(UBig::from(7u32)));
assert_eq!(a.div_exact(UBig::from(3u32), &()), None); // 3 doesn't divide

// In-place form
let mut b = UBig::from(10u32).pow(8) * 7u32;
assert!(b.div_exact_assign(UBig::from(10u32).pow(8), &()));
assert_eq!(b, UBig::from(7u32));
```

## Divisibility and factor removal

`is_multiple_of(&rhs)` tests whether `rhs` divides the value exactly — a faster alternative to
`% rhs == 0`. To strip all factors of a number, use `UBig::remove(&mut self, factor)`: it divides
out every occurrence of `factor`, returning the exponent (so `self = factor^k · rest` yields
`Some(k)`), or `None` when `factor` is 0, 1, or `self` is 0. `remove_word` is the single-word
specialization.

```rust
use dashu::integer::UBig;

let mut a = UBig::from(8u32) * 3u32; // 24 = 2³ · 3
assert_eq!(a.remove(&UBig::from(2u32)), Some(3));
assert_eq!(a, UBig::from(3u32));
assert!(UBig::from(10u32).is_multiple_of(&UBig::from(5u32)));
```

## Modular arithmetic

For repeated operations against a fixed modulus, precompute a `ConstDivisor` and reduce values into `Reduced`. Addition, subtraction, multiplication, exponentiation, and inversion then run against the precomputed modulus, and the result prints in `(mod N)` form.

```rust
use dashu::integer::{UBig, fast_div::ConstDivisor};

let ring = ConstDivisor::new(UBig::from(10000u32));
let x = ring.reduce(12345);
let y = ring.reduce(55443);
assert_eq!(format!("{}", x - y), "6902 (mod 10000)");
```

## Montgomery reducer

For **odd moduli**, `MontgomeryRepr` offers [Montgomery-form] modular arithmetic — an alternative to the Barrett-style `ConstDivisor`/`Reduced` above. A modular multiplication is an ordinary multiplication followed by a Montgomery reduction (REDC) instead of a division, so it is faster than Barrett whenever the REDC is cheaper than the division.

Montgomery multiplication, squaring, and exponentiation beat `Reduced` across roughly the 256–4096-bit range; beyond ~8 kbits the two are comparable. **For inverse-heavy workloads, prefer `Reduced`** — a Montgomery inverse must exit Montgomery form, run the extended GCD, and re-enter, whereas `Reduced` inverts directly.

```rust
use dashu::integer::{UBig, monty::MontgomeryRepr};

// A Mersenne prime (odd).
let p = UBig::from(2u8).pow(607) - UBig::ONE;
let ring = MontgomeryRepr::new(p.clone());

// reduce values into Montgomery form, then multiply / square / pow modularly
let a = ring.reduce(123);
assert_eq!(a.pow(&(p - UBig::ONE)), ring.reduce(1)); // Fermat: a^(p-1) = 1 (mod p)
```

`MontgomeryRepr::new(m)` requires `m` to be odd; `ring.reduce(x)` lifts `x` into Montgomery form, and the resulting `Montgomery` values support `+`, `-`, `*`, `.sqr()`, `.pow(&exp)`, and `.inv()`.

[Montgomery-form]: https://en.wikipedia.org/wiki/Montgomery_modular_multiplication

## Diophantine approximation

Rational approximation of reals — the simplest rational within a tolerance, continued fractions — lives on `RBig`; see [Conversion](../convert.md#conversion-to-rbig) for `simplest_in` / `nearest_in`.
