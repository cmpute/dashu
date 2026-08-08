`FBig`/`DBig` 和 `CBig` 提供三角函数与双曲函数。它们被归在同一页面，是因为复数圆函数由实数圆函数*和*双曲函数共同构建。

## 实数函数

- 圆函数：`sin`、`cos`、`tan` 以及 `sin_cos`（同时计算两者）；反函数 `asin`、`acos`、`atan` 以及四象限的 `atan2(y, x)`。
- 双曲函数：`sinh`、`cosh`、`tanh`、`sinh_cosh`；反函数 `asinh`、`acosh`、`atanh`。

角度以弧度为单位。`atan2` 遵循 C99 有符号零模型，这对于坐标轴上正确的支割线行为至关重要。

## 复数函数

`CBig` 提供圆函数族 `sin`、`cos`、`tan`、`sin_cos`、`asin`、`acos` 和 `atan`，以及双曲函数族 `sinh`、`cosh`、`tanh`、`sinh_cosh`、`asinh`、`acosh` 和 `atanh`。圆函数通过以下恒等式由实数的 `sin`/`cos` 和 `sinh`/`cosh` 计算得出：

$$\sin(x+iy) = \sin x\cosh y + i\cos x\sinh y, \qquad \cos(x+iy) = \cos x\cosh y - i\sin x\sinh y.$$

双曲函数通过旋转恒等式 `sinh z = -i\sin(iz)`、`cosh z = \cos(iz)`、`tanh z = -i\tan(iz)` 复用圆函数——即对实部和虚部做一次精确互换，因此两族共享舍入验证。反函数（圆函数与双曲函数）遵循 Kahan 有符号零支割线公式。完整的 Annex G 特殊值和支割线表请参见[标准合规性](../compliance.md)。
