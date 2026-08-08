# dashu-base

Common trait definitions for the `dashu` mathematics crates. See [Docs.rs](https://docs.rs/dashu-base/latest/dashu_base/) for the full documentation.

## Quick example

The traits are implemented on primitive integers and on every dashu big-number
type alike, so generic numeric code works across the whole library:

```rust
use dashu_base::{DivRem, ExtendedGcd, Gcd};
use dashu_int::UBig;

// ...on primitive integers
assert_eq!(12u8.gcd(10u8), 2u8);
let (q, r) = 17u8.div_rem(5u8);
assert_eq!((q, r), (3u8, 2u8));

// ...and on every dashu big-number type (UBig, FBig, RBig, ...)
assert_eq!(UBig::from(12u8).gcd(UBig::from(10u8)), UBig::from(2u8));
let (q, r) = UBig::from(17u8).div_rem(UBig::from(5u8));
assert_eq!((q, r), (UBig::from(3u8), UBig::from(2u8)));

// ExtendedGcd also returns the Bézout coefficients
let (g, x, y) = 12u8.gcd_ext(10u8);
assert_eq!(g, 2u8);
assert_eq!(g as i8, 12 * x + 10 * y); // 2 = 12·1 + 10·(-1)
```

## License

See the [top-level readme](../README.md).
