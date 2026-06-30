# Interoperability

Besides the standard formatting and parsing traits, `dashu-int` exposes lower-level access to a `UBig`'s raw representation, for interoperating with other libraries or building custom (de)serialization.

## Digit access

`UBig::to_digits(base)` returns the number's digits in any base `2..=Word::MAX` (most-significant first, stored as `Word`), and `UBig::from_digits(base, &digits)` reconstructs it. This generalizes `in_radix` (which is limited to base 2–36 for string output) to arbitrary bases and word-sized digits.

```rust
use dashu_int::UBig;

let n = UBig::from(0x1234u16);
let digits = n.to_digits(16); // [1, 2, 3, 4], most-significant first
assert_eq!(UBig::from_digits(16, &digits)?, n);
```

## Byte access

`to_le_bytes` / `to_be_bytes` and `from_le_bytes` / `from_be_bytes` give a portable, explicit-endianness byte representation — see [Serialization](./serialize.md).

## Word access

`UBig::from_words(&[w0, w1, …])` builds a value from little-endian words, and `.as_words()` borrows the underlying word slice without copying. This is the closest to the raw in-memory form.

```rust
use dashu_int::{UBig, Word};

let n = UBig::from_words(&[3, 2, 1]); // 3 + 2·Word + 1·Word²
let words: &[Word] = n.as_words();
```

> The exact in-memory layout of a `UBig` is not yet stabilized — don't rely on the word layout across versions.
