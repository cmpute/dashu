`UBig` 和 `IBig` 支持完整的 Rust 标准格式化 trait 集合：`Display`、`Debug`、`Binary`、`Octal`、`LowerHex`、`UpperHex`。浮点数、有理数和复数类型支持 `Display` 和 `Debug`，并附带下文所述的额外进制/位置展开辅助方法。所有类型都遵循 `Formatter` 的符号、宽度、填充和对齐选项。

## 整数格式化

`Display` 以十进制渲染 `UBig`/`IBig`。`Binary`、`Octal`、`LowerHex` 和 `UpperHex` trait 分别以二进制/八进制/十六进制渲染，`#` 标志会添加传统的 `0b`/`0o`/`0x`/`0X` 前缀。对于其他基数，请使用 `in_radix(r)`（基数 2–36）；其 `#` 标志会将 9 以上的数位转为大写。

```rust
use dashu::integer::UBig;

let n = UBig::from(255u8);
assert_eq!(format!("{}", n), "255");
assert_eq!(format!("{:#x}", n), "0xff");
assert_eq!(format!("{:#b}", n), "0b11111111");

assert_eq!(format!("{}", n.in_radix(16)), "ff");
assert_eq!(format!("{:#}", n.in_radix(16)), "FF");
```

## 浮点数格式化

`FBig`/`DBig` 的 `Display` 根据指数定位小数点来渲染尾数——这是自然的位置形式，而非科学记数法。格式化器的精度选项会将结果舍入到指定的小数位数。

```rust
use core::str::FromStr;
use dashu::float::DBig;

assert_eq!(format!("{}", DBig::from_str("12.34")?), "12.34");
assert_eq!(format!("{:.1}", DBig::from_str("12.34")?), "12.3");
```

要使用科学记数法，请使用 `LowerExp`/`UpperExp`：在十进制下指数标记为 `e`/`E`，在其他基数下为 `@`。无穷值在 `Display` 和 `Debug` 下均渲染为 `inf` / `-inf`。

```rust
use core::str::FromStr;
use dashu::float::DBig;

assert_eq!(format!("{:e}", DBig::from_str("1234.5")?), "1.2345e3");
assert_eq!(format!("{:E}", DBig::from_str("1234.5")?), "1.2345E3");
```

## 有理数格式化

`RBig`/`Relaxed` 的 `Display` 渲染为 `numerator/denominator`，当分母为 `1` 时只渲染分子。`Binary`/`Octal`/`LowerHex`/`UpperHex` trait 和 `in_radix(r)` 以给定基数格式化分子和分母两部分。

```rust
use core::str::FromStr;
use dashu::rational::RBig;

assert_eq!(format!("{}", RBig::from_str("22/7")?), "22/7");
assert_eq!(format!("{}", RBig::from_str("5/1")?), "5");
```

对于位置（十进制）展开，请使用 `in_expanded(10)`。`{:.N}` 打印恰好 `N` 位小数；`#` 标志会检测循环节并用括号括起：

```rust
use dashu::rational::RBig;

let x = RBig::from_parts(1.into(), 3u8.into());
assert_eq!(format!("{:.4}", x.in_expanded(10)), "0.3333");
assert_eq!(format!("{:#}", x.in_expanded(10)), "0.(3)");
```

## 复数格式化

`CBig` 的 `Display` 使用代数 $a+bi$ 记法：虚部项始终带有显式符号，系数为 1 时省略（`i` 而非 `1i`），虚部为零时省略。

```rust
use dashu::complex::CBig;
use dashu::float::{FBig, round::mode::HalfAway};

type C = CBig<HalfAway, 10>;
type F = FBig<HalfAway, 10>;

assert_eq!(format!("{}", C::from_parts(F::from(1), F::from(2))), "1+2i");
assert_eq!(format!("{}", C::from_parts(F::from(-3), F::from(-4))), "-3-4i");
assert_eq!(format!("{}", C::from_parts(F::from(5), F::from(0))), "5");
assert_eq!(format!("{}", C::from_parts(F::from(0), F::from(1))), "i");
assert_eq!(format!("{}", C::from_parts(F::from(0), F::from(-1))), "-i");
```

输入端也接受相同的代数文法——参见[解析](./parse.md)。

## Debug 打印

