# dashu-int

Arbitrary precision integer implementation for the dashu library. 

The majority of the code is based on the [ibig crate](https://github.com/tczajka/ibig-rs). The modification notice based on the the original `ibig` repo is included in the [NOTICE](./NOTICE) file.

## Examples

```rust
use dashu_int::{ibig, modular::ModuloRing, ubig, UBig};

let a = ubig!(12345678);
let b = ubig!(0x10ff);
let c = ibig!(-azz base 36);
let d: UBig = "15033211231241234523452345345787".parse()?;
let e = 2u8 * &b + 1u8;
let f = a * b.pow(10);

assert_eq!(e, ubig!(0x21ff));
assert_eq!(c.to_string(), "-14255");
assert_eq!(
    f.in_radix(16).to_string(),
    "1589bda8effbfc495d8d73c83d8b27f94954e"
);
assert_eq!(
    format!("hello {:#x}", d % ubig!(0xabcd_1234_1341_3245_1345)),
    "hello 0x1a7e7c487267d2658a93"
);

let ring = ModuloRing::new(ubig!(10000));
let x = ring.convert(12345);
let y = ring.convert(55443);
assert_eq!(format!("{}", x - y), "6902 (mod 10000)");
```

## Optional dependencies

* `std` (default): for `std::error::Error`.
* `num-traits` (default): integral traits.
* `rand` (default): random number generation.
* `serde`: serialization and deserialization.

## Benchmarks

[Benchmarks](https://github.com/tczajka/bigint-benchmark-rs) contains a quick benchmark of
Rust big integer libraries.

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
