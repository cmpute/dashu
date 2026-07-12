## 安装

```sh
pip install dashu-rs
```

`dashu-rs` 是 [PyPI](https://pypi.org/project/dashu-rs/) 上的独立 wheel 包——
无需 Rust 工具链或编译器。它提供一个原生的 `dashu` 模块：

```python
from dashu import UBig, IBig, RBig, FBig, DBig, CBig
```

## 类型

| 类型 | 底层实现 | 基数 / 舍入 |
|------|---------|-----------------|
| `UBig`、`IBig` | 任意精度整数 | 无符号 / 有符号 |
| `RBig` | 任意精度有理数 | 精确的 `numerator / denominator` |
| `FBig` | 任意精度浮点数 | 基数 2，舍入模式 `Zero` |
| `DBig` | 任意精度浮点数 | 基数 10，舍入模式 `HalfAway`（十进制） |
| `CBig` | 任意精度复数 | 基数 2 |

所有类型都是 `Send` + `Sync`（兼容自由线程的 Python）。构造函数既接受原生 Python 数值，也接受字符串：

```python
UBig(144)              # 整数
FBig(1.5)              # 浮点数（使用模块默认精度）
DBig("1.23")           # 十进制字符串
RBig(Fraction(1, 3))   # fractions.Fraction
CBig(3.0, 4.0)         # (实部, 虚部)
```

## 运算

算术运算、比较以及 `bool()` 都接受任意 Python 数值（跨类型分派），因此混合操作数可以直接使用：

```python
UBig(2) + 3 == 5
FBig(1.5) * 2 == FBig(3.0)
UBig(17) // 5 == 3
divmod(UBig(17), 5) == (UBig(3), UBig(2))
```

- **整数**：`+ - * / // % **`、比较、原地运算、位操作
  （`& | ^ << >>`）、求根（`sqrt`/`cbrt`/`nth_root`）、`gcd`/`gcd_ext`、`ilog`、
  位谓词，以及 `to_words`/`to_chunks`/`to_bytes`。
- **浮点数 / Decimal**：算术、比较、舍入（`trunc`/`floor`/`ceil`/
  `round`/`fract`）、精度（`precision`/`with_precision`）、超越函数，
  以及转换 `to_decimal`/`to_binary`/`to_rational`/`to_int`。
- **有理数**：算术、`numerator`/`denominator`、舍入、`sqr`/`pow`、`to_float`。
- **复数**：算术、`real`/`imag`、`conj`/`proj`/`norm`/`abs`/`arg`，
  以及超越函数（`sin`/`cos`/`exp`/`ln`/`sqrt`/...）。

超越函数**不会 panic**：定义域错误抛出 `ValueError`，`0/0` 等不定形式抛出
`ZeroDivisionError`，上溢/下溢则产生带符号的无穷或零。它们共享一个模块级的常数缓存
（`dashu.Cache`），因此在不断提高精度的情况下重复调用时会复用预先计算的常数。

模块级的 `math` API 镜像了常见函数——`dashu.sin`、`dashu.sqrt`、
`dashu.exp`、`dashu.gcd`、`dashu.lcm`……——同样接受普通 Python 数值。

## 精度

`FBig` 和 `CBig` 是**二进制**（基数 2）的——精度按**位**计数。`DBig` 是
**十进制**（基数 10）的——精度按**十进制数位**计数（1 个十进制数位
≈ 3.32 位）。

一个数值的精度取决于它的构造方式：

| 输入 | `FBig` / `CBig` | `DBig` |
|---|---|---|
| `int` | 精确（该整数的位长度） | 字面量的数位 |
| `float` / `complex` | 模块默认值（见下文） | 该浮点数的有效十进制数位 |
| `str` | 字符串自身的数位 | 字符串自身的数位 |

**`FBig`/`CBig` 默认值。** `float`/`complex` 输入按模块默认值构造，默认值为
f64 的 53 位。用 `dashu.get_precision()` 读取，用
`dashu.set_precision(bits)` 设置（返回之前的值）；该默认值也适用于整数与浮点数混合的
算术运算。整数输入*不受*影响——`FBig(2)` 仍保持精确的 2 位，因此要套用默认值请写
`FBig(2.0)`。

```python
dashu.set_precision(100)
FBig(1.5).precision()    # 100   (float → 默认值)
FBig(2).precision()      # 2     (int → 精确，不受影响)
FBig(2.0).precision()    # 100
```

**`DBig` 没有模块默认值**，但 `float` 输入本身就会落到一个合理的精度上
——即该浮点数的有效十进制数位（能往返还原该 f64 的最短十进制表示），所以
`DBig(12.345)` 的精度为 5，`DBig(0.1)` 的精度为 1。若要超过这一精度（例如
超越函数需要比输入携带的更多数位），可以传入更长的字符串或调用 `.with_precision`。对于
这两种类型，`.with_precision(n)` 都会覆盖单个数值的精度（`FBig`/`CBig` 单位为位，
`DBig` 单位为十进制数位），而超越函数在该数值的精度下运行：

```python
DBig(2).with_precision(50).ln()                   # 50 个十进制数位
FBig(2.0).with_precision(200).exp().precision()    # 200 位
```

## 格式化

`format()` / f-string 遵循完整的 Python 格式化迷你语言：

```python
format(UBig(255), "#x")                           # '0xff'
format(UBig(10**9), ",")                           # '1,000,000,000'
format(FBig(2).with_precision(200).exp(), ".20e")  # '7.38905609893065022723e+00'
format(DBig("1.5"), ".3e")                         # '1.500e+00'
format(RBig.from_parts(1, 3), ".4f")               # '0.3333'
format(CBig(3.0, 4.0), ".2f")                      # '(3.00+4.00j)'
```

- **整数**委托给 Python 的 `int`，因此所有展示类型都可用（`b`/`o`/`d`/
  `x`/`X`/`c`/`n`），并支持符号、宽度、填充、零填充和分组——且无精度损失。
- **浮点数**（`FBig`/`DBig`）支持 `e`/`E`/`f`/`g`，并支持精度、符号、宽度、对齐、
  填充、零填充和分组。未显式指定精度时，会显示全部有效数位（用 `.6e` 可得到
  固定的 6 位默认值）。
- **`FBig` 的基数为 2**，因此其默认的 `str`/`format` 以及 `'a'`/`'A'` 类型以
  **十六进制**打印——无损、无需基数转换，例如 `str(FBig(1.5)) == '0x3p-1'`。
  十进制展示（`'e'`/`'f'`/`'g'`）会转换为基数 10。`DBig` 以十进制打印。
- **`RBig`** 默认输出精确的分数形式，例如 `str(RBig(1) / 3) == '1/3'`。
