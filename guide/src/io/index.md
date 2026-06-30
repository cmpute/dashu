# Input and Output

dashu's numeric types participate in Rust's standard formatting and parsing traits, plus a few dashu-specific APIs for radix conversion, positional expansion, and byte-level serialization. This section covers:

- [Parsing](./parse.md) — `FromStr` and `from_str_radix` for every type, including the float exponent forms.
- [Printing](./print.md) — `Display`, `Debug`, the `Binary`/`Octal`/`LowerHex`/`UpperHex` traits, `in_radix`, and the rational positional expansion.
- [Serialization](./serialize.md) — byte sequences, `serde`, and `rkyv`.
- [Interoperability](./interop.md) — low-level digit / byte / word access to a `UBig`'s raw representation.
