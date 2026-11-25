# Changelog

## Unreleased

### Fix
- Keep Rust 1.85 CI compatible with the PostgreSQL integration dependencies by
  pinning `sha2` to 0.11.0; 0.11.1 requires Rust 1.88.
- Fix the short-division fast path of `Context::div`/`Context::inv`: the kernel's ulp
  error was not rescaled onto the value's scale when the window is shifted up
  (`sigma < 0`), understating the classifier's error band by up to 4× — a value past
  the rounding midpoint but inside the true band could be classified to the wrong
  side. The error bound is now amplified by the same shift as the window; the
  previous over-estimate for a right shift (`sigma > 0`) is tightened at the same
  time.
- Fix the short-product fast paths for power-of-two bases whose per-digit bit count
  does not divide a word size (e.g. base 8, 32, 64, 128 on 64-bit words): folding the
  dropped words into the exponent used floor division, silently mis-scaling the result
  whenever the dropped bit count was not a multiple of the base's bit width. The
  remainder bits now stay on the significand as zero padding (like for base 10).
- Fix an exponent overflow in the shared rounding step: rounding a value whose wide
  significand shrinks to the target precision needs `exponent + shift` beyond
  `isize::MAX`, which previously panicked in debug builds and silently wrapped the
  exponent in release builds (reachable e.g. from `Context::mul/sqr/cubic/div/inv/add/sub`
  or `FBig::with_precision` on values with extreme exponents). Such results now saturate
  to the infinity sentinel — the operations above report `FpError::Overflow`, and
  `with_precision` returns the infinity. A trailing-digit fold in `normalize` that
  crosses the sentinel (possible when the rounded significand ends in zero digits and
  the split exponent sits just below it) is canonicalized the same way, so
  `with_precision` can no longer return a non-canonical representation at the
  overflow boundary. The result-finisher now also maps results
  that land directly on a sentinel exponent (through `normalize`'s trailing-digit
  fold, or the add/sub alignment padding) to `FpError::Overflow`/`FpError::Underflow`
  instead of returning a non-canonical representation; this covers the fast paths of
  `mul/sqr/cubic/div/inv` and the shortcuts of `pow`/`powf`/`nth_root`/`hypot`/`fma`.
- Tighten the composed error bound of the `cubic` fast path: when the second window
  extends past the square's word length, the square's shortfall is rescaled onto a
  finer final window than the plain `(n + 2) + (n2 + 2)` sum accounted for.
