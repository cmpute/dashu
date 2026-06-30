# Number Theoretic

`dashu-int` provides greatest-common-divisor and modular-arithmetic primitives.

## Greatest common divisor

The `Gcd` trait (from `dashu-base`) gives `gcd`, and `ExtendedGcd` gives `gcd_ext`, which returns `(gcd, x, y)` with $a\cdot x + b\cdot y = \gcd(a,b)$.

```rust
use dashu_base::Gcd;
use dashu_int::UBig;

let a = UBig::from(12u8);
let b = UBig::from(8u8);
assert_eq!((&a).gcd(&b), UBig::from(4u8));
```

## Modular arithmetic

For repeated operations against a fixed modulus, precompute a `ConstDivisor` and reduce values into `Reduced`. Addition, subtraction, multiplication, exponentiation, and inversion then run against the precomputed modulus, and the result prints in `(mod N)` form.

```rust
use dashu_int::{UBig, fast_div::ConstDivisor};

let ring = ConstDivisor::new(UBig::from(10000u32));
let x = ring.reduce(12345);
let y = ring.reduce(55443);
assert_eq!(format!("{}", x - y), "6902 (mod 10000)");
```

## Diophantine approximation

Rational approximation of reals — the simplest rational within a tolerance, continued fractions — lives on `RBig`; see [Conversion](../convert.md#conversion-to-rbig) for `simplest_in` / `nearest_in`.
