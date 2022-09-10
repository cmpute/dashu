# dashu-int

Arbitrary precision integer implementation as a part of the `dashu` library. 

> The majority of the code is based on the [ibig crate](https://github.com/tczajka/ibig-rs). The modification notice based on the the original `ibig` repo is included in the [NOTICE](./NOTICE) file.

## Features

- Support for both unsigned and signed big integers.
- Small integers are inlined on stack with specialized algorithms.
- Efficient implementation for basic arithmetic operations (`+`,`-`,`*`,`/`,`%`,`<<`,`>>`).
- Support other arithmetic operations including `pow`, `ilog`, `gcd`, `gcd_ext`.
- Bit operations for signed big integers follow the 2's complement rule.
- Efficient implementation for modular arithmetics (e.g. modular powering and inverse).
- Efficient integer parsing and printing with base 2~36.
- Developer friendly debug printing for big integers.
- Direct access to underlying machine word array.

## Examples

```rust
use dashu_int::{IBig, modular::ModuloRing, UBig};

let a = UBig::from(12345678u32);
let b = UBig::from(0x10ffu16);
let c = IBig::from_str_radix("-azz", 36).unwrap();
let d: UBig = "15033211231241234523452345345787".parse()?;
let e = 2u8 * &b + 1u8;
let f = a * b.pow(10);

assert_eq!(e, 0x21ff); // direct comparison with primitive integers
assert_eq!(c.to_string(), "-14255");
assert_eq!(
    f.in_radix(16).to_string(),
    "1589bda8effbfc495d8d73c83d8b27f94954e"
);
assert_eq!(
    format!("hello {:#x}", d % 0xabcd_1234_1341_3245_1345u128),
    "hello 0x1a7e7c487267d2658a93"
);

// modular arithmetics
let ring = ModuloRing::new(UBig::from(10000u32));
let x = ring.convert(12345);
let y = ring.convert(55443);
assert_eq!(format!("{}", x - y), "6902 (mod 10000)");
```

## Optional dependencies

* `std` (default): for `std::error::Error`.
* `num-traits` (default): integral traits.
* `rand` (default): random number generation.
* `serde`: serialization and deserialization.

## Performance

See the [built-in benchmark](../benchmark/).

## License

Licensed under either of

 * Apache License, Version 2.0
   ([LICENSE-APACHE](LICENSE-APACHE) or https://www.apache.org/licenses/LICENSE-2.0)
 * MIT license
   ([LICENSE-MIT](LICENSE-MIT) or https://opensource.org/licenses/MIT)

at your option.

The modification notice based on the the original `ibig` repo is included in the [NOTICE](./NOTICE) file.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.
