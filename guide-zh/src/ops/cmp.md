比较运算**仅在同类大数之间**原生支持，大数与基本类型之间不直接比较——这是为了避免 [`num-bigint`#150](https://github.com/rust-num/num-bigint/issues/150) 中所述的 trait 重叠问题。若要将大数与基本类型进行比较，请启用 `num-order` 特性并使用 `NumOrd` trait。

## 相等性

`PartialEq`/`Eq` 执行值的相等性比较。对于 `FBig`/`DBig`，它比较的是数值表示，忽略上下文（精度与舍入模式），因此精度不同但值相同的两个浮点数比较结果为相等。有符号零比较相等：`+0 == -0`。`CBig` 按分量比较，每个分量上 `+0 == -0`。

## 排序

`UBig`/`IBig`/`RBig`/`FBig`/`DBig` 携带自然的数值全序（`Ord`）。无穷值位于两端：$-\infty < \text{有限} < +\infty$。`CBig` 按 `(re, then im)` 定义字典序全序——可用于排序和 `BTreeMap`，但请注意它*并非*代数意义上的大小序。

## 符号

有符号类型（`IBig`、`FBig`/`DBig`、`RBig`、`CBig`）提供 `.sign()` 方法（返回 `dashu::base::Sign`，其中零为 `Positive`）和 `.signum()` 方法（以同类型返回 `-1`、`0` 或 `+1`）。

## 大小比较与跨类型排序

`AbsOrd`（来自 `dashu-base`）按绝对值比较；对于 `CBig`，它按 $|z|$ 比较。`num-order` 特性添加了 `NumOrd` 用于跨不同数值类型（大数与基本类型）的排序，以及 `NumHash` 用于跨类型哈希，并保持它们之间的一致性。
