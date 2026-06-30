# Serialization

```text
The layout for serialized numbers is protected by semver. A change to the layout is considered a breaking change and a new major version will be published.
```

dashu offers three layers of (de)serialization for its integer and float types, chosen by how portable or fast the format must be.

## Conversion to Bytes

`UBig` and `IBig` convert to and from explicit-endianness byte sequences via `to_le_bytes` / `to_be_bytes` and `from_le_bytes` / `from_be_bytes`. These are portable, layout-stable formats suitable for binary interchange.

```rust
use dashu_int::UBig;

let n = UBig::from(0x12345678u32);
let bytes = n.to_le_bytes();
assert_eq!(UBig::from_le_bytes(&bytes), n);
```

## Serialization with `serde`

With the `serde` feature enabled, every numeric type implements `Serialize` / `Deserialize`. The human-readable form (when `is_human_readable()` is true) is a string, for easy use with JSON/TOML; the compact binary form is used otherwise. Only the binary form's layout is semver-protected.

## Serialization with `rkyv`

With the `rkyv` feature enabled, zero-copy (de)serialization is available for the integer types — fastest for same-architecture scenarios, at the cost of a less portable layout.
