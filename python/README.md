# Python Binding

`dashu` brings the dashu arbitrary-precision number types to Python through a native
extension built with [PyO3](https://pyo3.rs). Install the package and import it:

```sh
pip install dashu-rs
```

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
FBig(1.5)              # float (kept at f64's native precision)
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
