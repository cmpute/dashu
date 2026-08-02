Dashu 支持完整的类型转换集，包括任意精度类型之间的转换，以及任意精度类型与基本类型之间的转换。

请注意，`dashu` 中 `TryFrom` 实现的一个通用原则是：`TryFrom` 仅在转换无损失时才成功。转换过程中产生的任何精度丢失都会导致 `TryFrom` 返回 `Err`。

## 类型之间的转换

大多数情况下，你可以使用 `From`/`Into`/`TryFrom`/`TryInto` 在这些类型之间进行转换。当转换可能失败时，只会实现 `TryFrom` 和 `TryInto`。下表列出了使用这些 trait 在任意精度类型之间的转换，其中列为源类型，行为目标类型。

| 目标\源   | UBig | IBig    | FBig/DBig    | RBig        | CBig    |
|-----------|------|---------|--------------|-------------|---------|
| UBig      | \    | TryFrom | TryFrom      | TryFrom     | TryFrom |
| IBig      | From | \       | TryFrom      | TryFrom     | TryFrom |
| FBig/DBig | From | From    | \            | TryFrom[^a] | TryFrom |
| RBig      | From | From    | TryFrom[^a]  | \           | —       |
| CBig      | From | From    | From         | —           | \       |

> [^a]: 要使用 `RBig` 与 `FBig` 之间的转换，必须为 `dashu-ratio` crate 启用可选特性 `dashu-float`。

这些转换仅在转换是精确的（无损失的）且在范围内的前提下才会成功。例如，从浮点数到整数的转换会在浮点数为无穷时失败（返回 `Err(ConversionError::OutOfBounds)`），或在有小数部分时失败（返回 `Err(ConversionError::LossOfPrecision)`）。

尽管如此，还有一些用于**有损**转换的有用方法：

| 源\目标   | UBig              | IBig                                  | FBig/DBig         | RBig                         |
|-----------|-------------------|---------------------------------------|-------------------|------------------------------|
| UBig      | \                 | \                                     | \                 | \                            |
| IBig      | `.unsigned_abs()` | \                                     | \                 | \                            |
| FBig/DBig | \                 | `.to_int()`[^b]                       | ...[^c]           | `.simplest_from_float()`[^d] |
| RBig      | \                 | `.to_int()/.trunc()/.floor()/.ceil()` | `.to_float()`[^e] | \                            |

> - [^b] `FBig` 的 `.ceil()`、`.floor()` 和 `.trunc()` 方法不返回 `IBig`，因为当 `FBig` 非常大（指数很高）时，`IBig` 结果可能消耗大量内存，这通常不是理想的行为。
> - [^c] 请参阅下方的*FBig/DBig 的转换*章节。
> - [^d] 请参阅下方的*从浮点数转换到 RBig*章节了解更多方法。
> - [^e] 此方法需要为 `dashu-ratio` crate 启用 `dashu-float` 特性。

另一个有用的转换是 `UBig::as_ibig()`。由于 `UBig` 和 `IBig` 具有相同的内存布局，`UBig` 可以通过此方法直接用作 `IBig`。类似地，当你想将 `RBig` 实例用作 `dashu::rational::Relaxed` 时，`RBig::as_relaxed()` 也会很有帮助。

