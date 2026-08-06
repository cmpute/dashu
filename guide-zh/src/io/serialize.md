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

启用 `rkyv` 特性后，整数、有理数、浮点和复数类型支持零拷贝（反）序列化。大整数以**原生 word 表示**（`ArchivedVec<Word>`，`IBig` 附带符号位）归档，因此 `rkyv::archived_root` 能原地得到大整数的 word，任何路径都没有字节转换——这是同架构下最快的编码方式，代价是布局依赖于目标机的 `Word` 宽度和字节序。具体而言：

- 归档**不能跨 32/64 位目标**（`Word` 宽度不同），也不能跨字节序不同的机器移植。
- rkyv 本身不保证跨版本归档兼容，且 `size_16/32/64`（偏移宽度）feature 在两端必须一致。

需要稳定、可移植的编码时，请使用上面的字节层：`to_le_bytes`/`to_be_bytes` 转换（显式、布局稳定）或 `serde` 的二进制形式。它们适合需要跨机器或跨 rkyv 版本保存的数据；`rkyv` 适合「原地读取归档数据」（如内存映射文件）比可移植性更重要的场景。

## 序列化缓存包装类型

`CachedFBig` / `CachedCBig` 不实现序列化 trait（serde、rkyv 或任何其他第三方 trait）——缓存类型有意只镜像数值 API。请先转换为普通值再序列化：

```rust,ignore
let f = cached.as_fbig();       // 或拥有式 `into_fbig()`
serialize(&f);                  // 例如 serde 的 `to_string` / rkyv 的 `to_bytes`
```

（`CachedCBig` 有对应的 `as_cbig()` / `into_cbig()`。）