- Fix `x - x` and `x + (-x)` returning `-0` (outside `mode::Down`) when the operands have exponent `-1` (e.g. `0.5 - 0.5`, `0.3 - 0.3`), and panicking when the operands sit at exponent `isize::MAX`.
- Fix the exact `Repr` `+` / `-` operators: a sum of zeros is `-0` only for `(-0) + (-0)` / `(-0) - (+0)` regardless of operand order, and an exact cancellation is always `+0`.
- Fix `FBig::signum` returning a precision-1 value instead of the exact `ONE` / `ZERO` / `NEG_ONE`, which rounded `x * x.signum()` to one digit for unlimited-precision `x`.
- Fix `FBig / FBig` with an unlimited-precision (or otherwise over-wide) dividend: it failed a debug assertion in debug builds and returned an unrounded or wrongly rounded quotient in release builds.
- Fix `Product` of an empty iterator of `CachedFBig` returning `0` instead of `1`.
- Fix `%` / `Context::rem` returning `+0` instead of `-0` when a negative dividend is an exact multiple of the divisor.
- Fix `%`, `rem_euclid` and `div_rem_euclid` taking time and memory proportional to the exponent gap between the operands (base-2 `%` ran out of memory for a large gap); `rem_euclid` / `div_euclid` now panic on an infinite operand like `%`.
- Fix `ceil` and `split_at_point` returning `+0` instead of `-0` for the zero integral / fractional part of a negative number; `split_at_point` now matches `(trunc(), fract())` exactly and panics on `+inf` as it did on `-inf`.
- Fix `fract()` (and the precision of `trunc()`) for values below one that the magnitude estimate did not recognize: `fract()` reset the precision of values in `[0.01, 1)` to their number of fractional digits.
- Fix `quantize` returning `+0` for a negative value that rounds to zero, and overflowing (debug builds) for a quantum exponent near `isize::MIN`.
- Fix `Context::fma` ignoring the sign of a zero product: `fma(+0, 1, -0)` is now `+0` (outside `mode::Down`), matching `add` / `sub`.
- Fix `Context::mul` tagging an exactly representable product of wide operands as `Inexact(_, NoOp)`.
- Fix the `Rounding` flag of an inexact `add` / `sub` whose operands partly cancel: it is now relative to the truncated exact value like every other operation (e.g. `mode::Away` reported `NoOp` for `4 - 1` rounded to `4`).
- Fix `Context::mul` / `sqr` / `cubic` / `div` (and the operators) reporting `Underflow` / `Overflow` from the operand exponents when the rounded result is representable, e.g. `2 / (3·2^(isize::MIN+1))`, or narrow operands whose exponents sum to `isize::MIN`.
- Fix `FBig + FBig` / `FBig - FBig` returning a nonzero significand at the sentinel exponent instead of saturating like the other operators when the sum overflows the exponent range.
- Fix an overflow panic (debug builds) in `add` / `sub` when the operand exponents are more than `isize::MAX` apart.
- Fix `<<` / `>>` moving a value onto (or past) the sentinel exponents: an out-of-range shift now overflows / underflows like a multiplication by a power of the base.
- Fix `Sum` returning `-0` for a sum of `+0` addends under `mode::Down`.
- Fix `Sum` taking time and memory proportional to the exponent gaps between the addends at limited precision.
- Fix `CachedFBig::ulp` panicking on infinities and (debug builds) on exponents near the ends of the `isize` range; it now delegates to `FBig::ulp`.
- Fix `ulp()` / `ulp_lb()` (and the error bounds) of `-0` being one digit smaller than for `+0`.
- `to_f32`/`to_f64` (on `FBig` and `Repr`) round correctly just below the smallest subnormal (values above half of it no longer become zero under round-to-nearest, and the exact half rounds to the subnormal under `HalfAway`) and just above `MAX` (values below `MAX + ½ulp` round to `MAX` instead of infinity under round-to-nearest).
- Negative zero converts to the integer 0: `FBig::to_int`/`Repr::to_int` return `Exact(0)`, the primitive integer `TryFrom` impls return `Ok(0)`, and `Repr::is_int` returns `true`.
- `FBig::with_base` computes the documented precision exactly (it was one digit short when `B^precision` is a power of the new base, e.g. 128 bits to base 16) and keeps at least one digit, so `with_base`, `to_decimal` and the num-traits `from_f32`/`from_f64` no longer panic for binary floats with 1 to 3 bits of precision.
- Base conversion (`with_base`, `with_base_and_precision`, `to_decimal`, `to_binary`) is correctly rounded, and reports exactness correctly, for every exponent; it used to return wrong digits once the exponent magnitude exceeded 38 (64-bit) at short target precisions.
- Base conversion maps zeros to zeros of the same sign in every base (negative zero used to panic or turn positive), rounds to the requested precision when the base is unchanged, and converts exactly between bases that are powers of a common root (e.g. 4 and 8).
- `AbsOrd::abs_cmp` between `FBig`/`Repr` and `UBig`/`IBig` compares magnitudes when either side is negative.
- `NumOrd` between `FBig`/`Repr` and `f32`/`f64`: a zero no longer ranks above positive floats below 1/2, and exponents beyond `isize::MAX / bit_len(base)` no longer overflow (panic in debug builds, wrong order in release builds).
- `NumOrd` between two floats of the same base no longer tries to allocate the full power of the base for large exponents.
- `EstimatedLog2::log2_bounds` for `FBig`/`Repr` stays valid for exponents beyond 2^24.
- num-traits: `Signed::abs_sub` returns the positive difference (zero when `self <= other`), and the integer `ToPrimitive` conversions return `None` for infinities instead of panicking.
- rand: `UniformFBig` (and rand's `Uniform`/`gen_range` for `FBig`/`DBig`) always samples inside the requested range; bounds of unlimited precision (such as `FBig::ZERO..FBig::ONE`) sample at the default precision of the builtin distributions instead of returning only the lower bound.
- rand 0.9/0.10: `Uniform::new`/`new_inclusive` for `FBig`/`DBig` return `Error::EmptyRange`/`Error::NonFinite` instead of accepting an empty range or panicking.
- `FBig::from_parts_const` infers the same precision as `from_parts` (trailing zero digits count, and significands at the top of the `DoubleWord` range get their last digit), and `Repr::new_const` saturates the exponent like `Repr::new`.
- `ErrorBounds::error_bounds`: the round-to-nearest bounds no longer exceed half an ulp in odd bases (which made transcendental functions misround there, e.g. `ln(1296)` in base 3), `mode::Down`/`mode::Up` return zero bounds for unlimited-precision values instead of panicking, `mode::HalfEven` includes the ties of floats with an even last digit (it was the other way round), and the bound facing zero is narrowed below powers of the base.
- Fix the correct-rounding certification (Ziv loop) next to a power of the base: `ErrorBounds::error_bounds` used a full ulp on the toward-zero side of `±B^k`, where the neighbour is in the finer binade below, so a value just below a power of the base could be returned as that power (`exp`/`powi` returning `1` for `0.99949…` at 3 digits, `powf(x, 0.5)` returning `8` for `3.9993` at 1 bit, `15^-16` at 1 bit). The ties-to-even inclusion flags of `HalfEven::error_bounds` now use the significand parity at the precision.
- Fix the `Exact`/`Inexact` flag (and the rounding direction) reported by the Ziv-backed functions (`exp`, `ln`, `log2`, `log10`, `powi`, `powf`, `hypot`, …): it described the working approximation rather than the true value, so an inexact result could be flagged `Exact` (`ln(55)` at 2 bits, `183^-8` at 53 bits, `hypot(1, 2^40)`) or carry the wrong direction, and under `Zero` a result above the true value could be returned.
- Fix `powi` failing with `ZivRetryLimitExceeded` (a panic through `FBig::powi`) when the exact power is a rounding tie (`5^2` at 1 decimal digit under `HalfEven`).
- Fix `sqrt` in non-binary bases: an exact root that is a rounding tie did not terminate (`sqrt(225)` at 1 decimal digit), a perfect square was reported `Inexact` or mis-rounded in base 4 and did not terminate in base 9 (`sqrt(36)`, `sqrt(441)`), and an inexact root could be flagged `Exact`. `sqrt` is now a single correctly rounded step in every base.
- Fix `sqrt` of a value with an exponent next to `isize::MIN`/`isize::MAX` (debug overflow panic, wrong exponent in release builds).
- Fix `hypot` for operands far from the middle of the exponent range: spurious `Underflow`/zero or a wrong exponent near `B^(isize::MIN/2)`, spurious `Overflow` near `B^(isize::MAX/2)` when a significand is wider than the precision, and a running time proportional to the exponent gap under directed rounding modes. `hypot` is now computed from the exact sum of squares.
- Fix `exp(0)`, `exp_m1(±0)`, `exp_m1(-inf)`, `ln(1)`, `ln_1p(±0)`, `log2(1)`, `log10(1)`, `powi(x, 0)`, `powf(x, ±0)`, `powf(1, y)` and `powf(±0, y)` returning a result with precision 0 (unlimited) instead of the context's precision, which made the next operation on the result panic.
- Fix `cbrt`, `nth_root` and `hypot` panicking on an unlimited-precision zero (`FBig::try_from(0.0)`, `DBig::ZERO`).
- Fix `log2` not terminating under directed rounding for a negative power of two in a non-binary base (`log2(0.5)` in base 10), and `log10` likewise for a power of ten in a base with other prime factors.
- Fix `ln`/`log2`/`log10` panicking on a value with exponent `isize::MIN` (the value a directed-mode underflow of `exp` saturates to).
- Fix `powf` not terminating under directed rounding (including `FBig`'s default `Zero`) when the result is exactly representable (`powf(4, 0.5)`, `powf(0.25, 1.5)`); such results are now returned as `Exact`.
- Fix `exp` for arguments around `±isize::MAX·ln(B)`: a spurious `Underflow` at low precision, no termination or a spurious `Overflow` for a result within a few hundred digits of either end of the exponent range, and a running time proportional to the argument's exponent before an overflow was reported (also through `sinh`/`cosh`).
- Fix `powf` returning a value up to `2^260` times too large for a result just above the smallest exponent.
- Fix `powi`: a spurious `Underflow`/`Overflow` for an exponent of `2^63` or more (`0.999^(2^63)`) and for a negative exponent with a result just above the smallest exponent; no termination for a base of `-1` with an exponent past `2^64` under `Down`/`Up`; and an out-of-memory panic for a base near one with an exponent past `2^64`.
- Fix `ConstCache::ln_base` for a base other than 2, 10 or a power of two returning a value with `precision + 2` digits and several ulps of error; it is now the correctly rounded `ln(B)`.
- `FBig::from_str`: a zero literal ignores its scale. `0e-1` parsed as `-0` and
  `0e<isize::MAX>` / `0e<isize::MIN>` as the infinities; only a leading `-` gives `-0` now.
- `FBig::from_str`: only one sign is accepted, in front of the integral part. `0.+1` parsed
  as 0.01, and `++1`, `-+1`, `0x+1` were accepted.
- `FBig::from_str`: `0x.` (a hex prefix and a dot without any digit) is rejected with
  `ParseError::NoDigits`.
- `FBig::from_str`: a nonzero literal whose exponent overflows or lands on the reserved
  exponents `isize::MIN` / `isize::MAX` is rejected with `ParseError::InvalidDigit`. It used to
  panic in debug builds, wrap around in release builds, or return a value on which arithmetic
  panics.
- `Display` with a width and no precision padded integers one character short
  (`format!("{:6}", 123)` gave 5 characters).
- Scientific formatting (`{:e}`, `{:b}`, `{:o}`, `{:x}`) with a precision printed one
  fractional digit too many when rounding carried into a new leading digit (`95` at `{:.0e}`
  gave `1.0e2`).
- Scientific formatting ignored the `0` flag when an alignment was given (`{:<08e}`).
- `Display` with a precision no longer runs out of memory (or takes time proportional to the
  exponent) for values with a very large negative exponent.
- serde: the human-readable string now has exactly as many digits as the precision, so the
  precision round-trips also when the positional form has more digits than the precision
  (`10` at precision 1, `0.5` at precision 1) and for digits above 9 in bases above 10. Such
  values are written with an explicit scale (`"1@1"`, `"5@-1"`).
- serde: the `inf` / `-inf` strings written for infinities can be deserialized.
- serde: the struct form can be deserialized from formats that provide owned or transient
  field names (e.g. CBOR).
- postgres: negative zero is stored as zero instead of panicking in `to_sql` / `from_sql`.
- postgres: `dscale` always covers the fractional digits of the value. A result carrying a
  guard digit (`11 - 0.1` at precision 2) was sent with `dscale = 0`, so the server stored 10.
- postgres: `to_sql` accepts every value whose weight fits the NUMERIC format (`1e32768` was
  refused) and no longer overflows for precisions above 32767.
- postgres-types: `from_sql` returns an error instead of panicking on an unknown sign word, a
  truncated value or an out-of-range digit.
- rkyv (0.7 and 0.8): infinities round-trip. `INFINITY` used to come back as `-0` and
  `NEG_INFINITY` as `+0`. An exponent that does not fit the 32-bit archived exponent is now
  refused (an error with rkyv 0.8, a panic with rkyv 0.7) instead of being truncated.
- `atan2_unit` returned the two negative-`y` diagonals swapped (`atan2_unit(-1, 1, 360)` was
  `-135`).
- `asin_unit` / `acos_unit` treated 2/3 and 1/3 (base 3), 2/5 (base 5), … as the exact `±1/2`
  row in odd bases.
- `atan(±inf)` and `atan2` at infinite arguments or `x = 0` are correctly rounded (they scaled
  an already rounded π) and report `Inexact`.
- The trigonometric and hyperbolic functions no longer panic on an exact zero carrying
  unlimited precision (`FBig::ZERO.sin()`); `acos(1)` and `acosh(1)` likewise.
- `sin` / `cos` / `tan` no longer return an exact `0` for an argument next to a multiple of
  π/2 that has more digits than the context.
- `acos` / `asin` next to `±1` and `acosh` next to `1` no longer trip a debug assertion when
  the argument is wider than the working precision; the internal `Ball::sqrt` now covers the
  lower end of its input ball.
- `sin`, `cos`, `tan`, `asin`, `atan`, `sinh`, `cosh`, `tanh`, `asinh`, `atanh` (and `cos_unit`,
  `cosh_pi`, `acos_unit`, `atan_unit`, `atan2_unit` next to their exact values) of a tiny
  argument are rounded directly: they report `Inexact` with the correct `Rounding` (not
  `Exact`), take microseconds under the directed modes at any exponent, and no longer underflow
  to `0` for exponents below `isize::MIN / 2`.
- The transcendental functions report the `Rounding` flag of the true result, not of the
  working approximation, when the two fall on opposite sides of the returned value.
- `tanh` at saturation (and `exp_m1` at its `-1` saturation) report the `Rounding` adjustment
  relative to the truncated significand, consistently with the unsaturated path.
- `sinh_pi` / `cosh_pi` / `sinh_cosh_pi` no longer run out of memory for a large argument at a
  low precision, and `sinh` / `cosh` and their ×π variants report an astronomically large
  argument as an overflow immediately.
- `asinh` / `acosh` no longer run out of memory for huge arguments (from about `2^(2^40)`).
- serde: in bases above 23 the finite number whose digits spell `inf` (24171 in base 36) is written as `inf@0` by the human-readable serializers of `Repr` and `FBig`; it was written as `inf`, which reads back as an infinity.
- Fix `Repr::new` and `FBig::from_parts` reading a zero significand at exponent `-1`, `isize::MAX` or `isize::MIN` as `-0` / `+inf` / `-inf`: zero is `+0` at every exponent.
- Fix a debug-build overflow panic in `NumHash` for a value with exponent `isize::MIN`.
- Fix `quantize` returning a nonzero significand at the sentinel exponent `isize::MAX` (a factor of the base too small when the carry went beyond it): a result outside the exponent range now saturates like an overflowing multiplication.
- Fix `Context::fma` / `FBig::fma` reporting the `Underflow` / `Overflow` of the product `a·b` when `c + a·b` is representable (`fma` returned `0` for `3 + 2^(isize::MIN - 2)`): the exponent range is checked on the rounded result.
- Fix `+`, `-`, `*` between an unlimited-precision `FBig` (or `CachedFBig`) and an integer rounding the result to the number of digits of the integer (`12.34 + 1u8` was `10`): the result stays exact.
- Fix `Context::rem` panicking on a zero divisor: it returns `Err(FpError::Indeterminate)`.
- Fix `%`, `rem_euclid`, `div_euclid` and `Context::rem` running out of memory for a zero dividend and a divisor with a huge exponent.
- Fix `Sum` returning a nonzero significand at the sentinel exponent, or the wrong saturation endpoint, when the sum overflows the exponent range: it now saturates like `+`, and partial sums may leave the range.
- Fix `<< 0` / `>> 0` changing a value whose exponent is `isize::MAX` or `isize::MIN` (to an infinity, the largest finite value, or zero).
- Fix `cbrt`/`nth_root` of a value with an exponent next to `isize::MIN`/`isize::MAX` (debug overflow panic, wrong exponent or significand in release builds).
- Fix an even `nth_root` of `-0` reporting an out-of-domain error (a panic through `FBig::nth_root`); it is `+0`, as IEEE 754 defines rootn (`sqrt(-0)` remains `-0`).
- Fix the rounding direction and flag of `powi` under `Up`/`Down` for a negative base with an odd exponent above `2^64`.
- Fix `powf` panicking with "precision cannot be 0" for a trivially exact power (zero or unit exponent, zero or unit base) of unlimited-precision operands such as `FBig::try_from(0.0)`.
- Fix `log10` not terminating under directed rounding for a power of ten whose logarithm exceeds `isize` (`log10(100^e)` with `|e| > isize::MAX / 2`).
- `to_f32` / `to_f64` no longer panic for a decimal or hexadecimal number whose exponent does not fit an `isize` once scaled to base 2 (`1e3000000000000000000`); they saturate like other out-of-range values.
- The scientific formats (`{:e}`, `{:E}`, `{:b}`, `{:o}`, `{:x}`) print the right exponent for a multi-digit significand next to `isize::MAX` instead of overflowing (a panic in debug builds, a negative exponent in release builds).
- `{:.0}` prints `-0` for a negative zero under the `Up` and `Away` rounding modes (it printed `-1`).
- `FBig::from_str` accepts a literal whose fractional trailing zeros take `cc - len(bbb)` below `isize::MIN` while the value itself is in range (`0.100e-9223372036854775806`).
- `EstimatedLog2::log2_bounds` / `log2_est` return `+inf` for the infinities (they returned the `-inf` of a zero).
- `FBig::ulp` / `ulp_lb` no longer return a too small value for a multi-digit significand next to the largest exponent.
- `FBig::with_precision` (and `with_base_and_precision` to the same base) no longer returns an infinity under a rounding mode that rounds toward zero on that side: a result that overflows saturates the way an overflowing operation does.
- serde: a nonzero number at a reserved exponent (`FBig::from_parts(2, isize::MAX - 1)`, or the saturated result of an overflow under an inward rounding mode) round-trips through the human-readable formats; it was written as a literal that the parser refuses.
- serde: the struct form of `FBig` is rejected when the significand has more digits than `precision + 1`.
- rkyv: a precision above `u32::MAX` is refused instead of being truncated to its low 32 bits (an error with rkyv 0.8, a panic with rkyv 0.7).
- postgres: `ToSql` no longer refuses numbers with more than 16383 significant digits; a number too large for the format is `ConversionError::OutOfBounds`.
- postgres: the encoded `dscale` never exceeds 16383, the largest scale the server accepts.
- rand: `UniformFBig` (and rand's `Uniform` / `gen_range`) never returns a negative zero.
- rand: the cost of a `UniformFBig` sample no longer grows with the distance between the exponents of the bounds.
- num-order: `NumOrd` between floats of different bases no longer takes time and memory proportional to the exponents.
- In an odd base, the transcendental functions no longer fail to return for a result within
  about `B^-129` ulps of the midpoint of two floats under the round-to-nearest modes
  (`acos_unit(3^-140, 2)`, `atan_unit(3^140, 2)`, `asinh(3^-100)` at 255 digits); the inverse
  ×u functions next to a half-integer axis are rounded directly.
- The trigonometric and hyperbolic functions report the `Rounding` flag of the true result
  however close it lies to the returned value: `cos_pi(2 - 1e-40)`, `sin_pi(0.5 + 1e-40)`,
  `tan_pi(0.25 + 1e-100)` or `sin` of π/2 to 100 digits are no longer `Exact(1)` at a lower
  precision.
- `sin_unit` / `tan_unit` (and `sin_pi`, `tan_pi`), `asin_unit`, `atan_unit` and `sinh_pi` of
  an argument near the bottom of the exponent range return the representable result, or an
  underflow with the sign of the result; `sin_pi` / `tan_pi` reported a positive underflow for
  a negative argument, and `sinh_pi` / `sinh_cosh_pi` reported an overflow (`±inf`).
- `atan` of an argument above about `B^(isize::MAX/3)`, and `atan2` of a ratio that large (or
  beyond the exponent range), return the rounded `±π/2` (or `±π`) instead of an underflow.
- `atanh(±1)` returns `±inf` at unlimited precision instead of panicking, and the docs no
  longer claim a panic at `|x| = 1`.
- The exact rows of `sin_unit`, `cos_unit`, `sin_cos_unit`, `tan_unit`, `asin_unit`,
  `acos_unit`, `atan_unit` and `atan2_unit` (`0`, `±1`, and the table ratios that have a finite
  expansion in the base) no longer panic at unlimited precision (`FBig::ONE.acos_unit(360)`).
- `FBig::from_parts_const` applies `min_precision` to a zero significand (it returned a zero of unlimited precision).

### Change
- Apply signs directly, avoid redundant absolute-value temporaries, and share quotient/remainder
  work during trigonometric argument reduction; preserve the existing rounding boundaries.
- Organize short-product and short-quotient checks into fixed regressions in
  `tests/high.rs` and bounded property tests in `tests/high_prop.rs`, sharing exact
  oracles and assertions through `tests/helpers.rs`. Explicitly cover precision
  and operand-shape combinations, replacing duplicate ordinary-input sweeps. Check
  division and reciprocal against exact integer quotient/remainder rounding, and
  retain small internal path-selection tests. Size short-quotient fuzz inputs for
  each radix.
- Share short-window sizing and deterministic arithmetic fixtures, simplify short-division
  operand preparation, and keep rounding-result helpers together. Clarify and strengthen
  exact-boundary fallback tests, including low-precision and signed operands.
- Share the exponent-saturation checks between the short-quotient and short-product
  paths through the borrowed representation validator.
- Share short-product scaling and rounding-result validation, reuse the common
  signed-zero/overflow constructor in addition, and avoid cloning the retained
  significand during short-product rounding.
- `Context::div` and `Context::inv` decide the rounding from a certified high window of
  the quotient (the new `dashu-int::high::div_high` kernel) when the operands are large
  relative to the target precision, instead of forming the full exact quotient. The
  quotient window carries a two-sided error bound, so unlike the product fast paths
  every boundary case — an exact division, a midpoint tie — falls back to the exact
  path, keeping results, rounding flags and exactness indicators identical. Division
  and reciprocal at medium and large precisions do roughly half to two thirds of the
  exact division's work; series-based transcendental evaluations that divide benefit
  automatically. No public API change.
- `Context::mul`, `Context::sqr` and `Context::cubic` now decide the rounding from a
  certified high window of the product (the new `dashu-int::high` kernels) when the
  operands carry far more digits than the target precision, instead of computing the
  full exact product first. Results, rounding flags and exactness indicators are
  unchanged — whenever the window's one-sided error bound cannot certify the rounding
  (a measure-ε neighborhood of a rounding boundary), the operation falls back to the
  exact product. For equal-precision operands roughly half of the multiplication work
  is skipped at medium and large precisions; unbalanced operands (one much wider than
  the target precision) gain much more. Transcendental functions and `CachedFBig`
  benefit automatically through these kernels. No public API change.
- An overflow that saturates to `±inf` (nearest and outward modes) no longer panics at unlimited precision; only the inward modes, which need the largest finite value, still require a limited precision.
- Document that `rem_euclid` rounds its result to the operands' precision (and can therefore return `|rhs|`), when a subtraction result carries the guard digit, the precision rule of `fract()`, and what `ulp()` means next to a power of the base.
- `UniformFBig::new` panics for an empty range (`low >= high`; it used to accept `low == high` and return the excluded bound), `UniformFBig::new`/`new_inclusive` panic for infinite bounds, and the sampling precision is raised to the digit count of the bounds when the requested precision is smaller.
- `Uniform01::new_open` panics for a precision of 0 (it used to loop forever in base 2).
- `FBig::with_base`/`with_base_and_precision` on an unlimited-precision integer (non-negative exponent) convert losslessly to any base instead of panicking.
- `exp`, `exp_m1` and `ln_1p` of an argument too small to be visible next to the result's leading term are rounded directly (no series evaluation), in time independent of the argument's exponent.
- **(breaking, encoding)** serde: in human-readable formats, an `FBig` whose precision cannot
  be expressed by a literal (unlimited precision, more significant digits than the precision,
  or an infinity with a nonzero precision) is serialized as the
  `(significand, exponent, precision)` struct instead of a string, so that the precision
  round-trips (`DBig::ONE` came back with precision 1). The deserializer accepts both forms.
- `Repr::new(significand, exponent)` no longer rebuilds `-0` or an infinity from the parts returned by `Repr::into_parts`; use `Repr::neg_zero`, `Repr::infinity` and `Repr::neg_infinity`.
- Dividing an unlimited-precision `FBig` by an integer (or an integer by it) panics like any other unlimited-precision division, instead of rounding the quotient to the number of digits of the integer.
- Document that `%` panics on a zero divisor, and that the guard digit of a subtraction result is rounded away by the next operation (also `x + 0` or a one-element `Sum`), but by `with_precision` only for a smaller precision.
- `powf` on a zero base now keeps the base's sign for an odd integer exponent, as `powi` and IEEE 754 `pow` do: `powf(-0, 3) = -0` and `powf(-0, -3) = -inf` (previously `+0` and `+inf`).
- Infinities honor the width, fill, alignment, `+` and `0` flags in `Display` and the scientific formats (`{:+8}` gives `    +inf`).
- serde: the human-readable form of a `Repr` with a large exponent is a literal with a scale (`"1@1000"`) instead of the positional form; a nonzero number at a reserved exponent is written as a struct, for `Repr` as well as `FBig`. Both forms are read.
- postgres: a number that needs more than 16383 fractional digits is `ConversionError::OutOfBounds` (the limit was 65535), and the padding implied by a larger precision stops at 16383.
- `FBig::atan2` / `FBig::atan2_unit` (and the `CachedFBig` mirrors) return the result at the
  higher of the two operands' precisions, like the binary arithmetic operations; they used the
  receiver's precision only.

## 0.6.2

### Change
- **Behavior change** (released as a patch: no API surface changes — the fixed
  nearest-ties-away rule was the reported defect): the remainder operator (`%` / `RemAssign` /
  `Context::rem`) now rounds the quotient with the rounding mode attached to the type
  (issues #110, #111). `DBig`/`FBig<HalfAway>` results are
  unchanged; `FBig<Zero>` (the binary default) now returns the truncated,
  dividend-signed remainder — fmod semantics, so `fbig!(0xF) % fbig!(0xA)` is `+5`;
  `FBig<HalfEven>` now implements the IEEE 754 `remainder` rule (ties to even); `Down`/
  `Up` round the quotient floor/ceil-style (Python-style `%` and its mirror). The mode
  also continues to round the remainder value to the context precision as before, and
  `RemEuclid` is unchanged. The kernel reduces modulo `2·|rhs|` on its fast path, so the
  `HalfEven` tie parity falls out of the same residue with no extra pass.
- Clarified the docstrings around rounding-to-integer (issue #110): `FBig::round` now
  states that it always rounds half away from zero like `f64::round()`, regardless of the
  rounding mode attached to the type, and points to the mode-aware alternatives
  (`to_int()`, `quantize(0)`, `with_precision()`); `to_int()` and `quantize()` cross-link
  back.
- Specialized the ×2 pattern out of `Mag`'s power-of-two scaling: the propagation folds'
  `mul_pow2(1)` cross-terms now call a dedicated `const #[inline] mul2()` (a normalized
  significand never renormalizes on a single doubling, so it is a bare exponent bump,
  inlinable cross-crate), and every other caller migrated to the general scaling at the
  internal `i128` exponent width — now the one `mul_pow2` (the `isize`-width wrapper and
  the `_i128` suffix are gone). The single-user kernels `add_up`/`sub_impl` are inlined
  into `add`/`sub_down`, and `exp_upper` keeps its halving exponent in `i128` end to end
  (no `usize` round-trip). The raw-parts assembler became an idiomatic private
  `Mag::new(man, exp)` associated constructor (absorbing `from_pow2_stored`'s duplicate
  saturation check), and the too-large-significand round-up normalizer likewise became
  `Mag::new_round_up(raw, exp)`; `new` debug-asserts the normalization contract, so the
  test suite validates it at every construction path. No observable behavior change.
- Pruned the internal `Mag` radius type down to the subset the float/complex error
  propagation actually uses: removed the round-up `Mag::sub` (dead since the complex log
  bracket switched to `sub_down`; its helper `from_double_rounded` collapsed to a
  truncating `from_double_truncated`), replaced the generic binary-exponentiation
  `Mag::pow` with a plain squaring helper (`exp_upper` only ever raises to `2^j`), dropped
  the unused `Hash` derive, and made `from_pow2` module-private.
  Bit-identical results on every live path (`Mag` stays `#[doc(hidden)]`, so no public API
  changes).

### Fix
- `tanh` no longer stalls on large `|x|`: the value saturates at `sign(x)·(1 − 2e^{−2|x|})`,
  whose sub-half-ulp residual makes the Ziv working-precision mid collapse onto exactly `±1`
  — the boundary of a one-sided directed-rounding preimage — so the containment test could
  never certify and the loop climbed toward a working precision of `~2|x|/ln 2` (an effective
  hang at `|x| = 10⁵`, minutes at `|x| ≈ 1.7·10⁶`; the positive side's overflow shortcut was
  unreachable for the same reason). Inputs beyond `|x| > (p·ln B + ln 4)/2` (where the residual
  is provably below half an ulp) now short-circuit to the mode-aware `±1` endpoint before the
  loop — the same gate `exp_m1` already had for its extreme-negative case (`exp_extreme_negative`,
  generalized to both signs as the shared `near_one_endpoint`). Directed modes return the
  ulp-inside neighbor (`Up`/`Zero` just above `−1`, `Down`/`Zero` just below `+1`) instead of
  the mode-blind `+1`.

## 0.6.1

### Add
- `#[doc(hidden)]` lockstep-shared re-exports of the internal error-propagation substrate
  (`Ball`, `Mag`, `ulp_mag`) at the crate root, consumed by `dashu-cmplx`'s complex ball
  (`CBall`) for its transcendental radius migration. Deliberately hidden: excluded from the
  semver and stability guarantees, consumed only by this repository's crates (released
  together). No documented API changes.
- ×u trigonometric functions `sin_unit`/`cos_unit`/`sin_cos_unit`/`tan_unit` (of `2π·x/u`, the
  argument in units of the full turn divided by `u` — e.g. `u = 360` gives degrees) and their
  inverses `asin_unit`/`acos_unit`/`atan_unit`/`atan2_unit` (of `u·θ/(2π)`), on `Context`,
  `FBig` and `CachedFBig` (the `u` parameter is `usize`; the forward family takes
  `Err(OutOfDomain)` at `u = 0`, the inverse family returns the `u → 0` signed-zero limit).
  Unlike the radian variants, the argument reduces *exactly* mod u in integer arithmetic, so
  the accuracy is independent of the magnitude of the input; arguments where `12x/u` (resp.
  `8x/u` for the tangent) is an integer resolve exactly — quarters (`0`/`±1`), the sine/cosine
  sixths (`±1/2`), the tangent eighths (`±1`) and the tangent poles (`Err(Indeterminate)`,
  where the one-sided limits `+∞`/`−∞` disagree in sign). The inverse family resolves the
  axis/diagonal angles to exact `k·u/8` values (including the finite `y = ±x` diagonals, which
  an exactly-representable one-sided directed-rounding preimage would leave uncertifiable).
- ×π trigonometric functions `sin_pi`/`cos_pi`/`sin_cos_pi`/`tan_pi` (of `x·π`), on `Context`,
  `FBig` and `CachedFBig` — now thin wrappers over the ×u family at `u = 2`.
- ×π hyperbolic functions `sinh_pi`/`cosh_pi`/`sinh_cosh_pi` (of `x·π`), on `Context`, `FBig` and
  `CachedFBig` — the argument ball is built from the shared cached π; a direct series handles
  `|πx| ≤ 1`, the exponential composition the rest.

### Change
- **(internal, `tuning`) the per-attempt Ziv radius trace is now a settable hook** instead of an
  unconditional `eprintln!`: `ziv_set_trace_hook(Some(|guard, radius| …))` (deliberately
  `#[doc(hidden)]` — a profiling hook, not a stable API promise) installs a printer, and the
  default `None` costs one `Cell` read per attempt. The unconditional print cost ~3–4 µs per
  attempt, which silently poisoned any *timing* run built with `tuning` (reading as a 4–5×
  regression on sub-microsecond Ziv cases).
- **`-0` renders with its sign**, as `f64`'s does: `format!("{}", -0)` is now `"-0"` (was `"0"`)
  and `format!("{:e}", -0)` is `"-0e0"` (was `"0e0"`). The parse side moves with it — `"-0"`,
  `"-0.0"` and `"-0e0"` now produce *negative* zero (all three produced `+0`) — so a signed zero
  survives its own `Display`/`FromStr` round-trip. Neither direction could express the sign
  before, which is why the pair is fixed together. Nothing numeric changes (`±0` compare equal)
  and `{:+}` still prints `"+0"` for the positive zero.
- **(internal) the Ziv error radius is now a value-space `Mag` instead of an exact-integer
  ulp count** (`float/src/mag.rs`, `float/src/ball.rs`; both `pub(crate)`). Every `+`/`-`/`*`/`/`
  in a transcendental's algorithm is itself correctly rounded, so the radius composes through
  plain interval algebra over the operation's operands — no ulp-domain shift formulas, no
  per-ball precision state, and no `IBig` error bookkeeping. Results are unchanged for every
  input (validated bit-exact against MPFR/MPC); this is the mechanism the 0.6.0 note below
  described as "an exact-integer error count", replaced.
- **(internal) the operand error of the *input* to a transcendental is now part of the
  certified radius** rather than a per-function hand estimate, so a result that sits close to
  a rounding boundary is retried instead of being rounded the wrong way. See the two `Fix`
  entries below, which are the user-visible symptoms of the old per-function estimates.
- **(internal) faster digit bookkeeping in division**: `digit_len` on a power-of-two base is
  now a plain bit length (`ilog` allocated a `B^log` buffer only to discard it). No perf
  regression from the fixes below: `FBig / FBig` at parity or faster at all benched
  precisions, `DBig / DBig` ~10% faster at 10³–10⁴ digits (+8% at 10 digits, the cost of the
  now-correct wide-quotient rounding), `nth_root` 10–45% faster.
- **(internal) the ×u reduction's modular exponentiation delegates to `dashu-int`**
  (`ConstDivisor::reduce` + `Reduced::pow`) instead of a local binary-exponentiation loop
  over `rem_euclid` — the small `2k·u` modulus rides the single/double-word fast paths. The
  log₂ helpers (`log2_base`'s fixed-point bracket, the conservative `log2_u_bs_lb` bound)
  moved to `utils.rs`, next to the digit helpers that share their role.

### Fix
- **the internal radius type `Mag` widens its exponent field to `i128`** (was `isize`): a
  result in `10^(2.8e18)..10^(9.2e18)` — e.g. `sinh_pi(3.5e18)`, finite and representable as
  an `FBig` — used to saturate its *radius* bookkeeping to `+∞` (`Mag`'s ceiling was
  `2^isize::MAX ≈ 10^2.8e18`, below the value range), which no Ziv attempt can certify; the
  retry loop then ran its exponential guard growth for hours before erroring. The exponent
  field now covers every representable value, so such results certify in one retry.
  `fuzz/tests/float_trig_random.rs`'s `fbig_sinh_cosh_pi_fuzz` shards (×π hyperbolic at high
  precision) were the visible symptom: they hung for hours at the 500/1000-bit sweep widths.
- **`asin` of an argument that merely *rounds* onto `±1` returned the endpoint** — `asin(1 − 10⁻¹⁵⁵)`
  at 100 digits gave exactly `π/2` where the true value is `π/2 − 1.4·10⁻⁷⁸` (~10⁹ ulps off), and
  the Ziv loop certified it because the endpoint branch reports only a few ulps of radius. The
  argument is not the endpoint: `√(1−x²)` merely *collapsed* onto zero at the work precision, and
  `Ball::sqrt` returned that zero with the radius dropped. A rounded-to-zero root now carries the
  whole-line radius instead (only an *exact* zero stays exact), so the `±1` branch is taken for an
  exact `±1` argument alone and everything else retries at a higher guard, where the argument is
  representable and the general `atan(x/√(1−x²))` path computes the true `√(2δ)` offset. The same
  applies to the ×u inverses (`asin_unit`/`acos_unit`). The domain checks and `±1` rows now compare
  the raw `Repr` explicitly rather than relying on `FBig::ONE` being unlimited-precision for the
  same effect (behaviourally identical; `repr_cmp_same_base`'s precision argument and the poles'
  exactness requirement are documented on it).
- **`ln_1p` of an argument that rounds onto the pole at `−1` panicked** (debug: `attempt to
  subtract with overflow`; release: a wrapped nonsense exponent) — the `1 + x` cancellation left
  `ln_compute` with a zero midpoint, where `log2_bounds` saturates to `−∞`. A zero midpoint (and a
  `ln_1p` ball reaching the pole, as `atanh`'s `2x/(1−x)` does at the first attempt) now reports the
  whole-line radius, so the Ziv loop retries at a guard where the argument is representable.
- **A whole-line (`Mag::INFINITY`) radius panicked the Ziv containment test** instead of forcing a
  retry: the exported radius is a `Repr` infinity, and the test's `a ± e` arithmetic asserts its
  operands finite. It now short-circuits to "not contained", which is what the degenerate-denominator
  (`Ball::div`), pole and rounded-root paths documented as their retry signal.
- **32-bit `Word` targets returned a wrong `log₂ BASE` from the fixed-point walk** (the
  compile-time bracket behind the generic-base radius rules): the normalization shifted with
  `Word::leading_zeros`, so on `Word = u32` (e.g. the `i686` CI target) `log2(10)` came out
  ≈ 35 instead of ≈ 3.32 — radii then inflated by orders of magnitude, saturated to infinity
  inside the `atan` reduction and panicked the Ziv containment test (`atan2` of any
  non-unit-scaled pair). The walk is now pinned to explicit widths; a value-pinning test
  guards it.
- **The MSRV (1.68) workspace check failed on a std-only `f32::abs`** in `exp`'s
  overflow probe (that check compiles `dashu-float` without `std`; the inherent method is
  core-only from 1.85) — it now goes through the `Abs` trait, which is no_std-safe.
- **`no_std` test builds failed to compile**: the Ziv retry-count regression tests read the
  `thread_local` counter that only exists under `std`; those tests are now
  `#[cfg(feature = "std")]`, matching the `ziv.rs` test module's own gate.
- **Performance regressions of the Mag/Ball migration**: the
  two-stage `ln` reduction fired its cancellation double-precision on inputs that reduce to
  *exactly* a power of two (`ln(1e100)` in base 10 was 9× slower than the pre-migration
  code at 600 digits) — the cancellation condition now reads off the rounded input directly
  (one exact comparison) instead of the split exponents, and the power-two floor is settled
  by an exact comparison at the boundary. The generic-base radius rules (`scale_by_base_pow`
  and `to_repr`'s base-power export) no longer build `BASE^|e|` bigints per call — a
  compile-time fixed-point `log₂ BASE` bracket replaces them (one-bit tight, sound on either
  side), which was worth 1.5–3× on base-3 `exp` at high precision. `asinh` squares through
  the dedicated `sqr` kernel again, and the base-10 radius export keeps its significand
  instead of collapsing to a bare power of ten (≤ ~1× slack instead of ≤ 10×).
- **`exp`-family results near a rounding boundary paid one systematic Ziv retry** (~2.25×)
  on non-power-of-two bases: the `Bⁿ` powering chain compounds the per-op radius slack past
  the first attempt's preimage. The chain length is now charged to the initial guard
  (`pow_chain_guard`: `n` for `B ≠ 2` on `exp`/`exp_m1` and the hyperbolic family, `2n`
  unconditionally on `powf` — the base-2 `powf` margins are thin enough to need it). All 20
  measured regressions across the sweep are eliminated (18 of the 20 now at or below
  master's own timings; results bit-identical — correct rounding is unique), at the cost of
  a few guard digits on the affected functions.
- **`ln` of a large base power paid one Ziv retry near a rounding boundary**: the
  reconstruction constants (`s2·ln2`, `e_base·lnB`) evaluated at the bare work precision, so
  their 8-ulp radii were amplified by the scale factor (`ln(1e100)` @6 digits carried ~1/3
  of the target half-ulp before the candidate margin). The constants now evaluate with
  `⌈log_B|scale|⌉ + 1` extra digits — the same construction `exp` already uses for its
  `ln(B)` — keeping the amplified radius sub-ulp at a cost confined to inputs with large
  scale factors.
- **`sqrt` under directed modes** (#99): a sticky remainder was rounded *down* under
  `Up`/`Away` when the integer root carries `precision + 1` digits (`Up(sqrt(6))² < 6` at
  p53) — the final rounding now consults the rounding mode instead of hardcoding half-up.
- **`div` no longer pre-rounds an over-wide dividend** (#100): the kernel bounds it by an
  exact digit split instead, keeping the dropped digits as sticky rounding information.
  The old pre-round could report a false `Exact` (`5/1` @ p1), invert the direction
  (`7/-1` under `Up` gave `-8` instead of `-4`), or land on a false midpoint (`31/4`
  base 3 `HalfEven` gave `6` instead of `9`).
- **`div` now rounds exact and `precision+1`-digit quotients in a single step** (#100):
  an exactly-dividing quotient was returned unreduced (`15/3` @ p2 gave the 3-digit `5`
  instead of `4`), and an over-wide quotient was rounded on the integer instead of the
  precision grid (`3/5` @ p2 gave the unrepresentable `0.625` instead of `0.5`).
- **`div` with a negative divisor and an over-wide dividend** carried the sticky low part
  with the wrong sign (the divisor-sign normalization negates both operands), and the
  `precision+1`-digit quotient's half comparison sign-flipped `B − 2·ql` where only `|ql|`
  enters — the latter mis-rounded negative dividends in non-binary bases under nearest
  modes (`-21/2` base 10 @ p1 `HalfEven` gave `-20` instead of `-10`).
- **`nth_root` no longer double-rounds a `precision+1`-digit root** (#100): the exponent
  alignment now takes the truncating shift when the padding one would grow the root past
  the precision, so one rounding decides (`nth_root(2, 1.75)` @ p2 `HalfEven` gave `1.0`
  instead of `1.5`).
- **`with_base` rounds its exact-conversion shortcuts** (#100): the power-of-base shortcuts
  and small-exponent path returned significands wider than the target precision, and the
  division path pre-rounded its dividend, so the `Exact` flag could be false (`2.1` →
  base 2 reported `Exact`).
- `exp` — and every transcendental built on it, e.g. `sinh`/`cosh` — of an argument whose
  exponent is in the 10^14 range no longer dies on an out-of-memory allocation. The base-aware
  radius export converted a binary exponent to a decimal one (and back) through a small
  rational bound (`28/93` and `30102/100000` for log₁₀2, `3322/1000` and `33218/10000` for
  log₂10). Those err by ~5·10⁻⁵ *relative*, and that error multiplies the exponent: at 10^14 it
  overshot the outward power of ten by `10^1.2e10`, leaving the Ziv error radius astronomically
  larger than the value it bounded — the containment test then tried to align that gap and ran
  out of memory. The conversions now use 64- and 62-bit fixed-point bounds, so the outward
  slack stays under one digit for every `isize` exponent.
- Additions/subtractions of operands with an astronomically large exponent gap (~10⁹+ digits,
  e.g. the two exponentials composing `sinh(1e14)`) no longer die on an out-of-memory
  allocation: the sticky low part of the aligned sum is collapsed to a bounded position when
  it sits entirely below the rounding window (`repr_round_sum`), and the hyperbolic
  `sinh`/`cosh`/`sinh_cosh` compositions drop an exponential that sits below the other's ulp
  window instead of aligning the gap.
- `sqrt` of a perfect square in a non-power-of-two base never returned under the directed
  rounding modes (`Down`/`Up`/`Zero`): it kept doubling its working precision until the retry
  budget was exhausted — `sqrt(4)` in base 10 escalated past 10^8 digits instead of returning
  `2`. `sqrt` was the one transcendental still deriving its Ziv radius by hand (a blanket
  `value.ulp()`), and no nonzero radius fits a one-sided directed preimage (`Down`'s
  `[y, y+ulp)` cannot contain `[y−r, y+r]`), so an exactly-representable root could not be
  certified at all. It now wraps its kernel result as a `Ball`, so an exact root carries
  radius 0 and certifies immediately; an inexact root keeps the same one-ulp bound, and the
  base-2 fast path is untouched.
- `log2`/`ln` of a value a hair above 1 at low precision could return exactly `0`: the Ziv
  containment test accepted a zero candidate whose radius was nonzero, but no nonzero real
  rounds to exactly zero (the documented ±ulp preimage of ±0 is a special case). A zero
  candidate is now certified only with a zero radius. Scoped to this crate's driver — the
  public `ErrorBounds` semantics are unchanged.
- `x + (-0)`, `x - (-0)` and their mirrors returned `0` instead of `x` whenever the exponent
  gap to `-0`'s sentinel exponent (`-1`) reached past the end of `x`'s significand — e.g.
  `1e-16 + (-0)` was `0` at precision 12. The `addsub_*` kernels short-circuited only on
  `+0`, so `-0` fell through to the alignment path, where its sentinel exponent makes it look
  like the *larger* operand and `x`'s significand is shifted out entirely.
- A sum or difference of two *zero* operands now follows IEEE 754 §6.3: `(-0) + (-0)` and
  `(-0) - (+0)` are `-0` (`x + x` retains `x`'s sign, even when `x` is zero); a zero of mixed
  signs is `+0`, or `-0` under roundTowardNegative, as the exact zero of a cancellation
  already was.
- `powf` (and the `powi` fallback for exponents past the squaring chain) no longer stalls
  when the exponentiation drives the `exp` argument far negative — e.g.
  `powf(7.03e71, -84.91)`, whose `y·ln x ≈ −14035` makes the result ≈ `1e-6096`. The
  internal `exp` input-error fold omitted the result's magnitude for a negative argument,
  over-estimating the error radius by `e^{−x}` (hundreds of digits); the Ziv loop could only
  certify by growing its working precision past `|x|/log_B e`, burning 9 retries and ~6000
  digits: 1.5 s at 16 digits instead of 60 µs (and 6.5 s at 151). The fold now scales by the
  result's magnitude, so such calls certify on the first attempt.
- `ln` (and `log2`/`log10`/`ln_1p`, which share its core) of an argument with a huge
  exponent (e.g. `ln(1e1000000000)`) previously never returned, holding gigabytes: the
  argument reduction materialized a power of two spanning the whole exponent *gap*. The
  reduction now splits the magnitude by an exact base-exponent re-tag plus a bounded
  power-of-two finish, so no power of the gap is ever materialized, and the reduction
  arithmetic is exact integer work end to end (the previous single f32 `log2` estimate
  loses hundreds of bits of accuracy once `|log2 x|` approaches 10^9). (issue #103)
- The hyperbolic functions (`sinh`, `cosh`, `sinh_cosh`, `tanh`, `asinh`, `acosh`) now
  fold the *input's own rounding* into the certified error radius. Previously the input
  was rounded to the working precision and the rounding error dropped, which understated
  the radius by the ulp of the *original* magnitude — fatal for `acosh` near 1, where the
  `x−1` cancellation shrinks the value (and its ulp) while the inherited error does not:
  `acosh` could certify the wrong neighbour of a decimal tie (issue #102). All six now
  thread the input ball (or pass the raw repr into `exp_compute`/`ln_compute`, which fold
  it themselves).
- `ConstCache::pi` (and `Context::pi`) now compute one extra Chudnovsky series term,
  fixing an off-by-1-ulp mis-rounding at certain precisions (previously the term count
  provided only the ceiling — no accuracy headroom — so the series truncation error
  could push the result to the wrong side of a rounding boundary).
- `FBig::with_precision` (and `CachedFBig::with_precision`) now rounds an *unlimited*-
  precision source down to a finite target precision. Previously the shrink guard
  compared context precisions (`0 > N` is always false), so an unlimited value kept its
  full significand and the result violated the precision invariant.
- **(internal) `Ball`'s square rule folded only one `|mid|·rad` cross term** where
  `(m ± r)² = m² ± 2mr + r²` needs two — an under-bound by exactly `‖m‖·r` (the doc
  comment already stated the `2·`; the code did not). Only `asinh` squares a
  nonzero-radius ball through the dedicated `sqr` kernel, so the blast radius was its
  input-error term; the corner-coverage regression pins both `(m ± r)²` extremes.
- `hypot` of two operands past the extreme-exponent scale-down threshold returned
  `Err(Overflow)` whenever the result was not exactly representable (e.g.
  `hypot(7·2^e, 4·2^e) = √65·2^e` at `e ≈ isize::MAX/2`). The rescale factor `k` was
  derived from the *larger* operand's raw exponent only, but base-B normalization can
  leave the smaller value with the larger raw exponent (`4·2^e` normalizes to `1·2^(e+2)`)
  — the smaller operand's square then collided with the infinity sentinel. `k` now scales
  by the larger of the two raw exponents.
- `tan_pi` (and `tan_unit` at any `u`) returned `-0` at the *positive* odd multiples of
  `u/2` unconditionally, breaking oddness (`tan_pi(-1) == tan_pi(1) == -0`) and diverging
  from MPFR (`+0`). Every integer zero of the tangent now carries the sign of the input,
  like the even-multiple row already did.
- `asin_unit(x, 0)`/`acos_unit(x, 0)` returned the `u → 0` limit `±0` for out-of-domain
  `|x| > 1` instead of `Err(OutOfDomain)`. The domain error now outranks the limit (the
  limit of a function undefined at that `x` for every `u > 0` is not `+0`).
- `Sum` of `FBig` lost the sign of an all-`-0` sum: `[-x.zero].sum()` and
  `[-0, -0].sum()` returned `+0` under the nearest modes, while the chained `(-0) + (-0)`
  returns `-0`. The exact accumulator's zero sign now follows the same IEEE 754 §6.3
  left-fold rule as the chained operators (`-0` under roundTowardNegative or when every
  addend is `-0`).
- **(internal) the ×u reduction's f32 magnitude guards are now one-sided-conservative**:
  the bound products `u_lb + s·b_lb` round in f32, and an *upward* rounding of the
  `k = 0` fast-path test could in principle classify a `k ≥ 1` argument as first-quadrant
  (a wrong result; reachable only from gigabit-scale inputs, but a wrongness, not a
  slowdown). The shared bound is shaved by its own f32 slack (`log2_u_bs_lb`), so any
  error falls towards the always-exact general path. The sticky-collapse guard in
  `repr_round_sum` likewise used the upper bound of `log₂B` where the derivation needs
  the lower one, and now carries an explicit margin for the f32 products.
- **(internal) `CachedFBig` mirrors the full ×u family** (`sin_unit`/`cos_unit`/
  `tan_unit`/`asin_unit`/`acos_unit`/`atan_unit`), restoring the drop-in rule that code
  compiling against `FBig` compiles unchanged against `CachedFBig` (only the fused
  `sin_cos_unit`/`atan2_unit` had been mirrored; the forwarding macro cannot pass the
  `u` argument, so these are hand-written wrappers).

## 0.6.0

### Change
- **(breaking) Ziv-backed transcendentals now require `R: ErrorBounds` (not `R: Round`)**: `exp`/
  `exp_m1`/`ln`/`ln_1p`/`log2`/`log10`, `powf`/`powi`, `hypot`, the trig and hyperbolic families
  (incl. inverses), and `FBig::sqrt`. All six built-in modes satisfy `ErrorBounds`; only custom
  non-`ErrorBounds` modes are affected.
- **(breaking) new `FpError::ZivRetryLimitExceeded`** — a Ziv loop that exhausts its retry budget
  (only possible if a radius-bound estimate is wrong) now returns `Err` instead of silently returning
  a possibly-1-ULP-wrong best-effort value.
- **(internal) every transcendental's error radius is now derived mechanically by Ball arithmetic**
  (`float/src/ball.rs`, an exact-integer error count composed through the series/reduction/composition,
  precision-aware) instead of the hand-derived `·ulp` formulas and outward intervals. Behavior
  unchanged, validated bit-exact against MPFR.
- **(internal) `Context::exp` of an astronomically large negative value now returns `Err(Underflow)`**
  (consistent with overflow; the convenience layer is unchanged).
- Removed the `rustversion` dependency (MSRV is 1.68); the unversioned `rand` / `rkyv` feature aliases
  now select the newest versions (`rand_v010` / `rkyv_v08`).

### Add
- **Correctly rounded transcendentals via a Ziv retry loop** — `exp`/`exp_m1`/`ln`/`ln_1p`, `log2`,
  `log10`, `powf`/`powi`, `hypot`, and the trig/hyperbolic families (incl. inverses) certify their
  rounding against the `ErrorBounds` preimage, retrying with more guard digits; exact results report
  radius 0 (MPFR-style `exact` tracking) so directed modes terminate.
- **Base-10 `log10`** (mirrors `log2`, with an exact power-of-ten shortcut).
- **Exact `Add`/`Sub`/`Mul` operators for `Repr`** (new `repr_ops` module) — lossless exact
  intermediates for the Ziv containment test, the correctly-rounded `Sum`, and the `FBig` multiply
  path.
- **`FBig::sqrt` is now correctly rounded** — a `p+1`-digit integer root was previously double-rounded
  (off by 1 ulp at digit boundaries); it now rounds in a single step from the round digit + `sqrtrem`
  remainder (MPFR-style). Power-of-two bases use a fast path; other bases certify via Ziv.
- **`CachedFBig` mirrors the rest of `FBig`'s value surface** (`round`/`trunc`/…/`quantize`,
  `hypot`/`nth_root`, `signum`/`ulp_lb`, base conversions), completing the drop-in mandate.
- `tuning` feature + `ziv_retries()`/`ziv_retries_reset()` for profiling retry counts;
  `Repr::zero_with_sign` public; rkyv 0.7 support (`rkyv_v07`).

### Fix
- **Mode-aware overflow/underflow saturation** — `Err(Overflow)`/`Err(Underflow)` and the
  `FBig → f32`/`f64` conversions now saturate to the directed endpoint per rounding mode (`±∞` /
  largest finite / smallest / `±0`) instead of mode-blind `±∞`/`0`; the `TryFrom` error variant is
  mode-independent.
- **`exp` range reduction for large `|x|`** — `ln B` now carries `⌈log_B|x|⌉+2` extra digits so the
  reduction quotient `s` is pinned (was off by ~`|x|`); the 32-bit `isize` threshold is fixed.
- **Directed `ln`/`log2` near `x ∈ [1, B)`** — the radius now also covers the pre-cancellation `sum`
  scale and the over-delivered `ln_base` context.
- **`FBig → f32`/`f64` subnormal/underflow** — round-to-odd at `width + 24` bits; a
  source-`log2_bounds` short-circuit for catastrophically tiny values.
- `ulp()`/`Repr::cmp` near the exponent ceiling (saturating arithmetic); the `no_std` build (the
  test-only Ziv counter is gated on `std`); `tan` pole check removed (~2× faster).

## 0.5.2

### Add
- `FBig::e` / `Context::e` / `CachedFBig::e`: Euler's number *e*, computed by
  exact-integer binary splitting on `e = Σ 1/k!` (leaf `(1, k, 1)`, reusing the
  universal `(P, Q, T)` merge). Unlike π, *e* is self-contained — it depends on no
  other cached constant and is itself reused by no operation — so it is **not**
  stored in `ConstCache` and `Context::e` takes no cache parameter. The factorial
  series is the optimal algorithm for *e* (asymptotically `O(M(n) log n)` under
  FFT multiplication, i.e. faster than π), and it avoids both the `ln`-based
  argument reduction and the `√p`-fold powering that `exp(1)` would pay for.
- `FBig::fma` / `Context::fma` / `CachedFBig::fma`: fused multiply–add
  `c + sign·(a·b)` with a single rounding. `sign` (`Sign::Positive`/`Sign::Negative`)
  selects add vs subtract, mirroring integer's `add_signed_mul`. It assembles the
  existing exact-product (`make_mul_repr`) and add/round kernels, inheriting their
  severe-cancellation and sticky-tail handling (and the guard digit an effective
  subtraction may leave). The trig quadrant reduction `x − k·(π/2)` now uses it,
  removing one rounding from range reduction. (`sqr` keeps its dedicated, faster
  kernel — don't write `x²` as `fma(x, x, …)`.)

- `FBig::ulp_lb`: a cheap lower bound on `ulp`, guaranteed strictly smaller than it
  (computed from the approximated `digits_lb` rather than the exact digit count). Public
  successor to the internal `sub_ulp`; useful as a conservative negligibility threshold
  (e.g. iterative-method termination). For a rigorous error/radius bound, use `ulp`.
- `Context::addsub_vv` / `addsub_vr` / `addsub_rv` / `addsub_rr`: low-level
  ownership-aware add/subtract kernels computing `lhs + rhs_sign· rhs` directly on
  `Repr` (no `FBig` wrapping, no `Result`) — `Sign::Positive` adds, `Negative`
  subtracts. The four variants cover every ownership combination of the two operands
  (`v` = by-value, `r` = by-ref); each reuses the owned operand's significand buffer
  where it can, avoiding a clone versus the `Context::add`/`sub` path. Intended for
  downstream crates (e.g. `dashu-ball`) that want by-value `Repr` arithmetic on a
  fixed context. The `+`/`-` operators and `Context::add`/`sub` now route through
  `addsub_*`, making it the single source of truth for add/subtract routing.

### Change
- `x ± 0` now rounds `x` to the context precision. The add/subtract path previously
  short-circuited on a zero operand and returned the other operand verbatim, which
  could leave a guard digit (up to `precision + 1` digits) in the result. All other
  add/subtract results are unchanged.

### Fix
- `test_e_known_decimal_prefix` failed to compile under `no_std` (`ToString` is not in
  the prelude without `std`): import `alloc::string::ToString` in the cache tests.
- `Context::mul`, `Context::sqr`, and `Context::cubic` are now strictly correctly rounded. They
  previously shrank the operand(s) to `2*precision` (mul/sqr) or `3*precision` (cubic) before
  multiplying — a speed optimization. That operand pre-rounding — though each operand is rounded
  correctly — perturbs the result by the accumulated rounding error, so the final value could land
  1 ulp off the exact-product-rounded value when it sat near a rounding boundary. The exact product
  / square / cube of the full operand(s) is now computed and rounded (still via the dedicated
  `sqr`/`cubic` kernels), which is always correctly rounded, at the cost of operating on operands
  far larger than the target precision (uncommon).
- `Display`/`LowerExp`/`UpperExp` (and the per-base `{:b}`/`{:o}`/`{:h}`/`{:x}` formatters) now read
  `Repr::sign` (not the bare significand, which is always `+` for zero) and clamp a zero
  significand's display exponent to `0`, so the `-0` sentinel exponent no longer leaks into the
  output. By default `-0` and `+0` both render as `"0"` (signed zero is treated as an internal
  detail); the formatter's `+` flag reveals the sign (`-0` → `"-0"`, `+0` → `"+0"`) — use it for
  string round-trips (e.g. into MPFR) that must preserve the signed-zero sign.

## 0.5.1

### Add
- `Repr::new_const`: a `const`-evaluable, normalized `Repr` constructor from a `DoubleWord`
  significand (the `const` counterpart of `Repr::new`). `FBig::from_parts_const` now delegates to
  it, and the complex literal macro uses it.
- `FBig::log2` / `Context::log2` / `CachedFBig::log2`: base-2 logarithm, correctly rounded via
  `ln(x)/ln(2)` evaluated at an elevated working precision. Previously only the f32-precision
  `log2_bounds` magnitude estimate was available, so directed `log2` was wrong by many ULPs.

### Fix
- `powi`'s overflow guard no longer reports a spurious overflow when the base is very close to 1
  (a large significand with a large negative exponent). It estimated `log2(base)` with `log2_est`,
  which for such a base is the difference of two ~1e3-magnitude terms and catastrophically cancels
  to ~1e-4 of `f32` noise; scaled by a large exponent that noise crossed the overflow threshold
  — which on 32-bit targets is only `isize::MAX·log2(B) ≈ 7e9` — and returned a spurious ±inf. This
  made high-precision base conversion (`FBig::with_base`) panic ("arithmetic operations with the
  infinity are not allowed!") on 32-bit targets (wasm32, i686). The guard now uses the
  bit-length-based `log2_bounds`, which does not cancel (#95).
- `to_f64`/`to_f32` now round the source once, directly to the target's precision at its own
  magnitude (fewer than 53/24 bits for subnormals), instead of through a fixed 53/24-bit
  intermediate that re-rounds into the subnormal grid. This removes a 1-ULP double-rounding error
  on subnormal values that sit just past a subnormal halfway.
- `to_f64`/`to_f32` no longer panic in debug builds (nor silently double-round in release) on
  high-precision inputs: the base-changing conversion now pre-shrinks the source significand before
  dividing, upholding `repr_div`'s dividend-width contract (as `Context::div` already does) instead
  of feeding an oversized dividend into the division.
- `exp`, `exp_m1`, `sqrt`, `ln`, and `ln_1p` no longer panic on exact zero/one inputs that carry
  unlimited precision (precision 0), such as `FBig::try_from(0.0)` and the `FBig::ONE`/`ZERO`
  constants. The exact-result shortcuts now run before the limited-precision assertion.
- The `round_fract` debug assertion no longer materializes `B^precision`, which for a sparse sticky
  tail (where `precision` is the exponent gap — e.g. `exp_m1` of a large-magnitude input) could
  exhaust memory in debug builds. It now checks the precondition with `log2` bounds.

## 0.5.0

### Add
- **IEEE-754 signed zero (`-0`)**: operations now produce the sign of zero mandated by the standard
  (e.g. `1 / -inf = -0`, `sqrt(-0) = -0`, `ceil(-0) = -0`, cancellation under round-toward-negative).
  `+0` and `-0` compare equal; `-0.0` round-trips through `f32`/`f64`.
- **New error model**: `FpError` (`InfiniteInput`, `OutOfDomain`, `Indeterminate`, and new
  `Overflow(Sign)`/`Underflow(Sign)`) with `FpResult<T> = Result<Rounded<T>, FpError>`. Infinite
  *outputs* are values inside `Ok` (`1/0 → +inf`, `ln(0) → -inf`, `exp(huge) → +inf`); infinite
  *inputs* are `Err(InfiniteInput)` (structurally avoiding NaN-producing indeterminate forms); domain
  errors (`0/0`, `sqrt(-x)`, `ln(-x)`, `asin(|x|>1)`) are `Err`. The `FBig`/`CachedFBig` convenience
  layers panic on error and saturate `Overflow`/`Underflow` to signed infinity/zero.
- **`ConstCache` + `CachedFBig`**: `ConstCache` caches exact binary-splitting tree state for constants
  (π, ln2, ln10, ln(B) — including the base-free `√10005` isqrt that feeds π) so repeated calls at
  increasing precision *extend* prior work instead of recomputing. `CachedFBig` is an `FBig` carrying a
  shared `Rc<RefCell<ConstCache>>` handle; its transcendentals (`ln`, `exp`, `sin`/`cos`/…, `pi`, base
  conversion) thread that handle through `Context`. `Context`/`FBig` stay `Copy` + `Send` + `Sync` +
  `no_std`; only `CachedFBig` is `!Send + !Sync`. `CachedFBig::cache()`/`clear_cache()` and
  `ConstCache::total_terms()`/`total_words()` inspect/free cached memory.
- **Hyperbolic functions** `sinh`/`cosh`/`tanh`/`asinh`/`acosh`/`atanh` on `Context`/`FBig`/`CachedFBig`,
  built from cancellation-free `exp_m1`/`ln_1p` formulas with IEEE special-value handling.
- **`FBig::hypot`** / `Context::hypot`: overflow/underflow-safe `sqrt(a² + b²)` via the scaled
  sum-of-squares (the larger operand is never squared).
- **`FBig::sinh_cosh`** / `Context::sinh_cosh`: combined `sinh`+`cosh` sharing the `exp_m1(±x)` work.
- `exp`/`exp_m1` now accept infinite input (`exp(+inf) = +inf`, `exp(-inf) = +0`, `exp_m1(-inf) = -1`).
- `CachedFBig` now mirrors `FBig`'s full trait surface — formatting, ordering, conversions, shift and
  root/euclid ops, `Sum`/`Product` — plus the reference-operand variants of its binary operators and
  mixed ops with `FBig` and the integer primitives. Third-party traits (serde/num-traits/num-order/
  rand/zeroize/postgres) are reached via `.as_fbig()`.
- `Repr::num_hash_residue` (behind `num-order`), exposed so composite types can combine their parts'
  residues algebraically.

### Change
- **(breaking)** `Repr::is_zero` is renamed to `Repr::is_pos_zero` (it tests only `+0`); use
  `significand().is_zero()` to detect either signed zero. `num_traits::Zero::is_zero` for `FBig` now
  returns `true` for either.
- **(breaking)** `Sum` for `FBig` is now correctly-rounded: addends are accumulated exactly and the
  total rounded once (MPFR `mpfr_sum` semantics). The generic `Sum<T>`/`Product<T>` impls are replaced
  by concrete `Sum`/`Sum<&FBig>`/`Product`/`Product<&FBig>`; cross-type sums (e.g. `Sum<u8>`) require
  converting the elements first.
- **(breaking)** `FBig` human-readable serde now pads the serialized string to the context precision's
  digit count so precision round-trips (the binary format already preserved it).
- **(breaking, encoding)** infinities are re-encoded with sentinel exponents `isize::MAX`/`isize::MIN`
  and `-0` at exponent `-1`; `normalize()` preserves these, and `Repr`'s `PartialEq`/`Eq` are manual
  so `+0 == -0`.
- **(breaking, result model)** `Context` arithmetic/transcendental/trig methods now return
  `FpResult<FBig<R, B>>` instead of `Rounded<FBig<R, B>>` (arithmetic) / the old trig `FpResult` enum.
  `FBig::tan`/`asin`/`acos`/`atan2` now return `Self` (panic on error), matching the other trig methods.
- **(breaking, low-level)** `Context` constant-source methods take an additional
  `cache: Option<&mut ConstCache>` parameter; the high-level `FBig` API is unchanged (passes `None`).
- `atan2(±finite, +inf)` returns the signed zero of `y`; `powf(±0, y)` returns the *positive* result
  (`+0` for `y > 0`, `+inf` for `y < 0`) — use `powi` for the sign-correct `pow(-0, odd) = -0`.

### Remove
- Public `Repr::from_str_native` / `FBig::from_str_native` (now crate-private — use `s.parse()`).
- The old `FpResult` enum and the `MathCache` type (subsumed by the public `ConstCache`).
- The `panic_overflow`/`panic_underflow`/`panic_infinite`/`panic_power_negative_base`/`panic_root_negative`
  helpers (their conditions are now `FpError`s).

### Fix
- Signed-zero correctness: `exp_m1(-0) = -0`; `powf(base, -0) = 1`; `quantize(-0)` preserves the sign;
  `+`/`-` produce `-0` on exact cancellation under `Down`; `Sum` cancellation to zero yields `+0` (or
  `-0` only under roundTowardNegative); `IBig`/`UBig::try_from(FBig)` accept `-0`.
- `NumOrd` against a primitive `0.0`/`-0.0` now reports either signed zero as `Equal`.
- `error_bounds` honors the `ErrorBounds` contract for unlimited precision (`Away` returns
  `(0, 0, true, true)`), and `HalfEven` gives `-0` the one-sided preimage (matching `Zero`/`HalfAway`).
- `Context::asin`/`acos` no longer panic on `±1` under `Down` (the `1 - x²` → `-0` → `sqrt(-0)` path).
- `exp`/`exp_m1`/`powi` return `±inf`/`0` on astronomically large results instead of panicking.
- `exp`/`exp_m1` at high precision (≳ a few thousand digits) were wrong in the low bits — the series
  working precision now carries `≈ √p` extra guard digits to absorb the `Bⁿ` final-powering error
  amplification (cf. MPFR's `K ≈ √precy`).
- `ShrAssign` (`>>=`) previously subtracted the shift twice.
- Trig functions no longer panic on tiny negative inputs (`sin(-1e-30)`): the signed-zero encoding no
  longer trips argument reduction.
- Broken intra-doc links surfaced by `cargo doc -D warnings`; `f64::ceil` in `ConstCache` replaced
  with a `no_std`-safe integer ceiling.
- `FBig::from_repr`'s debug assertion now accepts the documented single guard digit.

### Improve
- Documented the `math::trig` module and enabled `#![deny(missing_docs)]` together with
  `clippy::dbg_macro`, `clippy::undocumented_unsafe_blocks`, and `clippy::let_underscore_must_use`
  as crate-level denies.
- Migrated the verbose `FBig` type prose out of the rustdoc and into the user guide, leaving a concise
  summary with guide links; the runnable `# Examples` are kept verbatim.
- (internal) `Context::iacoth` and the `ConstCache` π path use binary splitting; the PostgreSQL
  `NUMERIC` conversion and trig argument reduction use `UBig::to_digits` / `IBig::try_from`.

## 0.4.5

### Add
- Add `FBig::quantize(exp)` to round to the nearest multiple of `BASE^exp` (the dashu analog of Python's `Decimal.quantize()`), returning `Rounded<Self>` with the result precision set so that `ulp()` equals `BASE^exp`.
- Implement the cubic root (`CubicRoot` for `FBig`, `Context::cbrt`) and the general nth root (`FBig::nth_root`, `Context::nth_root`) with correct rounding, built on top of `UBig::nth_root`.
- Implement trigonometric functions (`sin`, `cos`, `tan`, `asin`, `acos`, `atan`, `atan2`, `sin_cos`) for `FBig` and `Context<R>` ([#60](https://github.com/cmpute/dashu/pull/60)).
- Add π constant computation (`FBig::pi()` and `Context::pi()`) using the Chudnovsky algorithm with binary splitting ([#60](https://github.com/cmpute/dashu/pull/60)).
- Add `FpResult` enum to handle non-finite math operation results (NaN, Infinite, Overflow, Underflow) without panicking ([#60](https://github.com/cmpute/dashu/pull/60)).
- Add `panic_nan`, `panic_overflow`, `panic_underflow`, and `panic_infinite` helpers to the `error` module.
- Optional `rand_v09` (rand 0.9, MSRV 1.63) and `rand_v010` (rand 0.10, MSRV 1.85) features mirroring `rand_v08`. The default `rand` feature still maps to `rand_v08`.
- The random-float distributions (`Uniform01`, `UniformFBig`) and their sampling now live once in the version-agnostic `dashu_float::rand` module. The per-version modules are now private trait bindings.

### Fix
- Fix rounding issues in `to_f32()` and `to_f64()` ([#53](https://github.com/cmpute/dashu/issues/53), [#56](https://github.com/cmpute/dashu/issues/56)).
- Fix several rounding bugs in `FBig`/`Context` addition and subtraction: severe-cancellation collapse, spurious-ULP errors from negligible operands, the window-edge boundary, and `Context::sub` with a zero left operand under directed rounding modes.
- Fix `FBig::fract()` inflating context precision and `split_at_point_internal` using an incorrect fractional scale for values smaller than one.

## 0.4.4

- Bump MSRV from 1.61 to 1.68.

## 0.4.3

- Mark `FBig::from_str_native` as deprecated.
- Implement `TryFrom<Repr>` and `TryFrom<FBig>` for primitive integers.
- Implement `TryFrom<Repr<2>>` and `TryFrom<FBig<_, 2>>` for primitive floats.
- Implement `From<UBig>` and `From<IBig>` for `Repr`.
- Implement `core::fmt::{Binary, Oct, LowerExp, UpperExp, LowerHex, UpperHex}` for `Repr`, `FBig` (some are limited to certain bases).

## 0.4.2

- Add `Repr::from_static_words` to support the `static_fbig!` and `static_dbig!` macros.
- Add `FBig::from_repr_const` to support create an `FBig` instance from repr in const context.
- Add conversion from `f32`/`f64` to `Repr<2>`.
- Implement `NumOrd` between `FBig` and primitive integers / floats. 
- Implement `AbsOrd` between `FBig` and `UBig`/`IBig`.
- Now the `Debug` output of `FBig` values will not contains the rounding mode information (when alternative flag is not set).

## 0.4.1

- Fix the termination criteria for `ln` and `exp` series ([#44](https://github.com/cmpute/dashu/issues/44)).
- Fix `powf` panicking when base is 0.

## 0.4.0

### Add

- Implement `num-order::NumOrd` between `FBig` and `UBig`/`IBig` and between `FBig` with different bases.
- Implement `num-order::NumHash` for `FBig` and `Repr`.
- Add `ErrorBounds` trait that calculate the rounding range for a floating point number.

### Change

- Now feature `num-traits` and `rand` are not enabled by default, feature `num-order` is enabled instead.
- The type of `Repr::BASE` is changed from `IBig` to `UBig`
- `UBig::square` and `IBig::square` are renamed to `sqr`.
- The implementation of square root is now implemented by the `dashu_base::SquareRoot` trait instead of a standalone method of `FBig`.
- The rounding behaviors of `FBig::to_decimal` and `FBig::to_binary` are changed for better ergonomics.
- The rounding behaviors of `FBig::to_f32` and `FBig::to_f64` now follow the mode specified by the type argument.

## 0.3.2

- The default precision for float numbers from `from_parts`/`From<UBig>`/`From<IBig>` are now based on the actual digits on the integers, rather than the digits after simplification. (#28)

## 0.3.1

- Implement `num_traits::{Zero, One, FromPrimitive, ToPrimitive, Num, Signed, Euclid, Pow}` for `FBig` (#19)
- Implement `rand::distributions::uniform::UniformSampler` for `FBig` through `crate::rand::UniformFBig`
- Implement `rand::distributions::{Open01, OpenClosed01, Standard}` for `FBig`
- Implement `dashu_base::Inverse` for `FBig`
- Implement `rand::distributions::uniform::SampleUniform` for `FBig`.
- Implement `serde::{Serialize, Deserialize}` for `FBig` and `Repr`
- Implement `Rem` trait for `FBig`
- Add support of random floating point numbers generation through `crate::rand::Uniform01` and `crate::rand::UniformFBig`.
- Add support for serialization from/to PostgreSQL arguments through `diesel::{deserialize::FromSql, serialize::ToSql}` and `postgres_types::{FromSql, ToSql}`.
- Add `from_str_native()` for `Repr`
- Add `to_f32()`, `to_f64()` for `Repr`, and these two methods supports all bases for both `Repr` and `FBig`.
- Add `to_int()` for `Repr`, which is equivalent to `FBig::trunc()`
- Add `TryFrom<FBig>` for `UBig` and `IBig`
- Add `round()` for `FBig`
- Add `rand_v08` and `num-traits_v02` feature flags to prevent breaking changes due to dependency updates in future 
- Re-export operation traits through the `ops` module.

## 0.3.0

### Add

- Conversion from FBig to `f32`/`f64` support subnormal values now.
- Add a `split_at_point()` function to `FBig`

## 0.2.1

- Implement `core::iter::{Sum, Product}` for `FBig`
- Implement `powf`, `sqrt` for `FBig`

## 0.2.0 (Initial release)

- Support basic arithmetic operations (`add`/`sub`/`mul`/`div`/`exp`/`ln`) and base conversion.

# Todo

## Roadmap to next version
- Support generating base math constants (E, Pi, SQRT2, etc.)
- Support operations with inf
- Create operations benchmark
- Benchmark against crates: rug, twofloat, num-bigfloat, rust_decimal, bigdecimal, scientific
- Implement more formatting traits
- Other math functions: sin/cos/tan/etc.
