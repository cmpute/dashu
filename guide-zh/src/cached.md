[`CachedFBig`] 类型是携带一个共享 `Rc<RefCell<ConstCache>>` 句柄的 [`FBig`]。该缓存保存了数学常数（π、ln2、ln10）的精确二分裂状态，因此超越函数（`ln`、`exp`、`sin`、`cos`、……、`pi`）会复用并逐步扩展此前的工作，而非从头重新计算。

## 创建

`CachedFBig` 通过将缓存句柄附加到 `FBig` 上来创建：

```rust
use std::rc::Rc;
use core::cell::RefCell;
use dashu::float::{CachedFBig, ConstCache, FBig, Repr, Context};

let cache = Rc::new(RefCell::new(ConstCache::new()));

// 从 FBig 创建
let a = FBig::ONE.into_cached(cache.clone());

// 从原始组成部分配合新缓存创建
let b = CachedFBig::<_, 10>::with_cache(
    Repr::new(1234.into(), -3),
    Context::new(50),
);
```

使用 `From<FBig> for CachedFBig` 进行一次性转换（它会创建一个新的空缓存）：

```rust
let c: CachedFBig = FBig::from(3u8).into();
```

要丢弃缓存并恢复为普通 `FBig`，使用 `into_fbig()` 或 `From<CachedFBig> for FBig` trait：

```rust
let plain: FBig = cached.into();  // 或 cached.into_fbig()
```

## 缓存共享

`CachedFBig` 值之间的二元运算会在结果中保留缓存句柄：`(a + b).ln().exp()` 会全程扩展同一个缓存。当两个操作数携带不同的缓存句柄时，**左侧**的缓存将被保留。对于 `FBig op CachedFBig`，无论 `CachedFBig` 操作数在哪一侧，其缓存都会被保留。

与普通 `FBig` 和基本类型（`u8`、`i32`、`UBig` 等）的运算同样有效，并会保留 `CachedFBig` 操作数的缓存：

```rust
let cached = CachedFBig::<_, 10>::with_cache(
    Repr::new(2.into(), 0), Context::new(20),
);
let result = cached + 3u8;    // CachedFBig，缓存已保留
let result = 10i32 * cached;  // CachedFBig，缓存已保留
```

## 检查与清空缓存

使用 `cache()` 以只读方式借用缓存并检查其大小：

```rust
let terms = cached.cache().total_terms();
let words = cached.cache().total_words();
```

调用 `clear_cache()` 释放所有缓存的大整数内存。下一次超越函数运算将从头重新计算常数：

```rust
cached.clear_cache();
assert_eq!(cached.cache().total_terms(), 0);
```

## 更多构造函数与访问器

除了 `into_cached` / `with_cache` / `From<FBig>` 之外，`CachedFBig` 还镜像了 `FBig` 其余的构造 API，同时保留缓存句柄：

- `from_parts(significand, exponent)` —— 从尾数和指数构建，使用新缓存。
- `with_rounding::<NewR>()` —— 更改舍入模式，保留缓存句柄。
- `as_fbig()` —— 不可变地借用内部的 `FBig`（代价低；不会脱离缓存）。
- `from_repr(repr, context, cache)` / `into_repr()` —— 共享特定缓存句柄的原始 Repr 构造函数/析构函数。

## 直接计算常数

缓存保存了常数 π、ln2 和 ln10 的精确二分裂状态，因此生成它们的方法会复用并逐步扩展此前的工作，而非从头重新计算。在 `CachedFBig` 上，π 只需一次调用：

```rust
use std::rc::Rc;
use core::cell::RefCell;
use dashu::float::{CachedFBig, ConstCache};
use dashu::float::round::mode::HalfAway;

let cache = Rc::new(RefCell::new(ConstCache::new()));
let _pi = CachedFBig::<HalfAway, 10>::pi(100, &cache);
// 后续更高精度的调用会扩展同一缓存状态，而非从头开始
let _pi_more = CachedFBig::<HalfAway, 10>::pi(1000, &cache);
```

你也可以直接驱动一个裸的 `ConstCache`，而无需 `CachedFBig`——当你需要常数但不需要逐值的包装类型时，这会很有用。这些方法对基数和舍入模式是泛型的，单个缓存可服务于任意基数：

```rust
use dashu::float::ConstCache;
use dashu::float::round::mode::HalfAway;

let mut cache = ConstCache::new();
let pi = cache.pi::<10, HalfAway>(100).value();       // 从头计算
let pi_1000 = cache.pi::<10, HalfAway>(1000).value(); // 扩展缓存状态
let ln2 = cache.ln2::<10, HalfAway>(100);
let ln10 = cache.ln10::<10, HalfAway>(100);
```

