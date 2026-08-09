```text
The layout for serialized numbers is protected by semver. A change to the layout is considered a breaking change and a new major version will be published.
```

dashu offers three layers of (de)serialization for its integer and float types, chosen by how portable or fast the format must be.

## Conversion to Bytes

`UBig` and `IBig` convert to and from explicit-endianness byte sequences via `to_le_bytes` / `to_be_bytes` and `from_le_bytes` / `from_be_bytes`. These are portable, layout-stable formats suitable for binary interchange.

```rust
use dashu::integer::UBig;

let n = UBig::from(0x12345678u32);
let bytes = n.to_le_bytes();
assert_eq!(UBig::from_le_bytes(&bytes), n);
```

## Serialization with `serde`

With the `serde` feature enabled, every numeric type implements `Serialize` / `Deserialize`. The human-readable form (when `is_human_readable()` is true) is a string, for easy use with JSON/TOML; the compact binary form is used otherwise. Only the binary form's layout is semver-protected.

## Serialization with `rkyv`

With the `rkyv` feature enabled (rkyv **0.8**; use `rkyv_v07` for rkyv **0.7**), zero-copy
(de)serialization is available for the integer, rational, float, and complex types. The big integers
archive as their **native word representation** (`ArchivedVec<Word>`, plus a sign flag for `IBig`),
so `rkyv::archived_root` yields the words in place with no byte conversion on any path — the fastest
possible same-architecture encoding, at the cost of a layout that depends on the target's `Word`
width and endianness. In particular:

- `rkyv_v07` and `rkyv_v08` can be enabled **together** (their derive-generated types are namespaced
  through crate aliases, so they don't collide); the unversioned `rkyv` feature currently selects
  0.8. rkyv 0.8 requires Rust ≥ 1.81 (see the [MSRV policy](../faq.md#msrv-and-feature-policy)) and
  is excluded from the 1.68 MSRV build; its archives store multi-byte words little-endian by default.

- An archive is **not portable across 32/64-bit targets** (the `Word` size differs) or across machines with different endianness.
- rkyv itself does not guarantee archive compatibility across rkyv versions, and the `size_16/32/64` offset-width feature must match on both ends.

For a stable, portable encoding, prefer the byte layers above: either the `to_le_bytes`/`to_be_bytes` conversions (explicit, layout-stable) or `serde`'s binary form. Those are the right choice for data that must outlive a single machine or rkyv version; `rkyv` is the right choice when reading the archived data in place (e.g. memory-mapped files) matters more than portability.

## Serializing the cached wrappers

`CachedFBig` / `CachedCBig` do not implement the serialization traits (serde, rkyv, or any other
third-party trait) — the cached types intentionally mirror only the value API. Convert to the plain
value first:

```rust,ignore
let f = cached.as_fbig();       // or the owning `into_fbig()`
serialize(&f);                  // serde `to_string` / rkyv `to_bytes`, etc.
```

(`CachedCBig` has the matching `as_cbig()` / `into_cbig()`.)
