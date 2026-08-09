## 数值类型

在 `dashu` 的各个 crate 中，每种任意精度数值都有独立的类型，具体如下：

- `dashu::integer::UBig`（别名 `dashu::Natural`）表示无符号整数（即自然数）。
- `dashu::integer::IBig`（别名 `dashu::Integer`）表示（带符号）整数。
- `dashu::float::FBig`（别名 `dashu::Real`）表示以浮点形式表示的实数（$\text{significand} \times \text{base}^{\text{exponent}}$）。
- `dashu::float::DBig`（别名 `dashu::Decimal`）是 `FBig` 在 `base = 10` 时的特化版本。
- `dashu::rational::RBig`（别名 `dashu::Rational`）表示有理数。它有一个变体 `dashu::rational::Relaxed`，同样表示有理数，但不强制要求该数值处于规范化形式。
- `dashu::complex::CBig`（别名 `dashu::Complex`）表示复数，由两个共享同一精度和舍入模式的 `FBig` 部分组 pair 构成。

这些数值类型都实现了通用运算，具体用法请参阅其他章节或 API 文档。

### Word

`dashu::integer::Word` 是一个表示原生机器字（native machine word）的无符号整数。`Word` 的大小通常取决于平台，例如在 32 位平台上 `Word` 为 `u32`。不过，这一行为可以通过设置 `force_bits` 配置标志来覆盖（例如在环境变量 `RUSTFLAGS` 中加入 `--cfg force_bits="32"`）。由于该类型在不同平台上并不一致，编写可移植程序时请谨慎使用。

此外，还有另一个类型 `DoubleWord`，表示大小为 `Word` 两倍的整数类型。它是能在不进行堆分配的情况下放入一个 `UBig` 实例的最大整数类型，也参与某些 const 构造函数。

### Sign

`dashu::base::Sign` 是一个**二值**枚举，用于表示数值的符号。出于效率与清晰性的考虑，数值零会被归类为 `Sign::Positive`，尽管它在数学上是无符号的。（设想一下，如果用三值格式来存储符号，每个数值实例都要额外占用一位来保存符号，并且在运算时还要增加额外的分支。）若要获得三值表示，推荐使用各数值类型上的 `.signum()` 方法。

该枚举还提供了一些与符号相关的便捷工具。例如，你可以通过 `dashu::base::Signed` trait 获取任意基本类型数值或大数的符号，也可以将一个符号与另一个符号相乘。你甚至可以将符号与 `core::cmp::Ordering` 相乘，当你想根据操作数的符号来翻转比较结果时这会非常方便；`dashu` 的比较实现中就大量使用了这一技巧。

### `UBig` 的内存布局

`dashu` 库中最基础的类型是自然数 `UBig`。一个 `UBig` 数值的底层表示是一个由 `Word` 组成的数组。`dashu` 的特别之处在于：当一个 `UBig` 只包含一两个 word 时，这些 word 会被内联存储，不会发生堆分配。此外，内联存储的 `UBig` 通常只占用 3 个 word 的栈空间（如有兴趣可查看源码了解细节）。得益于 `dashu` 特有的内存优化，`Option<UBig>` 乃至 `Option<IBig>` 也都只占用 3 个 word。

> 目前 `UBig` 实例的内存布局尚未最终确定，因此暂时不要依赖它。此外，不同版本之间不保证内存布局的兼容性。内存布局很可能要到 `v1.0.0` 版本才会稳定下来。

### `FBig` 的内存布局

`FBig`（以及 `DBig`）的布局与其他类型略有不同。一个 `FBig` 实例包含数值表示 `dashu::float::Repr` 和上下文 `dashu::float::Context` 两部分。每当基于某个 `FBig` 创建新的 `FBig` 时，上下文都会被复制。上下文目前保存着与该数值相关的舍入信息和精度。上下文被刻意保持得很轻量（`Copy` + `Send` + `Sync`）：数学常数（如 π、ln2、ln10）的共享缓存位于上下文*之外*，存放在独立的 [`CachedFBig`](./cached.md) 包装类型中，这样普通的 `FBig` 才能保持低成本的拷贝，并可在 `const`/`static` 上下文中使用。因此，如果你不想存储这些额外的上下文信息，可以直接只存储 `FBig` 的 `Repr` 部分。对 `Repr` 的后续运算可以通过 `Context` 的关联方法来调用，这些方法都接受某个 `Repr` 实例的引用。不过，在某些情况下这可能会带来少许开销。

