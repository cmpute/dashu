# Changelog

## 0.4.0

First release of **dashu-rs** — the Python binding for
[dashu](https://github.com/cmpute/dashu), a pure-Rust arbitrary-precision number library
(native alternative to GMP + MPFR). A standalone wheel on
[PyPI](https://pypi.org/project/dashu-rs/): `pip install dashu-rs`, no Rust toolchain
needed. Requires Python ≥ 3.8.

Six types cover every number domain, all `Send` + `Sync` (free-threaded-Python compatible):

| Type | Description |
|------|-------------|
| `UBig`, `IBig` | arbitrary-precision integers (unsigned / signed) |
| `RBig` | exact arbitrary-precision rationals |
| `FBig` | arbitrary-precision binary floats (base 2) |
| `DBig` | arbitrary-precision decimal floats (base 10) |
| `CBig` | arbitrary-precision complex numbers (base 2) |

What's included:

- Full arithmetic, comparisons, and `bool()` that accept any native Python number
  (`int`/`float`/`Decimal`/`Fraction`), so mixed operands just work.
- Pythonic construction from numbers or strings, and the complete `format()` mini-language
  (`e`/`f`/`g`/`x`/`b`/… with sign, width, grouping, precision). `FBig` prints losslessly
  in hexadecimal by default.
- Panic-free transcendentals for the float/decimal/complex types
  (`sin`/`cos`/…/`exp`/`ln`/`sqrt`/`pow`/…) — domain errors raise `ValueError`, indeterminate
  forms raise `ZeroDivisionError`, overflow/underflow yields signed infinities/zeros — sharing
  a module-wide constant cache.
- Integer number theory and bit operations: roots, `gcd`/`gcd_ext`/`lcm`, `ilog`, bit
  predicates, and in-place / bitwise ops.
- A module-level `math` API (`dashu.sin`, `dashu.sqrt`, `dashu.gcd`, …) and a configurable
  default precision for `FBig`/`CBig` via `dashu.get_precision()` / `dashu.set_precision()`.
- Example scripts under `python/examples/`: a benchmark against `gmpy2`/`mpmath`/stdlib and
  an arbitrary-precision Mandelbrot deep-zoom.

See `USAGE.md` for the full guide.

# TODO (still open)
- support pickle through `__reduce__`
- support as much dunder methods as possible:
  https://docs.cython.org/en/latest/src/userguide/special_methods.html#special-methods
- route bare `FBig`/`CBig` arithmetic through the `Context` layer (or guard it) so that
  infinite-input / `0/0` cases raise Python exceptions instead of panicking
  (transcendentals are already panic-free).
