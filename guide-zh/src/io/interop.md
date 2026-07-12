除了标准格式化和解析 trait 之外，`dashu-int` 还暴露了对 `UBig` 原始表示的低级访问，以便与其他库互操作或构建自定义（反）序列化。

## 数位访问

`UBig::to_digits(base)` 以任意基数 `2..=Word::MAX` 返回该数的数位（最高位在前，存储为 `Word`），而 `UBig::from_digits(base, &digits)` 则从数位重建数值。这推广了 `in_radix`（后者仅限于基数 2–36 的字符串输出），支持任意基数和 word 大小的数位。

```rust
use dashu::integer::UBig;

let n = UBig::from(0x1234u16);
let digits = n.to_digits(16); // [1, 2, 3, 4], 最高位在前
assert_eq!(UBig::from_digits(16, &digits)?, n);
```

## 字节访问

`to_le_bytes` / `to_be_bytes` 和 `from_le_bytes` / `from_be_bytes` 提供可移植的、显式字节序的字节表示——参见[序列化](./serialize.md)。

## Word 访问

`UBig::from_words(&[w0, w1, …])` 从小端序 word 构建值，而 `.as_words()` 在不复制的情况下借用底层 word 切片。这是最接近内存中原始形式的方式。

```rust
use dashu::integer::{UBig, Word};

let n = UBig::from_words(&[3, 2, 1]); // 3 + 2·Word + 1·Word²
let words: &[Word] = n.as_words();
```

> `UBig` 的确切内存布局尚未稳定——不要跨版本依赖 word 布局。
