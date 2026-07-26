每个数值类型都实现了 `FromStr`，因此可以通过 `"...".parse()?` 或 `T::from_str(...)` 来构建值。所有数值字面量中都允许使用下划线分隔符。

## 解析整数

`UBig::from_str` / `IBig::from_str` 接受一个可选的符号后跟十进制数位。对于其他基数，请使用 `from_str_radix(s, radix)`（基数 2–36）；它会独立于 `radix` 参数自动识别 `0x`/`0o`/`0b` 前缀。

```rust
use dashu::integer::{UBig, IBig};
use core::str::FromStr;

assert_eq!(UBig::from_str("12345")?, UBig::from(12345u16));
assert_eq!(IBig::from_str_radix("-1aff", 16)?, IBig::from(-0x1aff));
```

## 解析浮点数

`FBig`/`DBig` 的 `FromStr` 以该值的原生基数读取尾数，指数采用以下形式之一：

| 形式 | 含义 | 基数 |
|------|------|------|
| `aaa` / `aaa.` / `aaa.bbb` | 定点数 | 任意 |
| `aaa.bbb@cc` | 尾数 × 基数^cc | 任意 |
| `aaa.bbbEcc` / `aaa.bbbecc` | 尾数 × 10^cc | 10 |
| `0xaaa.bbbPcc` | 十六进制尾数 × 2^cc | 2 |

精度根据所给出的有效数位数推断。字符串字面量 `inf`/`NaN` **不被**接受——请使用 `INFINITY` 常量来构造无穷值。

```rust
use dashu::float::DBig;
use core::str::FromStr;

assert_eq!(format!("{:e}", DBig::from_str("6.022e23")?), "6.022e23");
assert_eq!(DBig::from_str("-0.0123456789")?.to_string(), "-0.0123456789");
```

## 解析有理数

`RBig::from_str` 接受 `numerator/denominator` 格式，或仅一个分子（分母默认为 1）。`from_str_radix` 以给定基数解析分子和分母两部分；`0x`/`0o`/`0b` 前缀在两者之间必须保持一致。

```rust
use dashu::rational::RBig;
use core::str::FromStr;

assert_eq!(RBig::from_str("22/7")?.to_string(), "22/7");
```

对于定点展开字面量，`RBig::from_str_expanded` / `Relaxed::from_str_expanded` 可解析任意 2–36 进制。它们接受定点数（`1.5`、`-.25`）、科学记数法，以及将循环节用括号括起的**循环**记法（`0.1(6)` = 1/6，`0.(3)` = 1/3）。科学记数法的标记符在十进制下为 `e`/`E`，在其他进制下为 `@`（因为当进制达到 15 时，`e`/`E` 本身就是数位）。它是 `in_expanded(radix)` 的精确逆运算（参见[打印](./print.md)）：每个有理数都能通过 `{:#}` 往返还原，而有限展开能通过 `{:.N}` 往返还原。`from_str_decimal` 是十进制的别名。

```rust
use dashu::rational::RBig;
use core::str::FromStr;

let x = RBig::from_str_expanded("0.1(6)", 10)?; // 1/6
assert_eq!(x, RBig::from_str("1/6")?);
// 任意进制：二进制 0.(01) = 1/3
assert_eq!(RBig::from_str_expanded("0.(01)", 2)?, RBig::from_str("1/3")?);
// 每个有理数都能通过循环节打印格式往返还原
assert_eq!(RBig::from_str_expanded(&format!("{:#}", x.in_expanded(10)), 10)?, x);
```

## 解析复数

`CBig::FromStr` 接受与 `Display` 输出相同的代数 $a+bi$ 文法：一个可选的实部项加上一个可选的带符号虚部项（至少需要一项）；系数为 1 时可以省略（`i`、`-i`）。MPC 风格的括号形式 `(re im)` **不被**接受。

```rust
use dashu::complex::CBig;
use dashu::float::round::mode::HalfAway;
use core::str::FromStr;

type C = CBig<HalfAway, 10>;
assert_eq!(C::from_str("1+2i")?.to_string(), "1+2i");
assert_eq!(C::from_str("-i")?.to_string(), "-i");
```
