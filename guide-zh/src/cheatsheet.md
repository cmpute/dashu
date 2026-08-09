dashu 数值类型的速查参考。详细内容请参阅链接页面。

## 类型

| 类型 | Crate | 描述 | 字面量 |
|------|-------|------|---------|
| `UBig` | dashu-int | 无符号整数 | `ubig!(123)` |
| `IBig` | dashu-int | 有符号整数 | `ibig!(-123)` |
| `FBig` | dashu-float | 浮点数，默认基数 2 | `fbig!(0x1.8)` |
| `DBig` | dashu-float | 十进制浮点数，基数 10 | `dbig!(1.5)` |
| `RBig` | dashu-ratio | 有理数 | `rbig!(22/7)` |
| `CBig` | dashu-cmplx | 复数，默认基数 2 | `cbig!(1+2i)` |

## 构造

| 方式 | 示例 |
|------|------|
| `From` 基本类型 | `UBig::from(123u32)` |
| 解析 | `"12.34".parse::<DBig>()?` |
| 从组成部分 | `RBig::from_parts(1.into(), 3u8.into())` |
| 字面量宏 | `dbig!(1.5)`、`cbig!(1+2i)` |
| 原始 word | `UBig::from_words(&[3, 2, 1])` |

## 转换

无损失的转换使用 `From`；可能有损失的转换使用 `TryFrom`（任何精度丢失都会导致失败）。完整矩阵请参阅[类型转换](./convert.md)。

| 从 → 到 | Trait | 说明 |
|-----------|-------|-------|
| `UBig` → `IBig` | `From` | |
| `IBig` → `UBig` | `TryFrom` | 负数时失败 |
| 整数 → `FBig` | `From` | 精度从绝对值推断 |
| `FBig` → 整数 | `TryFrom` | 有小数或无穷时失败 |
| `FBig` → `f32`/`f64` | `.to_f32()` / `.to_f64()` | 返回 `Rounded<f*>` |
| `f32`/`f64` → `FBig` | `TryFrom` | 仅限基数 2 |
| 实数 → `CBig` | `From` | 虚部为 `+0` |
| `CBig` → `FBig` | `TryFrom` | 除非虚部为零，否则失败 |

## 运算符

| 类型 | `+ - * /` | `%` | `<< >>` | `& \| ^` | `!` |
|------|:---:|:---:|:---:|:---:|:---:|
| `UBig` | ✓ | ✓ | ✓ | ✓ | — |
| `IBig` | ✓ | ✓ | ✓ | ✓ | ✓ |
| `FBig` / `DBig` | ✓ | ✓ | — | — | — |
| `RBig` | ✓ | ✓ | — | — | — |
| `CBig` | ✓ | — | — | — | — |

所有类型都实现了 `Sum`/`Product`。对于 `FBig`/`DBig`，`Sum` 是正确舍入的（精确累加，一次舍入——而非 `+` 折叠）；对于 `FBig`/`DBig`/`CBig`，迭代器必须产生大数类型（或 `&T`），而非基本类型。参见[基本运算](./ops/basic.md#聚合sum-和-product)。

## 格式化

| 类型 | `Display` | `Debug` | 其他 |
|------|-----------|---------|-------|
| `UBig`/`IBig` | 十进制 | head‥tail（`#?` 可显示数位/位数） | `Binary`/`Octal`/`Hex`、`in_radix(2..=36)` |
| `FBig`/`DBig` | 位置记数 | `sig * base ^ exp` | `LowerExp`/`UpperExp` |
| `RBig` | `num/den` | — | `in_radix`、`in_expanded` |
| `CBig` | `a+bi` | `re:.. im:.. (prec: ..)` | — |

## 关键方法

| 方法 | 所属类型 | 返回 |
|--------|-----|---------|
| `.exp()` / `.ln()` / `.sqrt()` | `FBig`、`CBig` | 同类型 |
| `.sin()` / `.cos()` / `.tan()` / `.sin_cos()` | `FBig`、`CBig` | 同类型 |
| `.powi(IBig)` / `.powf(&Self)` | `FBig`、`CBig` | 同类型 |
| `.with_precision(p)` | `FBig` | `Rounded<FBig>` |
| `.to_decimal()` / `.to_binary()` | `FBig` | `Rounded<DBig>` / `Rounded<FBig>` |
| `.conj()` / `.proj()` | `CBig` | `CBig` |
| `.abs()` / `.arg()` / `.norm()` | `CBig` | `FBig` |
| `.gcd(&b)` / `.gcd_ext(&b)` | `UBig`/`IBig`（`Gcd`） | `Self` / `(gcd, x, y)` |