`ln_base::<B, R>(precision)` 在 `B` 为 2 或 10（或 2 的幂）时会分派到缓存的 ln2 / ln10，否则回退到直接计算 `ln(B)`。

## 线程安全性

`CachedFBig` 以 `Rc<RefCell<ConstCache>>` 携带其缓存，因此它是 **`!Send + !Sync`** ——缓存值不能跨线程移动。`FBig` 本身保持 `Copy + Send + Sync`（这就是 `static_fbig!` 仍然有效的原因）；只有缓存包装类型是非线程安全的。`ConstCache` 是一个由大整数组成的普通结构体，本身是 `Send + Sync`，因此要跨线程共享一个缓存，可以将 `ConstCache`（或 `CachedFBig`）包装在 `Arc<Mutex<ConstCache>>` 中。底层的 `Context` 方法无论使用何种容器都接受 `Option<&mut ConstCache>`，因此不需要任何 API 变更。

## 示例：在运算链中复用常数

由于每个生成值的运算都会保留缓存句柄，一条超越函数链会全程复用相同的常数。从同一个共享句柄构建多个结果时，每个常数只需计算一次：

```rust
use std::rc::Rc;
use core::cell::RefCell;
use dashu::float::{CachedFBig, ConstCache, Context, Repr};
use dashu::float::round::mode::HalfAway;

type F = CachedFBig<HalfAway, 10>;
let cache = Rc::new(RefCell::new(ConstCache::new()));

// π 被计算并存入共享缓存……
let _pi_50 = F::pi(50, &cache);
// ……后续更高精度的调用会扩展它，而非从头开始
let _pi_1000 = F::pi(1000, &cache);

// 基于同一句柄构建的算术链会从头到尾保持该句柄
let a = F::from_repr(Repr::new(2.into(), 0), Context::new(50), cache.clone());
let b = F::from_repr(Repr::new(3.into(), 0), Context::new(50), cache.clone());
let _ = (a + b).ln().exp();

assert!(cache.borrow().total_terms() > 0);
```

## 复数：`CachedCBig`

复数类型 [`CachedCBig`](https://docs.rs/dashu-cmplx/latest/dashu_cmplx/struct.CachedCBig.html)
是 `CachedFBig` 的完全镜像：它包装一个
[`CBig`](https://docs.rs/dashu-cmplx/latest/dashu_cmplx/struct.CBig.html) 加上同一个共享的
`Rc<RefCell<ConstCache>>` 句柄，并将其贯穿于复数超越函数（`ln`、`exp`、
`sin`/`cos`/`tan`/`sin_cos`、`asin`/`acos`/`atan`、`powf`、`arg`）中。复数超越函数完全由实数
`FBig` 运算构建，因此**同一个 `ConstCache`（π、ln2、ln10）被原样复用**——不需要存储任何复数专有的常数。

```rust
use dashu::complex::{CBig, CachedCBig};
use dashu::float::FBig;

// 从普通 CBig 构建缓存的 1+1i（新缓存）
let z = CachedCBig::from(CBig::from_parts(FBig::from(1), FBig::from(1)));

// ln / exp 全程复用共享的实数常数缓存
let _ = z.clone().ln().exp();
```

`CachedCBig` 镜像了 `CBig` 的全部公开 API（格式化、比较、转换、包括与 `CBig` 和 `FBig` 的跨类型二元运算、`Neg`/`Inverse`、`Sum`/`Product`），并且与 `CachedFBig` 一样是 `!Send + !Sync`（因此 `CBig` 保持 `Send + Sync`，`static_cbig!` 不受影响）。一个有意的差异：`CachedCBig::into_parts` 返回的是**共享句柄**的 `(CachedFBig, CachedFBig)`，因此对任一部分的超越函数仍能利用缓存——区别于 `CBig::into_parts`，后者返回的是 `(FBig, FBig)`。第三方 trait（serde/num-traits/num-order/num-complex/rand）不做镜像；请通过 `.as_cbig()` 访问。

## `Fast*` 别名

对于超越函数密集的代码，元 crate 以短别名暴露了缓存变体，以便能方便地按名称使用更快的类型：

| 别名 | 类型 | 说明 |
|------|------|------|
| [`FastReal`](../index.html#fastreal) | `dashu_float::CachedFBig` | 基数 2，Zero——快速 `Real` |
| [`FastDecimal`](../index.html#fastdecimal) | `dashu_float::CachedFBig<HalfAway, 10>` | 快速 `Decimal` |
| [`FastComplex`](../index.html#fastcomplex) | `dashu_cmplx::CachedCBig` | 基数 2，Zero——快速 `Complex` |

三者均为 `!Send + !Sync`；非缓存的 `Real`/`Decimal`/`Complex` 仍为 `Send + Sync` 基线。
