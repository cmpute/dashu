有多种方式可以构造和析构这些数值类型，具体如下所示。这些构造函数用于直接从数值的组成部分来构建数值。若要从其他表示形式构造，请参阅[输入与输出](./io/index.md)和[类型转换](./convert.md)章节。

## 常量

所有数值类型都提供了若干与之关联的常量。你可以用它们来构造实例，也可以直接与二元运算符搭配使用。这些常量包括：

- `UBig`：`::ZERO`、`::ONE`
- `IBig`：`::ZERO`、`::ONE`、`::NEG_ONE`
- `FBig`/`DBig`：`::ZERO`、`::ONE`、`::NEG_ONE`、`::INFINITY`、`::NEG_INFINITY`
- `RBig`：`::ZERO`、`::ONE`、`::NEG_ONE`
- `CBig`：`::ZERO`（$0+0i$）、`::ONE`（$1+0i$）、`::NEG_ONE`（$-1+0i$）、`::I`（$0+1i$）

## `UBig` 的原始构造函数

对于 `UBig`，可以通过 `::from_words()` 方法从 [`Word`](./types.md#word) 切片来构造。word 必须按小端序排列，即第一个 word 表示数值的最低有效部分。如果要构造的整数较小，也可以使用 `::from_word()` 和 `::from_dword()` 方法，它们可以在 `const` 上下文中调用。

要析构一个 `UBig`，目前我们暂不支持获取 `UBig` 中存储的 word 的所有权。你只能通过 `.as_words()` 方法获取对这些 word 的引用。未来当 `UBig` 的内存布局稳定后，可能会添加一个将 word 所有权交出的析构函数，以避免不必要的复制。

## 从组成部分构造

其他数值类型通常由多个部分组成。你可以使用 `::from_parts()` 和 `::from_parts_const()` 方法来构造它们。后者可以在 `const` 上下文中调用，但在使用 `::from_parts_const()` 时，组成部分的大小有所限制。

各类型的组成部分如下：

- `::from_parts()`
  - `IBig` = 符号：`Sign` + 绝对值：`UBig`
  - `FBig`/`DBig` = 尾数：`IBig` + 指数：`isize`
  - `RBig` = 分子：`IBig` + 分母：`UBig`
  - `CBig` = 实部：`FBig` + 虚部：`FBig`（结果精度取两者中的较大值）

对于 `RBig`，还有一个备选的 `from_parts_signed(numerator, denominator)`，它接受**有符号**的分母（`IBig`），因此符号可以放在任一分量上。
- `::from_parts_const()`
  - `IBig` = 符号：`Sign` + 绝对值：`DoubleWord`
  - `FBig`/`DBig` = 符号：`Sign` + 尾数：`DoubleWord` + 指数：`isize`
  - `RBig` = 符号：`Sign` + 分子：`DoubleWord` + 分母：`DoubleWord`

值得注意的是，`FBig` 和 `DBig` 的构造函数还会决定结果浮点数的精度。通过 `::from_parts()` 创建的浮点数的精度等于其绝对值在给定基数下的数位位数。通过 `::from_parts_const()` 创建的浮点数的精度可以是从绝对值推断得出（与 `::from_parts()` 相同），也可以来自该方法的 `min_precision` 参数。

要析构这些数值类型，请使用 `::into_parts()` 函数来无复制地获取各组成部分。但对于 `FBig`/`DBig`，你应该先使用 `.into_repr()` 获取底层表示 `Repr`，然后再使用 `Repr` 的 `.into_parts()` 方法来获取绝对值和尾数。

## `dashu-macros`

我们还提供了通过宏 `ubig!`/`ibig!`/`fbig!`/`dbig!`/`rbig!`/`cbig!` 从字面量创建大数的便捷且高效的方式。这些宏可以直接从 `dashu-macros` crate 或 `dashu` 元 crate 获取。`cbig!` 宏接受与 `CBig` 的 `FromStr` 相同的代数形式（例如 `cbig!(3+4i)`、`cbig!(-i)`）或 `re, im` 对（例如 `cbig!(3, 4)`）。

你可以直接将数字字面量作为参数而无需加引号（例如 `dbig!(3.1415926535897932384626)`），并且无需担心精度丢失，因为它保证能够无近似地精确创建数值。此外，这些宏的运行时开销极小，因为数值在编译期就已由宏预处理完毕。

当数值精度不高时，这些宏可以在 `const` 环境中使用，不过这一能力取决于精度和机器字大小。要创建大型常量，可以使用该 crate 中的 `static_*` 宏（如 `static_ubig!`）。它们的语法与普通宏相同，不同之处在于宏的输出是静态实例的引用，而非直接生成实例。这些用于静态创建的宏还有其他一些限制。

有关这些宏的详细用法，请参阅 [`dashu-macros` 的文档](https://docs.rs/dashu-macros/latest/dashu_macros/)。