### `CBig` 的内存布局

`CBig`（位于 `dashu-cmplx` crate 中）镜像了 `FBig` 的 `Repr`+`Context` 布局，并将其推广到两个部分：一个 `CBig<R, B>` 实例持有两个 `Repr<B>` 部分——实部 `re` 和虚部 `im`——它们共享**同一个** `Context<R>`。只存一个上下文（而不是包装两个各自携带上下文的 `FBig`），使得“精度一致”这一不变量成为*物理层面*的保证：精度槽只有一个，`re` 和 `im` 在结构上就不可能不一致。每个部分各自保存自己的尾数长度；共享的上下文只保存精度上限和唯一的舍入模式，并独立地作用于每个部分。与 `FBig` 一样，上下文是 `Copy` 的，而尾数在堆上分配，因此 `CBig` 是 `Clone` 但不是 `Copy`。

由于两个部分都是 `Repr`，`CBig` 原封不动地复用了 `dashu-float` 的有符号零 / 有符号无穷 / 支割线（branch cut）机制。它遵循 C99 Annex G / Kahan 模型（参见[标准合规性](./compliance.md)），并且与 `FBig` 一样**没有 NaN**——C99 中那些会产生复数 NaN 的情形，在上下文层会以 `FpError` 的形式上报。`CBig::from_parts(re, im)` 会取两个操作数上下文中较大的那个。构造、运算、超越函数和 I/O 的相关内容，分别在[构造与析构](./construct.md)、[类型转换](./convert.md)和[运算](./ops/index.md)中介绍。

### 复数运算中的无穷

`CBig` 中的复数无穷是**单一的黎曼点** `+∞ + i·0`，并且——与 `FBig` 的 `±∞` 完全一样——它是**终端值**：可以由有限输入“爆炸”*产生*，但*绝不被接受为操作数*。任何接收无穷操作数的算术或超越运算，都在上下文层以 `FpError::InfiniteInput` 拒绝（在便捷层 panic）。这让 `dashu-cmplx` 与 `dashu-float` 保持一致，并且内部统一：没有任何运算会把无穷折叠进结果后又去消费它。

| 产生 `+∞ + i·0`（有限输入） | 被拒绝（无穷操作数） |
|---|---|
| `1/0`、有限非零的 `z/0` | `z·∞`、`∞·z`、`∞·∞`、`0·∞` |
| `exp(+∞ + i·0)`（以及 `exp(-∞ + i·0) = 0`） | `z/∞`、`∞/z`、`∞/∞` |
| `log(0) = -∞ + i·0` | `inv(∞)`、`log(∞)`、`sqrt(∞)` |
| 上溢饱和 | `z ± ∞`、`∞ ± z` |

`proj` 是唯一一个*接收*无穷值并返回结果的函数：它把任何部分为无穷的 `CBig` 投影为 `+∞ + i·0`（虚部零保持原始虚部的符号），按 C99 Annex G §G.5.3。`0/0` 为 `FpError::Indeterminate`。

## 辅助类型

除了数值类型之外，各 crate 中还用到几种辅助类型：**dashu-base** 中的 `Sign` 和 `Approximation`，**dashu-float** 中的 `ConstCache` 和 `FpResult`，以及 **dashu-cmplx** 中的 `CfpResult`。

### Sign

在 `dashu` 中，数值的符号用枚举 `dashu::base::Sign` 表示。它只有两个变体：`Positive` 和 `Negative`。零被视为 `Positive`。`Sign` 可以由布尔值通过 `::from()` 转换而来，其中 `true` 映射为 `Negative`。

