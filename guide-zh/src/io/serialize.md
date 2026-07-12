```text
序列化数值的布局受 semver 保护。对布局的更改被视为破坏性变更，将发布新的主版本。
```

`dashu` 为其整数和浮点数类型提供三个层次的（反）序列化方案，可根据所需的可移植性或速度来选择。

## 转换为字节

`UBig` 和 `IBig` 可以通过 `to_le_bytes` / `to_be_bytes` 和 `from_le_bytes` / `from_be_bytes` 与显式字节序的字节序列相互转换。这些是可移植的、布局稳定的格式，适合用于二进制交换。

```rust
use dashu::integer::UBig;

let n = UBig::from(0x12345678u32);
let bytes = n.to_le_bytes();
assert_eq!(UBig::from_le_bytes(&bytes), n);
```

## 使用 `serde` 序列化

启用 `serde` 特性后，每个数值类型都实现 `Serialize` / `Deserialize`。人类可读形式（当 `is_human_readable()` 为 `true` 时）为字符串，便于在 JSON/TOML 中使用；否则使用紧凑的二进制形式。仅二进制形式的布局受 semver 保护。

## 使用 `rkyv` 序列化

启用 `rkyv` 特性后，整数类型支持零拷贝（反）序列化——在同架构场景下速度最快，代价是布局的可移植性较低。
