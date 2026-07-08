# Unreleased

### Add
- Upgraded PyO3 from 0.20 to 0.29 (modern `Bound` API, `IntoPyObject`, edition 2024,
  rust-version 1.85). `requires-python` is now `>=3.8`.
- Added `dashu-cmplx` support: a new `CBig` Python type wrapping a bare complex number,
  with arithmetic, `real`/`imag`, `conj`/`proj`/`norm`/`abs`/`arg`, transcendentals
  (`sin`/`cos`/`tan`/`exp`/`ln`/`sqrt`/`powi`/`powf`), and `__complex__`.
- Arithmetic, comparison, and `__bool__` for `FBig`, `DBig`, and `RBig` (previously
  only construction/repr/hash were exposed).
- Integer number theory and bit operations on `UBig`/`IBig`: `sqrt`/`cbrt`/`nth_root`,
  `sqr`/`cubic`, `ilog`, `gcd`/`gcd_ext`, `count_ones`/`trailing_zeros`/…,
  `__floordiv__`/`__divmod__`, and in-place operators.
- Float/rational methods: predicates (`is_zero`/`is_finite`/`is_infinite`), rounding
  (`trunc`/`floor`/`ceil`/`round`/`fract`), `with_precision`/`precision`/`digits`,
  `to_int`, `numerator`/`denominator`, `split_at_point`, `sqr`/`cubic`/`pow`.
- Cross-type conversions: `FBig.to_decimal`/`to_binary`/`to_rational`,
  `RBig.to_float`/`to_decimal`.
- `powi`, `from_parts`, and `ilog` now accept a plain Python `int` (or a dashu
  integer) via the `UniInput` dispatch, so e.g. `FBig(12).powi(300)` works.
- Broadened constructors: `FBig`/`DBig`/`RBig`/`CBig` now accept any Python number
  (int/float/`Decimal`/`Fraction`) in addition to strings.
- A module-level `math` API (`sin`/`cos`/…/`exp`/`ln`/`sqrt`/`gcd`/`lcm`/…) and a
  `dashu.Cache` handle for the global constant cache.
- Transcendentals are now panic-free: domain errors raise `ValueError`,
  `0/0`-style indeterminate forms raise `ZeroDivisionError`, and overflow/underflow
  produce signed infinities/zeros — routed through the panic-free `Context` layer with a
  shared thread-local `ConstCache`.

### Fix
- Removed the `todo!()` panics in `UBig.__mod__`, `IBig.__mod__`, and `IBig.__pow__`.

### Changed
- `FBig(f64)` now constructs at f64's native precision (53 bits) so that subsequent
  transcendental operations (which require precision > 0) are well-defined.

# TODO (still open)
- support pickle through `__reduce__`
- support as much dunder methods as possible:
  https://docs.cython.org/en/latest/src/userguide/special_methods.html#special-methods
- route bare `FBig`/`CBig` arithmetic through the `Context` layer (or guard it) so that
  infinite-input / `0/0` cases raise Python exceptions instead of panicking
  (transcendentals are already panic-free).