`Debug` 输出用于快速检查（它**不是**稳定的序列化格式——参见[序列化](./serialize.md)）。大整数使用紧凑的 **head..tail** 格式——最高位数位、`..` 分隔符、最低位数位，中间部分省略——而小整数则完整打印。每种数值类型都有各自的 `Debug` 形式：

```rust
use core::str::FromStr;
use dashu::complex::CBig;
use dashu::float::{CachedFBig, Context, DBig, FBig, Repr, round::mode::HalfAway};
use dashu::integer::{IBig, UBig};
use dashu::rational::RBig;

// UBig / IBig — 大数值使用 head..tail，小数值完整打印
assert_eq!(format!("{:?}", UBig::from(12345u16)), "12345");
assert_eq!(format!("{:?}", IBig::from(-12345)), "-12345");
assert_eq!(
    format!("{:?}", UBig::ONE << 1000),
    "1071508607186267320..4386837205668069376"
);

// FBig / DBig — significand * base ^ exponent (prec: N)
let f: FBig = FBig::from(3u8); // FBig<Zero, 2>
assert_eq!(format!("{:?}", f), "3 * 2 ^ 0 (prec: 2)");
assert_eq!(format!("{:?}", DBig::from_str("12.34")?), "1234 * 10 ^ -2 (prec: 4)");

// CachedFBig — 暴露 repr 和精度的结构体
let c = CachedFBig::<HalfAway, 10>::with_cache(Repr::new(1234.into(), -3), Context::new(50));
assert_eq!(format!("{:?}", c), "CachedFBig { repr: 1234 * 10 ^ -3, precision: 50 }");

// RBig — numerator / denominator
assert_eq!(format!("{:?}", RBig::from_parts(1.into(), 3u8.into())), "1 / 3");

// CBig — re:<re> im:<im> (prec: N)
type F = FBig<HalfAway, 10>;
assert_eq!(
    format!("{:?}", CBig::<HalfAway, 10>::from_parts(F::from(3), F::from(4))),
    "re:3 im:4 (prec: 1)"
);
```

head..tail 的数位数量取决于 `Word` 大小——在 64 位平台上每端 19 位十进制数位，32 位平台上每端 9 位。详细形式 `{:#?}` 以结构化视图美观地打印值；对于复合类型，它会展示完整的分解：

```rust
use core::str::FromStr;
use dashu::complex::CBig;
use dashu::float::{CachedFBig, Context, DBig, FBig, Repr, round::mode::HalfAway};
use dashu::integer::UBig;
use dashu::rational::RBig;

let f: FBig = FBig::from(3u8);
let c = CachedFBig::<HalfAway, 10>::with_cache(Repr::new(1234.into(), -3), Context::new(50));
type F = FBig<HalfAway, 10>;
let z = CBig::<HalfAway, 10>::from_parts(F::from(3), F::from(4));

println!("{:#?}", UBig::from(12345u16));
println!("{:#?}", f);
println!("{:#?}", DBig::from_str("12.34")?);
println!("{:#?}", c);
println!("{:#?}", RBig::from_parts(1.into(), 3u8.into()));
println!("{:#?}", z);
```

依次对 `UBig`、`FBig`、`DBig`、`CachedFBig`、`RBig`、`CBig` 的输出如下：

```text
12345 (digits: 5, bits: 14)
FBig {
    significand: 3 (2 bits),
    exponent: 2 ^ 0,
    precision: 2,
    rounding: Zero,
}
FBig {
    significand: 1234 (digits: 4, bits: 11),
    exponent: 10 ^ -2,
    precision: 4,
    rounding: HalfAway,
}
CachedFBig {
    repr: Repr {
        significand: 1234 (digits: 4, bits: 11),
        exponent: 10 ^ -3,
    },
    precision: 50,
}
RBig {
    numerator: 1 (digits: 1, bits: 1),
    denominator: 3 (digits: 1, bits: 2),
}
CBig {
    re: FBig {
        significand: 3 (digits: 1, bits: 2),
        exponent: 10 ^ 0,
        precision: 1,
        rounding: HalfAway,
    },
    im: FBig {
        significand: 4 (digits: 1, bits: 3),
        exponent: 10 ^ 0,
        precision: 1,
        rounding: HalfAway,
    },
    precision: 1,
}
```
