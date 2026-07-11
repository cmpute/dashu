Create an arbitrary precision complex number ([dashu_cmplx::CBig]) with base 2 rounding towards zero.

The literal is written either in the algebraic `a ± bi` notation or as a `re, im` pair. Each
coefficient follows the same grammar as [fbig!] — binary or hexadecimal digits, an optional `Bxx`
binary exponent, and `_` digit separators:

```rust
# use dashu_macros::cbig;
let z = cbig!(11+100i);          // 3 + 4i   (11₂ = 3, 100₂ = 4)
let w = cbig!(11-100i);          // 3 - 4i
let r = cbig!(111);              // purely real (7)
let im = cbig!(10i);             // purely imaginary (2i)
let p = cbig!(11, -100);         // pair form: 3 - 4i
assert_eq!(z, p + cbig!(1000i)); // (3-4i) + 8i = 3+4i

// hexadecimal coefficients and binary exponents work as in `fbig!`
let h = cbig!(0x1p4 + 0x1p3i);   // 16 + 8i
```

The result's precision is the larger of the two coefficients' (the smaller-precision part is
widened exactly, so only the precision cap changes):

```rust
# use dashu_macros::cbig;
let z = cbig!(11.001 + 100i); // re has 5 binary digits, im has 3
assert_eq!(z.precision(), 5);
```

When both coefficients are small enough (their significands fit in a
[`DoubleWord`][dashu_int::DoubleWord]), the literal can be assigned to a constant — it expands to
`CBig::from_parts_const`:

```rust
# use dashu_macros::cbig;
use dashu_cmplx::CBig;

const Z: CBig = cbig!(11+100i);  // 3 + 4i
const P: CBig = cbig!(11, -100); // 3 - 4i (pair form)
```

Larger coefficients fall back to a runtime construction path and so cannot be used in `const`
position — use [`static_cbig!`] for those.
