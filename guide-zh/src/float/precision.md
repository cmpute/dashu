## 什么是精度

一个 `FBig<R, B>` 值以 `有效数字 × 底数^指数` 的形式存储，并携带一个
`Context<R>`，其中保存两项设置：

- **精度** —— 有效数字最多可以保留的、以 `B` 为基数的有效数位位数；
- **舍入模式** —— 放不下的数位如何处理（见[舍入](./rounding.md)）。

`Context::new(p)` 创建精度为 `p` 的上下文；`p == 0` 表示**无限制精度**（完全没有上限）。用
`FBig::precision()` 读取当前上限：

```rust
use dashu_float::{Context, DBig, Repr};
use dashu_float::round::mode::HalfAway;

let context = Context::<HalfAway>::new(20);
let a = DBig::from_repr(Repr::new(1234.into(), -2), context);
assert_eq!(a.precision(), 20);
```

## 构建时精度来自哪里

大多数构造函数会根据输入自动推断精度，只有基于上下文的构造函数允许你显式指定：

| 构造函数 | 结果精度 |
|---|---|
| `from_parts(significand, exponent)` | 推断 —— `significand` 的精确有效数位位数（至少为 1） |
| `from_parts_const(sign, significand, exponent, min_precision)` | 推断，或当 `min_precision` 为 `Some` 时取 `max(推断值, min_precision)` |
| `from_repr(repr, context)` | 直接取自给定的 `Context` |
| `From<整数>` / `FromStr` | 从输入的有效数位推断 |
| 常量（`ONE`、`ZERO`、……）/ `from_repr_const` | 无限制（`Context::new(0)`） |
| `Context::new(p)` + `Context::max(lhs, rhs)` | 显式旋钮；`max` 取两个上下文中较高的精度 |

```rust
use core::str::FromStr;
use dashu_base::Sign;
use dashu_float::{Context, DBig};
use dashu_float::round::mode::Zero;

// from_parts：从有效数字推断精度
let a = DBig::from_parts(1234.into(), -2);   // 12.34
assert_eq!(a.precision(), 4);                // "1234" 有 4 位

// from_parts_const：max(推断值, min_precision)
let b = DBig::from_parts_const(Sign::Negative, 1234, -2, Some(6));
assert_eq!(b.precision(), 6);

// 解析时从有效数位推断精度
assert_eq!(DBig::from_str("2.3450")?.precision(), 5);

// 常量是无限制精度的
assert_eq!(DBig::ONE.precision(), 0);

// Context::max 取较高精度
let c1 = Context::<Zero>::new(10);
let c2 = Context::<Zero>::new(50);
assert_eq!(Context::max(c1, c2).precision(), 50);
# Ok::<(), dashu_base::ParseError>(())
```

## 运算符如何传播精度

二元算术（`+`、`-`、`*`、`/`）以 `Context::max(lhs.context, rhs.context)` 作为结果上下文——
**较高的精度胜出**，结果舍入到该精度。将 2 位与 30 位的值混合运算，会得到 30 位的结果：

```rust
# use core::str::FromStr;
# use dashu_float::DBig;
let lo = DBig::from_str("2.0")?;
let hi = DBig::from_str("1.23456789012345678901234567890")?; // 30 位
assert_eq!(lo.precision(), 2);
assert_eq!(hi.precision(), 30);

let sum = lo + hi;
assert_eq!(sum.precision(), 30); // 较高的精度胜出
# Ok::<(), dashu_base::ParseError>(())
```

值得了解的其它规则：

- 迭代器上的 `Sum` 是**正确舍入**的：加数被精确累加，只舍入一次，精度取所有加数精度的最大值，
  而不是用 `+` 逐项折叠（后者每步都舍入，可能丢失小加数）。`Product` 是普通折叠。
- 不精确的加法或减法可能携带一个**保护位**：结果有效数字在下一步运算将其舍回之前，可以短暂地
  容纳多达 `精度 + 1` 位。这是内部细节，通常你不会观察到。
- `with_precision(p)` 显式地将结果重新舍入到 `p` 位。扩展总是精确的；缩减按上下文的舍入模式进行，
  返回一个 `Rounded` 包装，报告舍入是 `Exact` 还是 `Inexact`（含方向）：

```rust
# use core::str::FromStr;
# use dashu_base::Approximation::*;
# use dashu_float::DBig;
use dashu_float::round::Rounding::*;

let a = DBig::from_str("2.345")?;
assert_eq!(a.precision(), 4);
assert_eq!(a.clone().with_precision(3), Inexact(DBig::from_str("2.35")?, AddOne));
assert_eq!(a.clone().with_precision(5), Exact(DBig::from_str("2.345")?));
# Ok::<(), dashu_base::ParseError>(())
```

## 无限制精度

`Context::new(0)`（等价地 `with_precision(0)`）设置**无限制**精度：有效数字没有上限，因此只要真实
结果在基数是有限可表示的，`+`、`-` 和 `*` 就是精确的——例如在十进制下 `0.1 + 0.2` 保持全部精度，
乘积也绝不丢位：

```rust
# use core::str::FromStr;
# use dashu_float::DBig;
let a = DBig::from_str("0.1")?;
let b = DBig::from_str("0.2")?;
let c = a + b;                   // 精确：0.3
assert_eq!(c, DBig::from_str("0.3")?);
assert_eq!(c.precision(), 0);    // 仍然无限制
# Ok::<(), dashu_base::ParseError>(())
```

注意事项：

- **并非所有运算都支持无限制精度。** 除法（`/`、`inv`）和超越函数（`exp`、`ln`、`sin`、……、开方）
  需要有限的舍入目标，对无限制精度的操作数会 **panic**——没有任何舍入模式能产生无限长的有效数字。
  `ulp()` / `ulp_lb()` 同样会 panic（没有可报告的固定单位）。
- 常见的做法是：在无限制精度下持有某个值（例如解析得到的常量），在可能损失的运算之前调用
  `with_precision(p)`：

```rust
# use core::str::FromStr;
# use dashu_float::DBig;
let x = DBig::from_str("3.1415926535897932384626433832795028841971")?; // 无限制
let r = x.with_precision(20).unwrap();   // 舍入到 20 位有效数字
let y = r / DBig::from(2u8);             // 现在允许除法了
# Ok::<(), dashu_base::ParseError>(())
```

## 复数：共享精度

`CBig` 镜像了 `FBig`，但将实部和虚部存放在**同一个共享 `Context`** 上——因此恰好只有一个精度和
一个舍入模式，两个部分在结构上就不可能不一致（精度一致不变量）。

- `CBig::from_parts(re, im)` 取两个部分上下文中**较高**的那个：

```rust
# use core::str::FromStr;
# use dashu::complex::CBig;
# use dashu::float::DBig;
let re = DBig::from_str("1.234567890123456789")?; // 19 位
let im = DBig::from_str("2.0")?;                  // 2 位
let z = CBig::from_parts(re, im);
assert_eq!(z.precision(), 19);                    // 较大的上下文胜出
# Ok::<(), dashu_base::ParseError>(())
```

- `CBig::from_parts_const(...)` 需要显式的 `precision` 参数，而非推断。
- `CBig` 之间的算术保持共享上下文；与普通 `FBig` 部分的运算遵循同样的“较高者胜出”规则，`CBig`
  结果的每个分量都是正确舍入的（见[舍入](./rounding.md)）。
