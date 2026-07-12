所有数值类型都实现了标准算术运算符，同时支持拥有值与借用值作为操作数。除法和余数的行为因类型而异。

## 整数算术

`UBig` 和 `IBig` 支持 `+`、`-`、`*`、`/` 和 `%`。整数除法向零取整，余数取被除数的符号（C/Rust 约定）。若需要欧几里得除法（非负余数），请使用 `dashu-base` 中的 `DivRemEuclid` / `RemEuclid` trait；`DivRem` 则同时返回商和余数。

```rust
use dashu::integer::IBig;

let b = IBig::from(-0x10ff);
let e = 2 * &b - 1; // 与基本类型自然混用
assert_eq!(e, IBig::from(-0x21ff));
```

## 浮点算术

`FBig`/`DBig` 支持**相同基数和舍入模式**的值之间的 `+`、`-`、`*`、`/`（混合基数是设计上的编译错误）。结果的精度为 `max(lhs.precision, rhs.precision)`，每次运算通过[指数与对数](./exp_log.md)中所述的两层 API 报告其不精确性。无穷值是终态：`1/0` 和 `ln(0)` 产生 `±∞`，但将无穷值再次送入算术运算则是错误（`FpError::InfiniteInput`）。

## 有理数算术

`RBig` 支持 `+`、`-`、`*`、`/`。除以零会 panic。`Relaxed` 执行相同的运算但不自动约分到最简形式（在连续运算链中更快）；需要时调用 `canonicalize()` 进行约分。

## 复数算术

`CBig` 支持域运算 `+`、`-`、`*`、`/`，以及 `sqr`（平方）和 `inv`（乘法逆）。还可以用实数 `FBig` 与 `CBig` 进行混合类型的乘法和除法。乘法和除法使用 Smith 方法并配合保护位和重新舍入，提供与 `dashu-float` 超越函数相同的近似正确舍入保证。

```rust
use dashu::complex::CBig;
use dashu::float::{FBig, round::mode::HalfAway};

type C = CBig<HalfAway, 10>;
let z = C::from_parts(FBig::from(3), FBig::from(4));
let sum = &z + &C::I; // (3+4i) + i = 3+5i
assert_eq!(sum.im().significand(), &5.into());
```

## 聚合：`Sum` 和 `Product`

所有数值类型都实现了 `core::iter::Sum` 和 `Product`，可迭代 `T` 或 `&T`。对于 `FBig`/`DBig`，`Sum` 是正确舍入的——加数被精确累加，只舍入一次（MPFR `mpfr_sum`），而非用 `+` 逐项折叠（后者每步都舍入，可能丢失小加数）；结果的精度为所有加数精度的最大值。`Product` 在所有类型上均为普通折叠。对于 `FBig`/`DBig`/`CBig`，迭代器必须产生大数类型本身（需先转换基本类型），而 `UBig`/`IBig`/`RBig` 可直接接受基本类型元素。

```rust
use dashu::float::DBig;

let vals = [dbig!(1), dbig!(1e-20), dbig!(-1)];
let folded = vals.iter().cloned().fold(DBig::ZERO, |a, b| a + b); // 0 — 1e-20 被舍入丢弃
let summed: DBig = vals.iter().sum();                             // 1e-20 — Sum 是精确的
```

## 混合类型算术

不同种类的大数之间**没有隐式混合类型运算符**（例如 `UBig + FBig` 不能编译）——请先显式转换（参见[类型转换](../convert.md)）。
