`dashu-int` 提供最大公约数和模运算原语。

## 最大公约数

`Gcd` trait（来自 `dashu-base`）提供 `gcd`，`ExtendedGcd` 提供 `gcd_ext`，后者返回 `(gcd, x, y)`，满足 $a\cdot x + b\cdot y = \gcd(a,b)$。

```rust
use dashu::base::Gcd;
use dashu::integer::UBig;

let a = UBig::from(12u8);
let b = UBig::from(8u8);
assert_eq!((&a).gcd(&b), UBig::from(4u8));
```

## 模运算

对于针对固定模数的重复运算，可预计算一个 `ConstDivisor` 并将值约化为 `Reduced`。加法、减法、乘法、幂运算和求逆随后基于预计算的模数运行，结果以 `(mod N)` 形式输出。

```rust
use dashu::integer::{UBig, fast_div::ConstDivisor};

let ring = ConstDivisor::new(UBig::from(10000u32));
let x = ring.reduce(12345);
let y = ring.reduce(55443);
assert_eq!(format!("{}", x - y), "6902 (mod 10000)");
```

## Montgomery 约化器

对于**奇数模数**，`MontgomeryRepr` 提供 [Montgomery 形式]模运算——是上述 Barrett 式 `ConstDivisor`/`Reduced` 的替代方案。模乘法是普通乘法后接一次 Montgomery 约化（REDC），而非除法，因此当 REDC 的代价低于除法时，它比 Barrett 更快。

Montgomery 乘法、平方和幂运算在约 256–4096 位范围内优于 `Reduced`；超过约 8 kbits 时两者性能相当。**对于求逆密集型工作负载，请优先使用 `Reduced`**——Montgomery 求逆必须先退出 Montgomery 形式、运行扩展最大公约数算法、再重新进入，而 `Reduced` 可直接求逆。

```rust
use dashu::integer::{UBig, monty::MontgomeryRepr};

// 一个 Mersenne 素数（奇数）。
let p = UBig::from(2u8).pow(607) - UBig::ONE;
let ring = MontgomeryRepr::new(p.clone());

// 将值约化为 Montgomery 形式，然后进行模乘 / 平方 / 幂运算
let a = ring.reduce(123);
assert_eq!(a.pow(&(p - UBig::ONE)), ring.reduce(1)); // Fermat: a^(p-1) = 1 (mod p)
```

`MontgomeryRepr::new(m)` 要求 `m` 为奇数；`ring.reduce(x)` 将 `x` 转换为 Montgomery 形式，得到的 `Montgomery` 值支持 `+`、`-`、`*`、`.sqr()`、`.pow(&exp)` 和 `.inv()`。

[Montgomery 形式]: https://en.wikipedia.org/wiki/Montgomery_modular_multiplication

## 丢番图逼近

实数的有理逼近——容差范围内的最简有理数、连分数——位于 `RBig` 上；关于 `simplest_in` / `nearest_in`，请参见[类型转换](../convert.md#转换到-rbig)。
