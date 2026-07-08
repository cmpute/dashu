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
- All numeric function/method arguments now accept plain Python numbers via the
  `UniInput` dispatch — e.g. `dashu.sqrt(9.0)`, `FBig(2).powi(300)`, `UBig(12).gcd(8)`,
  `UBig(n) += 5`, `RBig.from_parts(1, 3)`. The module-level `math` functions, `powf`,
  `atan2`, `gcd`/`gcd_ext`/`lcm`, `is_multiple_of`/`remove`, in-place ops, `powi`,
  `from_parts`, `ilog`, and `simplest_from_float` all take native int/float.
- `__format__` now honors the Python format mini-language for all types: scientific
  (`e`/`E`), fixed (`f`), general (`g`), integer (`b`/`o`/`d`/`x`/`X`/`c`), with
  sign/width/align/fill/zero-pad/grouping and precision. Float formatting preserves
  the value's arbitrary precision (e.g. `f"{FBig(2).with_precision(200).exp():.20e}"`).
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
