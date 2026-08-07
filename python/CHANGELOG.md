# Changelog

## 0.5.0 (Unreleased)

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
