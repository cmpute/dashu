# dashu-macros

Utility macros to create mathematical number literals. See [Docs.rs](https://docs.rs/dashu-macros/latest/dashu_macros/) for the full documentation.

# Features

- Support creating **big integers** with literals using `ubig!` and `ibig!`.
- Support creating **big floats** with literals using `fbig!` and `dbig!`.
- Support creating **big rationals** with literals using `rbig!`.
- Support creating **big complex numbers** with literals using `cbig!`.
- All macros can be used to create **const** numbers if they are small enough.

## Quick example

One macro per number domain — all compile-time, with zero precision loss:

```rust
use dashu_macros::{ubig, ibig, fbig, dbig, rbig, cbig, static_ubig};
use dashu_int::UBig;

let a = ubig!(0x5a4653ca_67376856_5b41f775_); // integer, any base
let b = ibig!(-0x10ff);                       // signed integer
let c = fbig!(0x1.ffffp1023);                 // binary float (hex float syntax)
let d = dbig!(3.1415926535897932384626);      // decimal float
let e = rbig!(22 / 7);                        // rational
let f = cbig!(3 + 4i);                        // complex

// const-capable when the value fits in a DoubleWord
const C: UBig = ubig!(0xffff_ffff);

// explicit radix, and static_ variants for large const values
let g = ubig!(dead_beef base 16);
static BIG: &UBig = static_ubig!(1234567890123456789012345678901234567890);

// the `_` escape for literals the tokenizer rejects (hex floats)
let _h = fbig!(_0x1.0p0);
```

## License

See the [top-level readme](../README.md).
