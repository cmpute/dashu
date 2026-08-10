## 为什么需要舍入

精度限制了有效数字最多能保留多少位。当运算产生的数位超过上限时——这通常是常态而非例外：
`1/3`、`sqrt(2)`、`exp(1)`，乃至简单地把两个指数不同的数相加——多余的数位必须被丢弃，而*如何*
丢弃它们，就是舍入模式决定的事。

## 舍入模式是一个类型参数

`FBig<R, B>` 把舍入模式作为类型参数 `R: Round` 携带。`R` 是一个零尺寸的标记类型，运行时零开销——
但它是类型的一部分，因此两个舍入模式不同的 `FBig` 是不同类型，不能混合参与同一个运算（设计上即
编译错误）。

dashu 在 `dashu_float::round::mode` 中提供了六种模式：

| 模式 | 行为 | 备注 |
|------|------|------|
| `Zero` | 向零舍入（截断） | `Real`（`FBig`）的默认模式 |
| `HalfEven` | 最近舍入，平局时取偶 | |
| `HalfAway` | 最近舍入，平局时远离零 | `Decimal`（`DBig`）的默认模式 |
| `Down` | 向 −∞ 舍入 | 定向 |
| `Up` | 向 +∞ 舍入 | 定向 |
| `Away` | 远离零舍入 | |

因此 `dashu::Real` 是 `FBig<Zero, 2>`，`dashu::Decimal` 是 `FBig<HalfAway, 10>`（即 `DBig`）。

## 切换模式

`with_rounding::<NewR>()` 更改类型参数；值本身不变，精度也不变：

```rust
use dashu_float::{DBig, FBig};
use dashu_float::round::mode::Zero;

let a = DBig::from_parts(1234.into(), -2); // FBig<HalfAway, 10>
let b = a.with_rounding::<Zero>();         // FBig<Zero, 10>，值相同
assert_eq!(a, b);
```

## 报告舍入方向

在上下文层，不精确的运算返回一个 `Rounded<T>` 包装——携带 `Rounding` 标志的 `Approximation`。
`Rounding` 有三个变体，`NoOp` / `AddOne` / `SubOne`，描述对截断后有效数字所做的调整（而非误差的
方向）。便捷层（运算符、`.exp()`、……）会解开这个包装并返回普通值，因此只有在你自行调用上下文层时
才能观察到方向。

## 精度与舍入是耦合的

两项设置存放在同一个 `Context<R>` 中：`Context::new(p)` 固定精度，`R` 类型参数固定舍入。
`with_precision(p)` 使用*当前*舍入模式重新舍入；`with_rounding` 则只改模式、不动值。要同时控制两者，
可以链式调用这两个方法，或构造一个新的上下文。

## 每个运算都是正确舍入的

dashu-float（以及 dashu-cmplx）保证每个运算都返回**正确舍入**的结果：即当前舍入模式下、最接近
无限精确真实结果的唯一可表示值——绝不是宽松的容差，也绝不差 1-ulp。这一保证由与 MPFR/MPC 的 fuzz
差分测试强制：这些测试断言在每种舍入模式下结果都逐位精确一致。

支持这一保证的舍入模式，是那些实现了 `ErrorBounds` 的模式——`ErrorBounds` 给出真实结果与某个计算
近似值之间误差的上界。float 的 Ziv 层要求超越函数满足 `R: ErrorBounds`，而六种模式全部实现了它。

## 如何实现正确舍入：Ziv 重试循环

超越函数（`exp`、`ln`、`sin`、`cos`、`sqrt`、……）无法在有限时间内精确计算，因此 dashu 使用
**Ziv 重试循环**：

1. 以 `精度 + guard` 位计算该运算。
2. 通过 Ball 算术在结果周围计算一个严格的误差界。
3. 将该区间舍入到 `精度` 位。如果舍入结果无歧义——误差区间没有跨越舍入边界——则认证并返回。
4. 否则将 guard 加倍并重试。

循环几乎总是在第一次重试时收敛；达到重试上限表明误差半径估计有 bug，此时上下文层会显式报告
`FpError::ZivRetryLimitExceeded`，而不是返回一个可能差 1-ulp 的值。

float 的 Ziv 驱动（`float/src/ziv.rs`）负责认证实数超越函数；复数的 Ziv 驱动（`complex/src/ziv.rs`）
将其包装起来，对同一个 preimage 同时认证 `CBig` 结果的两个部分，因此复数的超越函数按分量正确舍入。
