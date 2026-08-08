Create an arbitrary precision complex number ([dashu_cmplx::CBig]) with base 2 rounding towards zero.

The literal is written either in the algebraic `a ± bi` notation or as a `re, im` pair. Coefficients
are **decimal by default** and may use the `0x` / `0b` / `0o` prefixes for other bases, matching
[ubig!] / [ibig!]. Decimal and octal coefficients are converted to base 2 (integer coefficients
convert exactly); hexadecimal and binary are parsed directly, supporting the same exponent syntax
as [fbig!]:

```rust
# use dashu_macros::cbig;
let z = cbig!(3+4i);               // 3 + 4i
let w = cbig!(3-4i);               // 3 - 4i
let r = cbig!(7);                  // purely real (7)
let im = cbig!(2i);                // purely imaginary (2i)
let p = cbig!(3, -4);              // pair form: 3 - 4i
assert_eq!(z, p + cbig!(8i));      // (3-4i) + 8i = 3+4i

// 0x / 0b / 0o prefixes select other bases
let h = cbig!(0x3 + 0x4i);         // 3 + 4i
let b = cbig!(0b11 + 0b100i);      // 3 + 4i
let o = cbig!(0o3 + 0o4i);         // 3 + 4i
assert_eq!(z, h);
assert_eq!(z, b);
assert_eq!(z, o);
```

The result's precision is the larger of the two coefficients' (the smaller-precision part is
widened exactly, so only the precision cap changes):

```rust
# use dashu_macros::cbig;
let z = cbig!(3.125 + 4i); // re has 4 decimal digits, im has 1
assert_eq!(z.precision(), 13); // decimal digits converted to bits
```

When both coefficients are small enough (their significands fit in a
[`DoubleWord`][dashu_int::DoubleWord]), the literal can be assigned to a constant — it expands to
`CBig::from_parts_const`:

```rust
# use dashu_macros::cbig;
use dashu_cmplx::CBig;

const Z: CBig = cbig!(3+4i);  // 3 + 4i
const P: CBig = cbig!(3, -4); // 3 - 4i (pair form)
```

Larger coefficients fall back to a runtime construction path and so cannot be used in `const`
position — use [`static_cbig!`] for those.
