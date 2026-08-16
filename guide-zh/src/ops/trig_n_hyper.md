`FBig`/`DBig` 和 `CBig` 提供三角函数与双曲函数。它们被归在同一页面，是因为复数圆函数由实数圆函数*和*双曲函数共同构建。

## 实数函数

- 圆函数：`sin`、`cos`、`tan` 以及 `sin_cos`（同时计算两者）；反函数 `asin`、`acos`、`atan` 以及四象限的 `atan2(y, x)`。
- ×u 变体：`sin_unit(u)`、`cos_unit(u)`、`tan_unit(u)` 以及 `sin_cos_unit(u)`——计算 `2π·x/u`，即以整圈 divided by `u`（整圈的 u 分之一）为单位度量角度，反函数为 `asin_unit(u)`、`acos_unit(u)`、`atan_unit(u)` 与 `atan2_unit(y, u)`。`u = 2` 即 ×π 函数（`sin_pi` 等）；`u = 360` 即角度（度数）。
- 双曲函数：`sinh`、`cosh`、`tanh`、`sinh_cosh`；反函数 `asinh`、`acosh`、`atanh`。×π 变体 `sinh_pi`、`cosh_pi`、`sinh_cosh_pi` 使用共享缓存的 π。

角度以弧度为单位。`atan2` 遵循 C99 有符号零模型，这对于坐标轴上正确的支割线行为至关重要。

×u 圆函数在整数运算中对参数做*精确的* mod u 约化，因此与弧度函数不同，其精度不会随输入增大而下降（`sin_unit(10^100, 360)` 精确等于 `0`）。满足 `12x/u`（正切为 `8x/u`）为整数的参数得到精确结果：四分之一与八分之一点映射到 `0`/`±1`（`sin_unit(90, 360) == 1`），六分之一点映射到 `±1/2`（`sin_unit(30, 360) == 0.5`）。在 `u/4` 的奇数倍处正切到达极点，两侧单侧极限分别为 `+∞` 和 `−∞`：该情形是不定的，以错误形式报告。反函数族在坐标轴与对角线上返回精确的 `k·u/8` 值（`atan2_unit(1, 1, 360) == 45`），且 `u → 0` 时每个反函数的极限是有符号零。

## 复数函数

`CBig` 提供圆函数族 `sin`、`cos`、`tan`、`sin_cos`、`asin`、`acos` 和 `atan`，双曲函数族 `sinh`、`cosh`、`tanh`、`sinh_cosh`、`asinh`、`acosh` 和 `atanh`，以及 ×π 圆函数族 `sin_pi`、`cos_pi`、`tan_pi` 和 `sin_cos_pi`。圆函数通过以下恒等式由实数的 `sin`/`cos` 和 `sinh`/`cosh` 计算得出：

$$\sin(x+iy) = \sin x\cosh y + i\cos x\sinh y, \qquad \cos(x+iy) = \cos x\cosh y - i\sin x\sinh y.$$

×π 函数族将实数 `sin_cos_pi` 与缩放参数的双曲 `sinh_cosh_pi` 复合。双曲函数通过旋转恒等式 `sinh z = -i\sin(iz)`、`cosh z = \cos(iz)`、`tanh z = -i\tan(iz)` 复用圆函数——即对实部和虚部做一次精确互换，因此两族共享舍入验证。反函数（圆函数与双曲函数）遵循 Kahan 有符号零支割线公式。完整的 Annex G 特殊值和支割线表请参见[标准合规性](../compliance.md)。
