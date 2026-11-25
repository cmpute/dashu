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
7 % UBig(3) == 1          # Python 数值可以位于运算符的任意一侧
UBig(3) - 5 == IBig(-2)   # UBig 相减得到负数时结果为 IBig
```

相等的数值其**哈希值**也相等——无论是在各 dashu 类型之间，还是与 Python 的 `int`、`float`、
`Fraction`、`Decimal` 和 `complex` 之间——因此它们可以混用作 `dict` 的键和 `set` 的成员
（`{1.5: "x"}[FBig(1.5)]`、`5 in {UBig(5)}`）。

除以零和对零取模的行为与 Python 一致：除数为整数或有理数的零时抛出 `ZeroDivisionError`
（对浮点零取模 `%` 也是如此），而用 `/` 除以浮点零得到无穷。`pow(x, e, 0)` 抛出
`ValueError`。`int(x)` 对所有类型都向零截断，`math.floor` / `math.ceil` / `math.trunc` /
`round()` 对 `FBig`、`DBig` 和 `RBig` 都是精确的（`round()` 在恰好一半时远离零舍入）。

整数是值，与 Python 的 `int` 一样：增量赋值（`n += 1`、`n <<= 3`、`n &= m`……）等价于
`n = n + 1`，会把名字绑定到一个新对象，因此指向旧对象的其他引用（别名、`set` 的成员、`dict`
的键）保持原值不变。

- **整数**：`+ - * / // % **`、比较、增量赋值、位操作
  （`& | ^ << >>`）、求根（`sqrt`/`cbrt`/`nth_root`）、`gcd`/`gcd_ext`、`ilog`、
  位谓词，以及 `to_words`/`to_sign_words`/`to_chunks`/`to_bytes`。
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
- **浮点数**（`FBig`/`DBig`）支持 `e`/`E`/`f`/`g`/`G`/`%`，并支持精度、符号、宽度、对齐、
  填充、零填充和分组，排版方式与 Python 的 `float` 相同。未显式指定精度时，会显示全部有效数位
  （用 `.6e` 可得到固定的 6 位默认值）。`g` 与 Python 一样会切换到科学计数法（指数小于 -4
  或不小于精度时）：`format(DBig("123456"), ".2g") == '1.2e+05'`。数位在恰好一半时远离零舍入
  （即 `DBig` 的舍入方式），而 `float` 则舍入到偶数。
- **`FBig` 的基数为 2**，因此其默认的 `str`/`format` 以及 `'a'`/`'A'` 类型以
  **十六进制**打印——无损、无需基数转换，例如 `str(FBig(1.5)) == '0x3p-1'`。
  符号选项和零填充在这里同样适用：`format(FBig(-1.5), "012a") == '-0x000003p-1'`。
  十进制展示（`'e'`/`'f'`/`'g'`）会转换为基数 10；显式指定精度时（以及对于 `'f'`），
  打印的是精确二进制值的数位，例如 `format(FBig(0.1), ".20f") == '0.10000000000000000555'`。
  `DBig` 以十进制打印。
- **`RBig`** 默认输出精确的分数形式，例如 `str(RBig(1) / 3) == '1/3'`；使用浮点展示类型时，
  打印的是精确值的数位，并在所请求的精度处只舍入一次。

## 跨类型比较

`dashu.compare(a, b)`、`dashu.min(a, b)` 和 `dashu.max(a, b)` 可以**精确**地比较任意两个
Python 数字（原生 `int`/`float` 或任意 dashu 类型），其底层由 `num-order` crate 提供——比较
过程绝不会经由有精度损失的原始 `float` 中转。这使得 `compare(UBig(2)**200, 1e60)` 是精确的，
不像朴素的浮点转换：

```python
>>> from dashu import compare, min, max, UBig
>>> compare(UBig(2)**200, 1e60)          # 精确，无精度损失
1
>>> max(UBig(2)**100, 2.0)               # 返回较大的那个操作数，类型保持不变
<UBig 1267650600228229401496703205376 (digits: 1, bits: 101)>
```

`compare` 返回 `-1`、`0` 或 `1`；`min`/`max` 返回两个原始操作数之一（并保留其类型）。
复数没有序关系，因此对复数进行比较会抛出 `TypeError`。

## 第三方集成

以下子模块仅在构建 wheel 时启用了对应的 Cargo feature 时才会被编译进去（默认全部关闭）：

| Feature | 子模块 | 功能 |
|---------|--------|------|
| `serde` | `dashu.serde` | JSON 与二进制（反）序列化 |
| `rand` | `dashu.rand` | 均匀随机数生成 |
| `rkyv` | `dashu.rkyv` | 零拷贝二进制序列化 |
| `zeroize` | *(方法)* | 每种类型上的 `.zeroize()` |

### `dashu.serde`（feature `serde`）

可将任意 dashu 类型序列化为 JSON（`to_json`）或紧凑二进制 postcard（`serialize`）；对应的
反序列化函数把目标类型*类*作为第一个参数：

```python
>>> from dashu import UBig, FBig, serde
>>> s = serde.to_json(FBig(1.5))
>>> s
'"0x3p-1"'
>>> serde.from_json(FBig, s) == FBig(1.5)
True
>>> data = serde.serialize(UBig(2)**100)  # 紧凑二进制
>>> serde.deserialize(UBig, data) == UBig(2)**100
True
```

### `dashu.rand`（feature `rand`）

为每种类型提供均匀生成器。所有函数都接受关键字参数（`ubig(bits=…)`、
`fbig(precision=…)`、`rbig(max_denom_bits=…)`）；浮点/复数生成器默认使用模块配置的精度，
并生成单位区间/单位正方形内的值：

```python
>>> from dashu import rand, UBig, FBig
>>> rand.ubig(bits=128)        # 在 [0, 2^128) 上均匀
<UBig 332087112958751295523159223253302643116 (digits: 1, bits: 128)>
>>> f = rand.fbig()            # 在 [0, 1) 上均匀
>>> 0 <= f < 1
True
>>> rand.ibig(bits=8)          # 带符号，在 (-2^8, 2^8) 上均匀
<IBig 73 (digits: 1, bits: 7)>
```

### `dashu.rkyv`（feature `rkyv`）

零拷贝序列化——反序列化不会复制字节负载。格式是**与架构相关**的：由 `to_bytes` 生成的字节
只在同一台机器上保证可读。`from_bytes` 不校验其输入；字节必须来自对相同类型的 `to_bytes`：

```python
>>> from dashu import UBig, rkyv
>>> data = rkyv.to_bytes(UBig(12345))
>>> rkyv.from_bytes(UBig, data)
<UBig 12345 (digits: 1, bits: 14)>
```

### zeroize（feature `zeroize`）

每种类型都会获得一个 `.zeroize()` 方法，在值被释放之前覆写其底层内存（缓冲区清零）：

```python
>>> v = UBig(12345)
>>> v.zeroize()   # 清空内部缓冲区
>>> v
<UBig 0 (digits: 1, bits: 1)>
```
