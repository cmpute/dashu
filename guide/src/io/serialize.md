```text
The layout for serialized numbers is protected by the semver. The change of the layout is considered as a break change and a new major version will be published.
```

# Conversion to Bytes

(use `to_le_bytes`, `to_be_bytes`, `from_le_bytes`, `from_be_bytes`)

# Serialization with `serde`

(Use serde for best platform compatibility and memory efficiency. Note that we support the `is_human_readable` option.)

# Serialization with `rkyv`

(Use rkyv for best speed.)

