Create an arbitrary precision float number ([dashu_float::FBig]) with base 2 rounding towards zero.

This macro only accepts binary or hexadecimal literals. It doesn't allow decimal literals because
the conversion is not always lossless. Therefore if you want to create an [FBig][dashu_float::FBig]
instance with decimal literals, use the [dbig!] macro and then change the radix with
[with_base][dashu_float::FBig::with_base].

```rust
# use dashu_macros::fbig;
let a = fbig!(11.001); // digits in base 2, equal to 3.125 in decimal
let b = fbig!(1.101B-3); // exponent in base 2 can be specified using `Bxx`
let c = fbig!(-0x1a7f); // digits in base 16
let d = fbig!(0x03.efp-2); // equal to 0.9833984375 in decimal

// underscores can be used to separate digits
let e = fbig!(0xa54653ca_67376856_5b41f775.f00c1782_d6947d55p-33);

// Due to the limitation of Rust literal syntax, the hexadecimal literal
// with floating point requires an underscore prefix if the first digit is
// not a decimal digit.
let f = fbig!(-_0xae.1f);
let g = fbig!(-0xae1fp-8);
assert_eq!(f, g);
let h = fbig!(-0x12._34);
let i = fbig!(-_0x12.34);
assert_eq!(h, i);
```

The generated float has precision determined by length of digits in the input literal.

```rust
# use dashu_macros::fbig;
let a = fbig!(11.001); // 5 binary digits
assert_eq!(a.precision(), 5);

let b = fbig!(0x0003.ef00p-2); // 8 hexadecimal digits = 32 binary digits
assert_eq!(b.precision(), 32);
assert_eq!(b.digits(), 10); // 0x3ef only has 10 effective bits
```

For numbers that are small enough (significand fits in a [u32]),
the literal can be assigned to a constant.

```rust
# use dashu_macros::fbig;
use dashu_float::FBig;

const A: FBig = fbig!(-1001.10);
const B: FBig = fbig!(0x123);
const C: FBig = fbig!(-0xffff_ffffp-127);
```