要获取一个数值的符号，数值类型上通常会有 `.sign()` 方法。对于基本整数，可以通过 `dashu::base::Signed` trait 获取符号。

`Sign` 类型还支持一些运算，即 `Neg` 和 `Mul`。可以用 `Neg` 翻转符号，也可以将它与另一个 `Sign` 或其他数值类型相乘，作用于后者的符号。

### Approximation

`Approximation` 枚举是 `dashu` 中另一个常用类型。它用于运算可能返回不精确结果的情形（如舍入和数值转换）。该枚举有两个变体：`Exact` 和 `Inexact`，后者带有一个误差项，用于表示不精确运算所引入误差的符号或大小。

当你持有一个 `Approximation` 实例时，可以调用 `.value()`、`.value_ref()` 或 `unwrap()` 来获取运算结果，调用 `.error()` 来获取误差项。该结构体还支持函数式编程风格的方法，例如 `.map()` 和 `.and_then()`。

### ConstCache

`dashu::float::ConstCache` 保存了数学常数 π、ln2 和 ln10 的精确二分裂（binary-splitting）状态，因此在不断提高精度的情况下重复调用超越函数时，可以*扩展*此前的工作，而不是从头重新计算。它是一个由大整数组成的普通结构体——与基数无关、`Send` + `Sync`——单个缓存即可服务于任意基数。`FBig` 和 `Context` 本身保持 `Copy` 且不携带任何缓存；该状态存放在独立的 [`CachedFBig`](./cached.md) 包装类型中（作为 `Rc<RefCell<ConstCache>>`），你也可以直接驱动一个裸的 `ConstCache`。

### FpResult 和 CfpResult

上下文层的不精确运算返回的是一个结果类型，而非裸值：`dashu::float::FpResult<T> = Result<Rounded<T>, FpError>`，其中 `Rounded<T>` 是携带 `Rounding` 标志的 [`Approximation`](#approximation)。复数的对应类型是 `dashu::complex::CfpResult`（`Result<CRounded<CBig>, FpError>`），其 `CRounded` 为每个坐标轴各携带一个 `Rounding` 标志。`FpError` 用于报告运算为何无法产生一个有限的、正确舍入的值：`Overflow`/`Underflow`、`Indeterminate`（例如 `0/0`）、`OutOfDomain`、`InfiniteInput` 以及 `ZivRetryLimitExceeded`（超越函数未能认证其舍入——只有当误差半径估计有误时才可能触发）。

## 两层 API

不精确运算——除法、超越函数，以及任何可能上溢、下溢或超出定义域的运算——都以两层形式提供：

- **上下文层（Context layer）**——`Context` 的方法接受 `&Repr`，返回 [`FpResult`](#fpresult-和-cfpresult)`<Rounded<FBig>>`（复数情形为 `CfpResult<CRounded<CBig>>`）：一个正确舍入的结果或一个 `FpError`，并携带舍入方向。它们接受可选的 `&mut ConstCache`，以便复用常数。
- **便捷层（Convenience layer）**——`FBig`/`CBig` 上的固有方法和运算符（`.exp()`、`.ln()`、`+`、`*`……）会解包为裸值：遇到 `Indeterminate`/`OutOfDomain`/`InfiniteInput` 时 panic，遇到上溢/下溢则饱和为 `±∞`/`±0`。

日常代码请使用便捷层；当你需要舍入方向或显式的错误处理时，再下沉到上下文层。

核心算术中你能遇到的所有 panic 都是可追踪的：`dashu-int` 和 `dashu-float` 中所有主动的 `panic!` 消息都定义在各 crate 唯一的 `error.rs` 中——[`dashu-int` 的 `error.rs`](https://github.com/cmpute/dashu/blob/master/integer/src/error.rs) 与 [`dashu-float` 的 `error.rs`](https://github.com/cmpute/dashu/blob/master/float/src/error.rs)——因此你看到的任何消息都可以在同一个地方查找到（并上报）。
