`UBig` 和 `IBig` 支持按位运算符 `&`（与）、`|`（或）、`^`（异或）和 `!`（取反）。在 `UBig` 上，`!` 是*无限宽度*的补码——最高置位以上的每一位都被视为 `1`，因此 `!n` 通常是一个非常大的数。在 `IBig` 上，`!` 遵循二进制补码规则。

```rust
use dashu::integer::UBig;

let a = UBig::from(0b1100u8);
let b = UBig::from(0b1010u8);
assert_eq!(format!("{:b}", &a & &b), "1000");
assert_eq!(format!("{:b}", &a | &b), "1110");
```

## 位测试与长度

`BitTest` trait（来自 `dashu-base`）用于测试和度量单个位：`.bit(n)` 返回第 `n` 位，`.bit_len()` 返回最高置位位置加一。`set_bit(n)` / `clear_bit(n)` 就地修改 `UBig`，`trailing_zeros()` 计算低位零位的个数。

## 移位

`<<` 和 `>>` 按 `usize` 值移位。左移使数值增长；右移使数值缩小，等价于向下取整除以 2 的幂。

## 将 `UBig` 用作位向量

由于 `UBig` 具有无限宽度，它天然适合用作任意大的位集合：用 `set_bit(i)` 设置第 `i` 位，用 `bit(i)` 测试它，用 `bit_len()` 读取范围。

```rust
use dashu::base::BitTest;
use dashu::integer::UBig;

let mut bits = UBig::ZERO;
bits.set_bit(0);
bits.set_bit(100);
assert!(bits.bit(0) && bits.bit(100));
assert!(!bits.bit(1));
assert_eq!(bits.bit_len(), 101);
```
