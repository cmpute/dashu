`FBig`/`DBig` 提供指数、对数、幂和根函数族，以及数学常数。`CBig` 提供这些函数的复数对应版本。与所有不精确运算一样，它们以[两层形式](../types.md#两层-api)提供——返回舍入结果的 `Context` 层和解包结果的便捷层。

## 实数函数

- 指数函数：`exp`、`exp_m1`（$e^x - 1$，在零附近精度更高）。
- 对数函数：`ln`、`ln_1p`（$\ln(1+x)$，在零附近精度更高）。
- 幂与根：`powi(IBig)`、`powf(&FBig)`、`sqrt`、`cbrt`、`nth_root(&n)` 以及 `hypot(&other)`（$\sqrt{x^2+y^2}$，不会溢出）。
- 常数：`FBig::pi(precision)` 计算 π；使用 [`CachedFBig`](../cached.md) 可在多次调用之间复用。

（`exp2`/`exp10`/`log2`/`log10` 将推迟到后续的 0.5.x 版本。）

## 复数函数

`CBig` 镜像了实数函数集，提供 `exp`、`ln`、`sqrt`、`powi` 和 `powf`，基于实数实现构建。其恒等式为

$$\exp(x+iy) = e^x(\cos y + i\sin y), \qquad \log z = \ln|z| + i\,\arg z,$$

其中 `ln` 的主支割线在 $]-\infty, 0]$ 上——因此虚部零的符号决定了支割线的一侧。完整的 C99 Annex G 特殊值和支割线表请参见[标准合规性](../compliance.md)。