除了这些专为转换设计的方法之外，构造函数和析构函数也可以用于类型转换的目的，尤其是从复合类型到其组成部分的转换。请参阅[构造与析构](./construct.md#从组成部分构造)页面了解此方式。

## 大数与基本类型之间的转换

`dashu` crate 中的所有数值类型都支持与基本类型之间的相互转换。

从基本类型到大数的转换：

| 目标\源   | u*（如 u8） | i*（如 i8） | f*（如 f32） |
|-----------|--------------|--------------|---------------|
| UBig      | From         | TryFrom      | TryFrom       |
| IBig      | From         | From         | TryFrom       |
| FBig/DBig | From         | From         | TryFrom*      |
| RBig      | From         | From         | TryFrom       |
| CBig      | From         | From         | TryFrom*      |

> *：从 `f32`/`f64` 到 `FBig`/`CBig` 的转换**仅在基数 2 中定义**，因为当基数不是 2 的幂时，转换几乎总是有损的。要从 `f32`/`f64` 转换到其他基数的大浮点数（如基数为 10 的 `DBig`），可以先转换为基数 2，再使用 `.with_base()` 方法转换到其他基数。这样可以在转换过程中显式选择舍入模式。

从大数到基本类型的转换：

| 源\目标   | u*（如 u8） | i*（如 i8） | f*（如 f32）                      |
|-----------|--------------|--------------|------------------------------------|
| UBig      | TryInto      | TryInto      | TryInto/`.to_f*()`                 |
| IBig      | TryInto      | TryInto      | TryInto/`.to_f*()`                 |
| FBig/DBig | TryInto      | TryInto      | TryInto/`.to_f*()`                 |
| RBig      | TryInto      | TryInto      | TryInto/`.to_f*()`/`.to_f*_fast()` |
| CBig      | TryInto      | TryInto      | TryInto                            |

在上表中，`.to_f*()` 表示 `.to_f32()` 和 `.to_f64()`，类似地 `.to_f*_fast()` 表示 `.to_f32_fast()` 和 `.to_f64_fast()`。*fast* 方法不保证正确舍入，因此速度更快。推荐使用 `.to_f*()` 方法而非 `TryFrom`/`TryInto` trait，因为 `.to_f*()` 不会失败，并且还会返回转换过程中的舍入方向（即舍入误差的符号）。（`CBig` 没有 `.to_f*()` 方法——它唯一的浮点数转换路径是 `TryInto`，该路径仅限基数 2。）

与基本类型之间的相互转换也为 `dashu::float::Repr` 类型实现。尤其是 `.to_f32()` 和 `.to_f64()` 的实现遵循默认的 IEEE 舍入模式。


### FBig/DBig 的转换

涉及 `FBig`/`DBig` 的转换比整数类型更为丰富，因为浮点数携带三个独立的可调参数：**基数**、**精度**（有效数位的上限）和**舍入模式**。因此大多数转换都有两种形式——当没有信息丢失时使用不会失败的 `From`/`Into`，当要求精确性时使用可能失败的 `TryFrom`/`TryInto`。

### 转换到不同的基数 / 精度 / 舍入模式

基数、精度和舍入模式可以独立更改：

- `with_rounding::<NewR>()` 在不同的舍入模式下重新解释同一个值——底层表示不变，只有上下文的舍入字段改变，因此不会发生舍入。
- `with_precision(p)` 将尾数扩展或缩减到 `p` 位。扩展总是精确的（`Approximation::Exact`）；缩减按 `R` 进行舍入并返回携带舍入方向的 `Approximation::Inexact`。

```rust
use dashu::base::Approximation::*;
use dashu::float::DBig;
use dashu::float::round::Rounding::*;

let a = DBig::from_str("2.345")?;
assert_eq!(a.precision(), 4);
assert_eq!(a.clone().with_precision(3), Inexact(DBig::from_str("2.35")?, AddOne));
assert_eq!(a.clone().with_precision(5), Exact(DBig::from_str("2.345")?));
```

- `with_base::<NewB>()` 转换到不同的基数。结果精度会被选择为使尾数上限不超过原来的——即满足 $\mathrm{NewB}^{\,p'} \le B^{\,p}$ 的最大整数 $p'$。当一个基数是另一个基数的幂时，转换是精确的；否则按 `R` 进行舍入。`with_base_and_precision::<NewB>(p)` 允许你显式设置目标精度。

对于常见的二进制 ↔ 十进制切换，有两个快捷方法会自动为你选择舍入模式：`to_decimal()` 是 `with_rounding::<HalfAway>().with_base::<10>()`（产生 `DBig`），`to_binary()` 是 `with_rounding::<Zero>().with_base::<2>()`。

> 如果关联的上下文具有**无限精度**且转换无法无损完成，这些方法将 panic——请先设置精度。

### 转换到整数或基本浮点数

将 `UBig`/`IBig`（或任何基本整数）转换*到* `FBig` 时，精度从绝对值推断：结果精度等于该整数在基数 `B` 下的有效数位位数。

反向转换时，`TryFrom<FBig> for IBig`/`UBig` 仅在浮点数有限且恰好为整数值时才成功——无穷时返回 `ConversionError::OutOfBounds`，有小数部分时返回 `LossOfPrecision`。如需舍入感知的路径，请使用 `to_int()`，它总是成功并报告舍入方向：

```rust
use dashu::base::Approximation::*;
use dashu::float::DBig;
use dashu::float::round::Rounding::*;

assert_eq!(DBig::from_str("1234")?.to_int(), Exact(1234.into()));
assert_eq!(DBig::from_str("1.234")?.to_int(), Inexact(1.into(), NoOp));
```

转换为基本浮点数时，`to_f32()` / `to_f64()` 返回携带舍入方向的 `Rounded<f32>` / `Rounded<f64>`；它们不会失败（在格式的边界处，结果取决于舍入模式——见下文[范围边界](#范围边界上溢下溢与舍入模式)；无穷映射到无穷）。反向——`TryFrom<f32>`/`TryFrom<f64> for FBig`——**仅限基数 2**（在其他任何基数中几乎总是有损的）；要到达非二进制的 `FBig`，请先转换为基数 2，再调用 `with_base()`。NaN 会被以 `ConversionError::OutOfBounds` 拒绝。

### 范围边界：上溢、下溢与舍入模式

`to_f32()` / `to_f64()` 按上下文的舍入模式一路舍入到格式的边界，因此当真实值落在有限范围之外时，舍入模式决定*终点*：

- **上溢**（值超出 `f32::MAX` / `f64::MAX`）：向外的模式（朝向 `±∞`、远离零）达到 `±∞`；朝向零、朝向相反方向的无穷以及就近舍入则饱和到最大的*有限*值（`±MAX`）。
- **下溢**（值小于最小次正规数——f32 为 `2⁻¹⁴⁹`，f64 为 `2⁻¹⁰⁷⁴`）：向外的模式达到对应符号的最小次正规数；其余模式舍入到带符号的 `±0`。

```rust
use dashu::integer::IBig;
use dashu::float::{FBig, round::mode::{Zero, Up}};

// 3·2^127 超出 f32::MAX：朝向零饱和到 f32::MAX，朝向 +∞ 达到 +∞。
let big = FBig::<Zero, 2>::from_parts(IBig::from(3), 127);
assert_eq!(big.to_f32().value(), f32::MAX);
assert!(big.with_rounding::<Up>().to_f32().value().is_infinite());

// 2^-160 小于 f32 的最小次正规数：朝向 +∞ 达到它，朝向零得到 +0。
let tiny = FBig::<Zero, 2>::from_parts(IBig::from(1), -160);
assert_eq!(tiny.with_rounding::<Up>().to_f32().value(), f32::from_bits(1));
assert_eq!(tiny.to_f32().value(), 0.0);
```

相比之下，`TryFrom<FBig> for f32` / `f64` 是*可失败*的路径：对于**任何**不精确的转换（包括范围内的舍入）它都返回 `Err`——因此想拿到值时请优先使用 `to_f32()` / `to_f64()`。它的错误变体按*输入的绝对值*分类，而非按模式感知的结果分类：超出 `±MAX` 的值在**任何**舍入模式下都是 `Err(ConversionError::OutOfBounds)`（无论该模式会饱和到 `±MAX` 还是上溢到 `±∞`），而仅仅损失精度的范围内值则是 `Err(LossOfPrecision)`。因此 `Err(OutOfBounds)` 可靠地表示“超出有限范围”，与舍入模式无关：

```rust
use dashu::base::ConversionError;
use dashu::integer::IBig;
use dashu::float::{FBig, round::mode::{Zero, Up}};

let big = FBig::<Zero, 2>::from_parts(IBig::from(3), 127);
// 超出 f32::MAX 在任何模式下都越界——模式只决定 to_f32() 返回的终点（±MAX 还是 ±∞）。
assert_eq!(f32::try_from(big.clone().with_rounding::<Up>()), Err(ConversionError::OutOfBounds));
assert_eq!(f32::try_from(big), Err(ConversionError::OutOfBounds));
```

### 转换到 RBig

当为 `dashu-ratio` 启用可选的 `dashu-float` 特性时，`TryFrom<FBig> for RBig` 仅在浮点数恰好可以表示为有理数时才成功，而 `RBig::to_float()` 是另一方向的舍入感知路径。

要用一个*简单*有理数（容差内分子/分母最小的）近似浮点数，请使用 `simplest_from_f32` / `simplest_from_f64`，或在 `FBig`/`DBig` 上使用区间查询 `simplest_in`、`nearest_in`、`next_up` 和 `next_down`——这些方法将浮点数自身的舍入区间作为搜索范围。
