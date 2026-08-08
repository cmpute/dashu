# dashu

[English](README.md) | [简体中文](README_zh.md)

<img src="guide/src/assets/dashu-banner.png" alt="dashu">

[![Crate](https://img.shields.io/crates/v/dashu.svg)](https://crates.io/crates/dashu)
[![Docs](https://docs.rs/dashu/badge.svg)](https://docs.rs/dashu)
[![Tests](https://github.com/cmpute/dashu/actions/workflows/tests.yml/badge.svg)](https://github.com/cmpute/dashu/actions)
[![MSRV 1.68](https://img.shields.io/badge/rustc-1.68%2B-informational.svg)](#dashu)
[![License](https://img.shields.io/crates/l/dashu)](#license)
[![Book](https://img.shields.io/badge/book-user_guide-yellow.svg)](https://zyxin.xyz/dashu-zh/)

一套用 Rust 实现的、面向数学与数值计算的任意精度数值（即大数）库。它是 GNU GMP + MPFR + MPC 的 Rust 原生替代方案。其主要特性包括：
- 纯 Rust 实现，完整支持 `no_std`。
- 优先关注易用性与可读性，其次才是运行效率。
- 经过优化的运行速度与内存占用。
- 当前 MSRV 为 1.68。

## 套件内的crate

- [`dashu-base`](./base)：通用 trait 定义
- [`dashu-int`](./integer)：任意精度整数
- [`dashu-float`](./float)：任意精度浮点数
- [`dashu-ratio`](./rational)：任意精度有理数
- [`dashu-cmplx`](./complex)：任意精度复数
- [`dashu-macros`](./macros)：用于创建大数的宏

`dashu` 是一个元 crate（meta crate），重新导出上述所有子 crate 中的类型。各子目录下的 README.md 中有针对单个 crate 的专门介绍。

## 示例

### 构造与字面量宏

使用编译期字面量宏（无精度损失，任意大小）和元 crate 的类型别名来构造数值——为每种数域提供可读的名称：

```rust
use dashu::{ubig, ibig, fbig, dbig, rbig, cbig};
use dashu::{Natural, Integer, Real, Decimal, Rational, Complex};

// Compile-time literals — zero precision loss, any size
let n: Natural = ubig!(0x5a4653ca_67376856_5b41f775_d6947d55_cf3813d1);
let e: Real = fbig!(0x1.ffffp1023);
let pi: Decimal = dbig!(3.1415926535897932384626);
let r: Rational = rbig!(22 / 7);
let z: Complex = cbig!(3 + 4i); // 复数，默认按十进制解析

// Meta-crate type aliases cover every number domain
let _neg: Integer = ibig!(-0x10ff);
let _prod = &n * &_neg; // Natural × Integer → Integer

// Explicit radix and base prefixes work too
let _hex = ubig!(dead_beef base 16);
let _bin = ibig!(-0b1111);

// Associated constants are available on every type
let _one: Natural = Natural::ONE;
let _unit = Complex::I; // the imaginary unit
```

### 字符串转换

从任意进制的字符串解析数值，再用完整的 `std::fmt` 迷你语法格式化——十六进制、科学计数法、位置展开式等等：

```rust
use dashu::{ubig, Integer, Decimal, Rational};
use core::str::FromStr;

// Parse from strings — any base, scientific notation, rational form
let a = Integer::from_str_radix("1a2b3c", 16).unwrap();
let b: Decimal = "3.1415926535897932384626".parse().unwrap();
let c: Rational = "22/7".parse().unwrap();

// Full std::fmt mini-language for every type
assert_eq!(format!("{:#x}", a), "0x1a2b3c");
assert_eq!(format!("{:e}", b), "3.1415926535897932384626e0");
assert_eq!(format!("{:#}", c.in_expanded(10)), "3.(142857)");

// in_radix formats in any base; {:.N} rounds the fractional digits
assert_eq!(format!("{}", a.in_radix(32)), "1kaps");
assert_eq!(format!("{:.3}", c.in_expanded(10)), "3.143");

// Debug prints a compact head‥tail form for large values
assert_eq!(
    format!("{:?}", ubig!(1) << 1000),
    "1071508607186267320..4386837205668069376"
);
```

### 序列化

启用 `serde` 特性，即可进行紧凑的二进制编码（如 `postcard`）和保留精度的、人类可读的 JSON 往返：

```rust
// requires: features = ["serde"]

use dashu::{Integer, Rational, Real};

// Binary format: compact little-endian byte encoding
let a = Integer::from(12345u32);
let bytes = postcard::to_stdvec(&a).unwrap();
let b: Integer = postcard::from_bytes(&bytes).unwrap();
assert_eq!(a, b);

// Human-readable: precision-preserving string format
let r = Rational::from_parts(22u8.into(), 7u8.into());
let json = serde_json::to_string(&r).unwrap();
assert_eq!(json, r#""22/7""#);

// JSON round-trips preserve the value exactly
let rt: Rational = serde_json::from_str(&json).unwrap();
assert_eq!(rt, r);

// Floats serialize losslessly too, with precision preserved
let x: Real = "0x1.8p0".parse().unwrap(); // 1.5
let json2 = serde_json::to_string(&x).unwrap();
let _rt: Real = serde_json::from_str(&json2).unwrap();
```

### 随机数

启用 `rand` 特性，即可进行均匀采样——指定位长的整数、指定精度的 `[0, 1)` 区间浮点数等等：

```rust
// requires: features = ["rand"]

use dashu::{Natural, Integer, Real};
use dashu::base::BitTest;
use dashu::integer::rand::UniformBits;
use dashu::float::rand::Uniform01;
use rand::RngExt;

let mut rng = rand::rng();

// Uniform random integers with a bit-length limit
let a: Natural = rng.sample(UniformBits::new(256));
let b: Integer = rng.sample(UniformBits::new(64));
assert!(a.bit_len() <= 256 && b.bit_len() <= 64);

// Uniform floats in [0, 1) at chosen precisions
let x: Real = rng.sample(Uniform01::new(53)); // binary64
let _y: Real = rng.sample(Uniform01::new(200)); // higher precision
assert!(x >= Real::ZERO && x < Real::ONE);

// sample_iter streams an unbounded sequence
let _stream: Vec<Natural> = rng.sample_iter(UniformBits::new(8)).take(3).collect();
```

### 类型转换

在不同数域之间自由转换——二进制与十进制浮点互转、浮点转有理数（还原程序员本意的分数）、浮点按舍入方向转整数：

```rust
use dashu::float::round::mode::Zero;
use dashu::{Real, Decimal, Rational, Integer};

// Base conversion: binary (Real) ↔ decimal (Decimal)
let x: Decimal = "3.141592653589793".parse().unwrap();
let y = x.to_binary().value(); // Decimal → binary float
let _back = y.to_decimal().value(); // and back

// Float → rational: recover the fraction the programmer meant
let r = Rational::simplest_from_f64(0.1).unwrap();
assert_eq!(r, Rational::from_parts(1u8.into(), 10u8.into()));

// Float → integer with a rounding direction
let floor: Integer = y.to_int().value(); // truncates toward zero
assert_eq!(floor, Integer::from(3u8));

// Exact rational → float at a chosen precision
let third = Rational::from_parts(1u8.into(), 3u8.into());
let _f = third.to_float::<Zero, 2>(50).value(); // 1/3 in base 2
```

## Python 包

[`dashu-python`](./python) 是 dashu 核心功能的友好试验田：通过 PyPI 上的
[`dashu-rs`](https://pypi.org/project/dashu-rs/) 包，用户无需 Rust 工具链，即可在
Python 中体验 dashu 的任意精度整数、有理数、浮点数与复数，直观了解 dashu 的能力。
除了用于探索 dashu 之外，它本身也是一个独立的、面向 Python 生态的任意精度数值包。

## 许可证

根据以下任一协议授权：

 * Apache License, Version 2.0
   ([LICENSE-APACHE](../LICENSE-APACHE) 或 https://www.apache.org/licenses/LICENSE-2.0)
 * MIT license
   ([LICENSE-MIT](../LICENSE-MIT) 或 https://opensource.org/licenses/MIT)

由你自行选择。

## 贡献

除非你明确声明，否则依据 Apache-2.0 协议的定义，任何由你主动提交并包含在本作品中的贡献，
都将按上述方式双重授权，不附加任何额外的条款或条件。
