# Changelog

## Unreleased

### Add
- Reflected `%`, `divmod`, `&`, `|`, `^` and `**` for `UBig` / `IBig` (`7 % UBig(3)`, `2 ** UBig(10)`), and `UBig` / `IBig` exponents and moduli in `pow`.
- `math.floor`, `math.ceil`, `math.trunc` and `round()` for `RBig` (exact).
- `CBig` arithmetic and comparison accept `UBig` / `IBig`.

### Change
- `UBig` has no `__isub__` any more: `n -= m` is `n = n - m`.
- `pow(UBig, e, negative modulus)` returns an `IBig` when the result is negative instead of raising `ValueError`.
- `CBig(1) == "1"` is `False` (strings are not parsed in comparisons).

### Fix
- `del n[i]` / `del n[a:b]` on a `UBig` returned wrong values when a bit below the deleted range was set; deleting or assigning an empty slice with `stop < start` panicked.
- An integer index that does not fit in an index-sized integer raises `IndexError` instead of `TypeError` (`Words`, `UBig`, `IBig`).
- Division and modulo by zero raise `ZeroDivisionError` instead of a Rust panic (`%`, `//`, `/`, `divmod` on integers and rationals, `%` on floats), `pow(x, e, 0)` raises `ValueError`, and `gcd(0, 0)` / `lcm(0, 0)` / `gcd_ext(0, 0)` return zeros. `nth_root(0)`, `ilog` of zero, `RBig.pow(0, -n)` and `RBig.from_parts(n, 0)` raise instead of panicking.
- `UBig - UBig` returns an `IBig` when the difference is negative instead of panicking.
- In-place operators of `UBig` / `IBig` fall back to the binary operator for operands they cannot take in place (`m = UBig(10); m += -3` raised `TypeError`).
- `int(DBig)` truncates toward zero instead of rounding; `int` / `math.floor` / `math.ceil` / `math.trunc` / `round` of an infinity raise `OverflowError`.
- `auto(float)` uses the default precision, like `FBig(float)` (`auto(0.0)` had precision 0).
- `hash()` agrees with Python's `int`, `float`, `Fraction`, `Decimal` and `complex` for equal values, and across the dashu types.
- `CBig(re, im)` with a complex `re` or `im` is `re + im*j` instead of dropping the imaginary parts.
- `CBig == x` / `CBig != x` return a bool for any object instead of raising, and compare real numbers exactly.
- `format()` of `FBig` / `CBig` with a decimal presentation prints the digits of the exact value (only the first ~15 were right); `format()` of `RBig` rounds once from the exact value.
- `'g'` / `'G'` use the scientific notation as in Python.
- The sign option and zero-padding work for the hexadecimal presentations of `FBig` (`'012a'` gives `-0x000003p-1`).
- Zero-padding with grouping, and the `' '` sign with grouping or zero-padding, are laid out as for `float`.

## 0.6.2

### Change
- Rename `IBig.to_words`/`IBig.from_words` to `to_sign_words`/`from_sign_words` so that the
  sign survives the roundtrip: `to_sign_words()` returns a `(Sign, Words)` pair, and
  `from_sign_words(sign, words)` takes the sign explicitly (the sign of zero is normalized to
  positive, leading zero words in the magnitude are trimmed). `UBig.to_words`/`from_words`
  are unchanged.

### Fix
- The type stub `dashu.pyi` was syntactically invalid: it declared the sign class as `Sign`
  while the runtime module exposes it as `PySign` (the stub now matches the runtime:
  `PySign.Positive` / `PySign.Negative`), and a `zeroize()` declaration had six dots instead
  of an ellipsis.

## 0.6.0

### Add
- **`log10`** — module-level `dashu.log10` plus `FBig`/`DBig` `.log10()` methods.
- **Complex hyperbolic functions** — `CBig` methods `sinh`, `cosh`, `tanh`, `asinh`, `acosh`, `atanh`.
- **CI wheel/sdist build workflow** (`.github/workflows/python-wheels.yml`) — triggered on version
  tags, builds manylinux / macOS universal2 / Windows wheels plus a self-contained sdist.

### Fix
- The bindings now handle `dashu-float`'s new `FpError::ZivRetryLimitExceeded` (a Ziv loop that
  exhausts its retry budget — only possible if a radius bound is wrong) — it raises `RuntimeError`
  instead of failing to compile on a non-exhaustive match.

## 0.5.0

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
- Exact cross-type comparison utilities `dashu.compare` / `dashu.min` / `dashu.max` backed
  by the `num-order` crate (never lossy through a primitive `float`).
- Optional third-party integrations behind Cargo features (all off by default, named
  unversioned — the wheel picks the newest underlying version):
  - `serde` — `dashu.serde` (`to_json`/`from_json` via serde-json, `serialize`/`deserialize`
    via postcard binary),
  - `rand` — `dashu.rand` (`ubig`/`ibig`/`fbig`/`dbig`/`rbig`/`cbig` uniform generators),
  - `rkyv` — `dashu.rkyv` (`to_bytes`/`from_bytes` zero-copy serialization),
  - `zeroize` — a `.zeroize()` method on every type that clears the backing memory.
- The binding now depends on the `dashu` meta-crate (rather than the individual sub-crates)
  and uses the unversioned feature aliases (`rand`, `rkyv`, `serde`, `zeroize`) for the
  optional integrations, so upstream sub-crate updates flow through without touching this
  crate's manifest.
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
