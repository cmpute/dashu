## Installation

```sh
pip install dashu-rs
```

`dashu-rs` is a standalone wheel on [PyPI](https://pypi.org/project/dashu-rs/) —
no Rust toolchain or compiler is needed. It provides a native `dashu` module:

```python
from dashu import UBig, IBig, RBig, FBig, DBig, CBig
```

## Types

| Type | Backing | Base / rounding |
|------|---------|-----------------|
| `UBig`, `IBig` | arbitrary-precision integer | unsigned / signed |
| `RBig` | arbitrary-precision rational | exact `numerator / denominator` |
| `FBig` | arbitrary-precision float | base 2, round `Zero` |
| `DBig` | arbitrary-precision float | base 10, round `HalfAway` (decimal) |
| `CBig` | arbitrary-precision complex | base 2 |

All types are `Send` + `Sync` (free-threaded-Python compatible). Constructors accept
native Python numbers as well as strings:

```python
UBig(144)              # int
FBig(1.5)              # float (at the module default precision)
DBig("1.23")           # decimal string
RBig(Fraction(1, 3))   # fractions.Fraction
CBig(3.0, 4.0)         # (re, im)
```

## Operations

Arithmetic, comparisons, and `bool()` accept any Python number (cross-type dispatch),
so mixed operands just work:

```python
UBig(2) + 3 == 5
FBig(1.5) * 2 == FBig(3.0)
UBig(17) // 5 == 3
divmod(UBig(17), 5) == (UBig(3), UBig(2))
```

- **Integers**: `+ - * / // % **`, comparisons, in-place ops, bit operations
  (`& | ^ << >>`), roots (`sqrt`/`cbrt`/`nth_root`), `gcd`/`gcd_ext`, `ilog`, bit
  predicates, and `to_words`/`to_chunks`/`to_bytes`.
- **Floats / Decimal**: arithmetic, comparisons, rounding (`trunc`/`floor`/`ceil`/
  `round`/`fract`), precision (`precision`/`with_precision`), transcendentals, and
  conversions `to_decimal`/`to_binary`/`to_rational`/`to_int`.
- **Rational**: arithmetic, `numerator`/`denominator`, rounding, `sqr`/`pow`, `to_float`.
- **Complex**: arithmetic, `real`/`imag`, `conj`/`proj`/`norm`/`abs`/`arg`, and
  transcendentals (`sin`/`cos`/`exp`/`ln`/`sqrt`/...).

Transcendentals are **panic-free**: domain errors raise `ValueError`, indeterminate forms
like `0/0` raise `ZeroDivisionError`, and overflow/underflow yield signed infinities or
zeros. They share a module-wide constant cache (`dashu.Cache`) so repeated calls at
increasing precision reuse the precomputed constants.

A module-level `math` API mirrors the common functions — `dashu.sin`, `dashu.sqrt`,
`dashu.exp`, `dashu.gcd`, `dashu.lcm`, … — and likewise accepts plain Python numbers.

## Precision

`FBig` and `CBig` are **binary** (base 2) — precision is counted in **bits**. `DBig` is
**decimal** (base 10) — precision is counted in **decimal digits** (1 decimal digit
≈ 3.32 bits).

A value's precision comes from how it's built:

| input | `FBig` / `CBig` | `DBig` |
|---|---|---|
| `int` | exact (the integer's bit length) | the literal's digits |
| `float` / `complex` | the module default (below) | the float's significant decimal digits |
| `str` | the string's own digits | the string's own digits |

**`FBig`/`CBig` default.** `float`/`complex` inputs build at the module default, which is
f64's 53 bits. Read it with `dashu.get_precision()` and set it with
`dashu.set_precision(bits)` (returns the previous value); it also applies to arithmetic
that mixes integers with floats. Integer inputs are *not* affected — `FBig(2)` stays exact
at 2 bits, so write `FBig(2.0)` to apply the default.

```python
dashu.set_precision(100)
FBig(1.5).precision()    # 100   (float → default)
FBig(2).precision()      # 2     (int → exact, unaffected)
FBig(2.0).precision()    # 100
```

**`DBig` has no module default**, but a `float` input already lands on a sensible precision
— the float's significant decimal digits (the shortest decimal that round-trips the f64), so
`DBig(12.345)` has precision 5 and `DBig(0.1)` has precision 1. To exceed that (e.g. for
transcendentals, which need more digits than the input carries), pass a longer string or
call `.with_precision`. For both kinds, `.with_precision(n)` overrides one value (bits for
`FBig`/`CBig`, decimal digits for `DBig`), and transcendentals run at the value's precision:

```python
DBig(2).with_precision(50).ln()                   # 50 decimal digits
FBig(2.0).with_precision(200).exp().precision()    # 200 bits
```

## Formatting

`format()` / f-strings honor the full Python format mini-language:

```python
format(UBig(255), "#x")                           # '0xff'
format(UBig(10**9), ",")                           # '1,000,000,000'
format(FBig(2).with_precision(200).exp(), ".20e")  # '7.38905609893065022723e+00'
format(DBig("1.5"), ".3e")                         # '1.500e+00'
format(RBig.from_parts(1, 3), ".4f")               # '0.3333'
format(CBig(3.0, 4.0), ".2f")                      # '(3.00+4.00j)'
```

- **Integers** delegate to Python `int`, so every presentation type works (`b`/`o`/`d`/
  `x`/`X`/`c`/`n`) with sign, width, fill, zero-pad, and grouping — with no loss.
- **Floats** (`FBig`/`DBig`) support `e`/`E`/`f`/`g` with precision, sign, width, align,
  fill, zero-pad, and grouping. With no explicit precision, all significant digits are
  shown (use `.6e` for the fixed 6-digit default).
- **`FBig` is base 2**, so its default `str`/`format` and the `'a'`/`'A'` types print in
  **hexadecimal** — lossless, with no base conversion, e.g. `str(FBig(1.5)) == '0x3p-1'`.
  Decimal presentations (`'e'`/`'f'`/`'g'`) convert to base 10. `DBig` prints in decimal.
- **`RBig`** defaults to the exact fraction, e.g. `str(RBig(1) / 3) == '1/3'`.

## Cross-type comparison

`dashu.compare(a, b)`, `dashu.min(a, b)` and `dashu.max(a, b)` compare any two Python numbers
(native `int`/`float` or any dashu type) *exactly*, backed by the `num-order` crate — the
comparison never round-trips through a lossy primitive `float`. This makes
`compare(UBig(2)**200, 1e60)` exact, unlike a naive float conversion:

```python
>>> from dashu import compare, min, max, UBig
>>> compare(UBig(2)**200, 1e60)          # exact, no precision loss
1
>>> max(UBig(2)**100, 2.0)               # returns the larger operand unchanged
<UBig 1267650600228229401496703205376 (digits: 1, bits: 101)>
```

`compare` returns `-1`, `0` or `1`; `min`/`max` return one of the two original operands
(preserving its type). Complex numbers have no ordering, so comparing one raises `TypeError`.

## Third-party integrations

The following submodules are compiled in only when the wheel is built with the matching Cargo
feature (all off by default):

| Feature | Submodule | Provides |
|---------|-----------|----------|
| `serde` | `dashu.serde` | JSON + binary (de)serialization |
| `rand` | `dashu.rand` | uniform random number generators |
| `rkyv` | `dashu.rkyv` | zero-copy binary serialization |
| `zeroize` | *(methods)* | `.zeroize()` on every type |

### `dashu.serde` (feature `serde`)

Serialize any dashu type to JSON (`to_json`) or compact binary postcard (`serialize`); the
corresponding deserializers take the target type *class* as their first argument:

```python
>>> from dashu import UBig, FBig, serde
>>> s = serde.to_json(FBig(1.5))
>>> s
'"0x3p-1"'
>>> serde.from_json(FBig, s) == FBig(1.5)
True
>>> data = serde.serialize(UBig(2)**100)  # compact binary
>>> serde.deserialize(UBig, data) == UBig(2)**100
True
```

### `dashu.rand` (feature `rand`)

Uniform generators for every type. All functions accept keyword arguments (`ubig(bits=…)`,
`fbig(precision=…)`, `rbig(max_denom_bits=…)`); the float/complex generators default to the
module's configured precision and produce values in the unit interval/square:

```python
>>> from dashu import rand, UBig, FBig
>>> rand.ubig(bits=128)        # uniform in [0, 2^128)
<UBig 332087112958751295523159223253302643116 (digits: 1, bits: 128)>
>>> f = rand.fbig()            # uniform in [0, 1)
>>> 0 <= f < 1
True
>>> rand.ibig(bits=8)          # signed, uniform in (-2^8, 2^8)
<IBig 73 (digits: 1, bits: 7)>
```

### `dashu.rkyv` (feature `rkyv`)

Zero-copy serialization — deserializing does not copy the byte payload. The format is
**architecture-specific**: bytes produced by `to_bytes` are only guaranteed readable on the
same machine. `from_bytes` does not validate its input; the bytes must come from `to_bytes`
on the same type:

```python
>>> from dashu import UBig, rkyv
>>> data = rkyv.to_bytes(UBig(12345))
>>> rkyv.from_bytes(UBig, data)
<UBig 12345 (digits: 1, bits: 14)>
```

### zeroize (feature `zeroize`)

Every type gains a `.zeroize()` method that overwrites the backing memory (buffers become
zero) before the value is dropped:

```python
>>> v = UBig(12345)
>>> v.zeroize()   # clears the internal buffers
>>> v
<UBig 0 (digits: 1, bits: 1)>
```
