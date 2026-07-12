`dashu` 的数值类型实现了 Rust 的标准格式化与解析 trait，并额外提供了一些 `dashu` 特有的 API，用于进制转换、按位展开和字节级序列化。本节涵盖以下内容：

- [解析](./parse.md) — 每个类型的 `FromStr` 和 `from_str_radix`，包括浮点数的指数形式。
- [打印](./print.md) — `Display`、`Debug`、`Binary`/`Octal`/`LowerHex`/`UpperHex` trait、`in_radix`，以及有理数的位置展开。
- [序列化](./serialize.md) — 字节序列、`serde` 和 `rkyv`。
- [互操作](./interop.md) — 对 `UBig` 原始表示的低级数位 / 字节 / word 访问。
