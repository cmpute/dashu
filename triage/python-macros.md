# Group `python-macros` (29 reports)

Three areas: the literal macros (`macros/`, 8 reports), one guide example (1 report) and the
Python binding (`python/`, 20 reports). All 29 reports reproduced at `a0fc535`; none was
dismissed. 28 are real bugs and were fixed, 1 is a documentation error.

How things were verified:

- Macros: `cargo test -p dashu-macros -p dashu` (debug, `--all-features`, and `--release` for
  `dashu-macros`), including the new `compile_fail` doctests.
- Guide: the new snippets were compiled and run in a scratch crate; `mdbook build` could not be
  run here (`mdbook-katex` is not installed). No heading or anchor changed.
- Python: the binding was built with `maturin develop` (debug profile, CPython 3.13, with and
  without `--features serde,rand,rkyv,zeroize`) in a scratch `uv` venv, and tested with
  `pytest python/tests` (118 passed with all features; 54 before the changes). Every Python
  cluster was first reproduced with a scratch script against the unmodified build. A scratch
  randomized cross-check against Python's own `list` / `int` / `float` / `Fraction` / `Decimal`
  models (bit deletion and slice assignment, hashes, reflected and in-place operators, modular
  pow, layout and digits of `format`) is clean except for exact ties (see "Rounding of ties"
  below) and for the float-fmt carry bug noted under cross-group notes.

## Clusters

### cbig-decimal-integer-truncated
Reports: D26, E36
Verdict: real, fixed
Root cause: `parse_coeff` (`macros/src/parse/cmplx.rs`) converted a decimal/octal coefficient
with `FBig::with_base::<2>()`, whose target precision is `floor(digits * log2(base))` bits: one
decimal digit became 3 bits, so 9 (4 bits) was truncated to 8, 65 to 64, 99 to 98, 999 to 998.
Test: `macros/tests/cmplx.rs::test_cbig_decimal_integers_exact` (and the precision pins in
`test_cbig_decimal_default`).
Fix: new `to_binary` helper (`macros/src/parse/cmplx.rs:86`) converts with as many bits as the
largest integer with that many digits needs (`bit_len(B^p - 1)`), and with the integer's own
bit length for integers scaled by a positive exponent (`9e2`). Every integer coefficient is now
exact, as the docs promised.
Notable: the documented precision of decimal coefficients changes with it (`cbig!(3 + 4i)` has
precision 4 instead of 3, `cbig!(3.125 + 4i)` 14 instead of 13); `macros/docs/cbig.md` and the
existing test were updated. `FBig::with_base` itself (float-convert area) was left alone.

### float-literal-negative-zero
Reports: E95
Verdict: real, fixed
Root cause: the float macros rebuild the value from `(sign, significand, exponent)`
(`macros/src/parse/float.rs`), and the sign of a zero is not carried by its significand
(`from_parts_const` returns `ZERO` for a zero significand), so `dbig!(-0.0)` / `fbig!(-0.0)`
were `+0` while `from_str("-0.0")` is `-0`. `cbig!` had the same loss per coefficient.
Test: `macros/tests/float.rs::test_negative_zero`, `macros/tests/cmplx.rs::test_cbig_negative_zero`.
Fix: a negative zero literal expands to `from_repr_const(Repr::neg_zero())` (const and static
positions included); `cbig!` routes a negative-zero coefficient through the non-const path
(`small_coeff` returns `None`, `gen_binary_fbig_value` emits `Repr::neg_zero()`).
Notable: the *precision* of a zero literal is still 0 (`dbig!(0.00).precision() == 0`, pinned by
the existing `test_dbig`/`test_fbig`), unlike `from_str`; that comes from
`FBig::from_parts_const` ignoring `min_precision` for a zero significand and was not changed.

### static-float-precision-depends-on-size
Reports: E247
Verdict: real, fixed
Root cause: `static_fbig!` / `static_dbig!` used `from_parts_const(.., Some(prec))` when the
significand fits in 32 bits and `from_repr_const` (unlimited precision) otherwise
(`macros/src/parse/float.rs`), so the documented "unlimited precision" only held for wide
literals.
Test: `macros/tests/float.rs::test_static_precision`; `tests/import.rs::test_static_macros`.
Fix: the small static path now uses `from_repr_const(Repr::new_const(..))`: every static float
literal has precision 0, as documented. Docs of both macros gained an example.
Notable: this is a behaviour change for short static literals (they used to carry the literal's
precision). The root crate's own `tests/import.rs` relied on it (`SC.to_decimal()` on a small
static) and was updated to set a precision first, as the docs tell users to. The alternative
(giving every static literal the parsed precision) needs a new const constructor in
`dashu-float` and would silently change results for wide static literals; see "Unsure".

### ibig-repeated-signs
Reports: E226
Verdict: real, fixed
Root cause: `parse_integer_with_error` (`macros/src/parse/int.rs:57`) set a `neg` flag for every
`-` and ignored every `+` before the digits, so `--5`, `+-5`, `-+5`, `---5` were all -5.
Test: `macros/src/parse/int.rs::tests::test_single_sign_only` (the parser is private),
`macros/tests/int.rs::test_ibig_single_sign`, `compile_fail` doctest in `macros/docs/ibig.md`.
Fix: at most one sign is accepted; anything else is `InvalidDigit` (a compile error). An empty
literal is now `NoDigits` instead of an `unwrap` panic.

### rbig-denominator-without-slash
Reports: E254, E129
Verdict: real, fixed
Root cause: `parse_ratio_with_error` (`macros/src/parse/ratio.rs:96`) took the second number
token as the denominator whether or not a `/` had been seen. So `rbig!(1 2)`, `rbig!(1 - 2)`,
`rbig!(3 + 4)` built fractions (E254), and in `rbig!(a base 16)` the identifier `base` was taken
as the denominator and failed to parse as a digit string (E129).
Test: `macros/src/parse/ratio.rs::tests::{test_accepted_forms, test_rejected_forms}`,
`macros/tests/ratio.rs::test_rbig`, `compile_fail` doctests in `macros/docs/rbig.md`.
Fix: a denominator is only read right after `/`; each component takes at most one sign; `~` at
most once; a dangling `/` is `NoDigits`.

### guide-unlimited-precision-example
Reports: E215
Verdict: docs
Root cause: `guide/src/float/precision.md` ("Unlimited precision") asserted that the sum of two
parsed values has precision 0; parsed values carry the precision of their digits (as the table
in the same file says). The follow-up example called `with_precision(20).unwrap()` on a 41-digit
value, which panics (`Approximation::unwrap` on an inexact result), and labelled a parsed value
"unlimited".
Test: none in the repo (guide snippets are not compiled by CI); the new snippets were run in a
scratch crate.
Fix: both examples lift the cap explicitly with `with_precision(0)` and show the contrast with
the rounded result; `guide/` and `guide-zh/` updated together.

### py-ubig-del-bits
Reports: D37, E49
Verdict: real, fixed
Root cause: `remove_bits_in_middle` in `UBig.__delitem__` (`python/src/int.rs:659`) kept the low
`end - start` bits of the lower part instead of the low `start` bits, so the deleted bits
survived whenever a lower bit was set. An empty slice went down the same path.
Test: `python/tests/test_int.py::test_del_bits_matches_list_model`.
Fix: `low.clear_high_bits(start)`; empty slices return early.

### py-ubig-empty-slice-panics
Reports: E140
Verdict: real, fixed
Root cause: `UBig.__setitem__` / `__delitem__` computed `stop - start` and the masks without
checking `stop <= start` (`slice.indices()` does not clamp `stop` to `start`), giving a huge
`UBig::ones` (capacity assertion) or a negative `UBig` subtraction.
Test: `python/tests/test_int.py::test_set_bit_slice_matches_list_model` (and the delete test).
Fix: an empty slice returns early in both methods (`python/src/int.rs:630`, `:682`).

### py-index-beyond-isize
Reports: E220
Verdict: real, fixed
Root cause: `index.extract::<isize>()` fails for a Python int that does not fit, and the code
fell through to "not an integer or slice" (`TypeError`). Same pattern in `Words` (3 methods) and
in the bit indexing of `UBig`/`IBig` (4 methods).
Test: `python/tests/test_words.py::test_index_out_of_index_range`,
`python/tests/test_int.py::test_bit_index_out_of_index_range`.
Fix: `convert::extract_index` (`python/src/convert.rs:46`) raises `IndexError` for an `int`
that does not fit, as `list` does.

### py-zero-division-panics
Reports: E81, E124
Verdict: real, fixed
Root cause: the binding called the Rust operators directly, which panic on a zero divisor:
`%` for all five real types, `//`, `/`, `divmod` for the integers and `/` for `RBig`
(`ibig_div_mod_floor`, the `impl_binops!`/`impl_float_binops!`/`impl_rpy_binops!` dispatchers),
and `ConstDivisor::new(0)` for `pow(x, e, 0)` (`ipy_pow`).
Test: `python/tests/test_int_math.py::{test_integer_zero_division, test_pow_zero_modulus}`,
`python/tests/test_float_ops.py::test_float_modulo_by_zero`,
`python/tests/test_ratio_ops.py::test_zero_division`.
Fix: an exact zero divisor raises `ZeroDivisionError` (`%` by a float zero too, as for Python
floats; `/` by a float zero still gives an infinity, as before); `pow(x, e, 0)` raises
`ValueError` like `int`. Also closed, same root cause: `RBig.pow(0, -n)`,
`RBig.from_parts(n, 0)` (`ZeroDivisionError`), `nth_root(0)`, `ilog` of zero (`ValueError`),
`is_multiple_of(0)`, and `%` with an infinite float operand (`ValueError`).

### py-gcd-lcm-of-zeros
Reports: E70
Verdict: real, fixed
Root cause: `lcm` (`python/src/math.rs`) computed `gcd(a, b)` first, and the `Gcd` trait panics
for two zeros.
Test: `python/tests/test_math.py::test_gcd_lcm_of_zeros`,
`python/tests/test_int_math.py::test_number_theory_zero_arguments`.
Fix: `lcm` is 0 when either argument is 0; `gcd(0, 0)` is 0 and `gcd_ext(0, 0)` is `(0, 0, 0)`
(module functions and `UBig` methods), as `math.gcd` / `math.lcm`.

### py-ubig-subtraction-underflow
Reports: E119
Verdict: real, fixed
Root cause: `UBig - UBig` and `UBig - non-negative int` (both orders, and `-=`) used the
panicking `UBig` subtraction.
Test: `python/tests/test_int_math.py::test_ubig_subtraction_below_zero`.
Fix: `unsigned_sub` (`python/src/int.rs:191`) returns an `IBig` when the difference is negative
(consistent with `UBig - negative int` and `UBig // int`, which already return `IBig`).

### py-inplace-operators-reject-operands
Reports: E132
Verdict: real, fixed
Root cause: the in-place operators converted the operand with `into_ubig()` / `into_ibig()` and
raised `TypeError` on failure, instead of returning `NotImplemented` so that Python falls back to
the binary operator.
Test: `python/tests/test_int_math.py::test_inplace_ops_agree_with_binary_ops`.
Fix: the operand types `UnsignedOperand` / `SignedOperand` (`python/src/convert.rs:238`) fail to
extract for other operands, which PyO3 turns into `NotImplemented`. `UBig.__isub__` was removed
(the type of the difference depends on its sign, see E119), so `n -= m` is `n = n - m`. The same
fix applies to `IBig` (`IBig(1) += 0.5` used to raise).
Notable: the in-place operators that remain still mutate the object in place, so
`a = b = UBig(5); a += 1` also changes `b` (pre-existing; `-=` on a `UBig` now rebinds instead).

### py-missing-reflected-operators
Reports: E164
Verdict: real, fixed
Root cause: `UBig`/`IBig` had no `__rmod__`, `__rdivmod__`, `__rand__`, `__ror__`, `__rxor__`,
`__rpow__`; and `ipy_pow` only accepted a small Python int as exponent.
Test: `python/tests/test_int_math.py::test_reflected_integer_ops`.
Fix: the reflected methods promote the left operand and call the forward implementation
(`int_reflected`, `python/src/int.rs:243`), so both orders agree in value and type; exponents
and moduli may be any integer type; `pow(UBig, e, negative)` returns an `IBig` instead of
raising.

### py-int-dbig-rounds
Reports: E66
Verdict: real, fixed
Root cause: `DBig.__int__` used `to_int()`, which rounds with the mode of the type (HalfAway).
Test: `python/tests/test_float_ops.py::test_int_truncates`.
Fix: `__int__` truncates (`python/src/float.rs:443`); `to_int()` is unchanged. `int`,
`math.trunc/floor/ceil` and `round` of an infinity raise `OverflowError` instead of panicking.

### py-auto-float-precision
Reports: E67
Verdict: real, fixed
Root cause: `auto()` converted a float with `FBig::try_from(f64)` (precision = significant bits
of the value: 0 for 0.0, 1 for 5e-324) instead of the helper `FBig(x)` uses.
Test: `python/tests/test_precision_config.py::test_auto_float_uses_default_precision`.
Fix: `auto` uses `fbig_from_f64` (`python/src/utils.rs:85`).

### py-hash-disagrees-with-equality
Reports: E71
Verdict: real, fixed
Root cause: `__hash__` used `NumHash` with Rust's `DefaultHasher` (and an ad-hoc formula for
`CBig`), unrelated to the hashes of the Python numbers the values compare equal to.
Test: `python/tests/test_hash.py` (new file).
Fix: new `python/src/hash.rs` implements Python's numeric hash (`p * q^-1 mod 2^61 - 1`, with
`sys.hash_info`'s `inf` and `imag` constants) for integers, rationals, floats in any base and
complex numbers.

### py-rbig-math-rounding-protocol
Reports: E84
Verdict: real, fixed
Root cause: `RBig` had no `__floor__`/`__ceil__`/`__trunc__`/`__round__`, so `math.floor`/`ceil`
went through `__float__` and `math.trunc`/`round` raised.
Test: `python/tests/test_ratio_ops.py::test_rounding_dunders`.
Fix: the four methods, exact (`python/src/rational.rs:285`). `round()` rounds ties away from
zero, like `RBig.round()` and the existing `round()` of `FBig`/`DBig` (see "Unsure").

### py-cbig-two-argument-constructor
Reports: E171
Verdict: real, fixed
Root cause: `CBig(re, im)` took only the real part of each argument (`python/src/complex.rs`).
Test: `python/tests/test_math.py::test_cbig_two_argument_constructor`.
Fix: `CBig(re, im)` is `re + im*j` as `complex(re, im)`; the general case is computed exactly.

### py-cbig-equality-raises
Reports: E177
Verdict: real, fixed
Root cause: `CBig.__richcmp__` propagated the conversion error of the other operand.
Test: `python/tests/test_math.py::test_cbig_equality_with_other_objects`.
Fix: `==`/`!=` return `NotImplemented` for objects that are not numbers (so `False`/`True`);
strings are not parsed any more (`CBig(1) == "1"` was `True`); real numbers of every type
(including `UBig`, `IBig`, `RBig`, `Fraction`, `Decimal`) are compared exactly. `CBig` arithmetic
also accepts `UBig`/`IBig` now.

### py-fbig-decimal-format-digits
Reports: E51
Verdict: real, fixed
Root cause: `FBig.__format__` / `CBig.__format__` converted with `to_decimal()`, which rounds to
the ~15 decimal digits that 53 bits are worth, before formatting.
Test: `python/tests/test_format.py::test_fbig_decimal_digits_are_exact`.
Fix: `format_fbig_decimal` (`python/src/format.rs:178`) formats the exact decimal expansion
whenever the spec fixes the digits (explicit precision, or `f`/`%`); `e`/`g` without a precision
keep the `to_decimal()` digits. Infinities print as `inf` instead of panicking.

### py-rbig-format-digits
Reports: E100
Verdict: real, fixed
Root cause: `RBig.__format__` converted to a decimal with `precision + 16` *significant* digits
and let the formatting round again (too few digits for `f` with a large integer part, and a
double rounding).
Test: `python/tests/test_format.py::test_rational_digits_are_exact`.
Fix: the digits are rounded once from the exact rational, relative to the decimal point for
`f`/`%` and by significant digits for `e`/`g` (`round_to_digits`, `python/src/rational.rs:76`).

### py-general-format-never-scientific
Reports: E120
Verdict: real, fixed
Root cause: `'g'`/`'G'` were rendered as the plain decimal `Display` (`python/src/format.rs`).
Test: `python/tests/test_format.py::test_general_format`.
Fix: `render_general` (`python/src/format.rs:269`) implements Python's rule (scientific when the
exponent is below -4 or not below the precision, trailing zeros removed). Without a precision
the precision of the value is used, so nothing is hidden by default.

### py-hex-format-sign-and-padding
Reports: E183
Verdict: real, fixed
Root cause: `format_fbig_hex` passed the rendered string straight to the layout: the sign option
was never applied and the `=` padding did not know about the `0x` prefix.
Test: `python/tests/test_format.py::test_hex_format_sign_and_padding`.
Fix: sign and prefix are split off and handled by the shared layout (`apply_layout`,
`python/src/format.rs:377`).

### py-decimal-format-layout
Reports: E202
Verdict: real, fixed
Root cause: grouping was applied after the sign (the `' '` sign counted as a digit), the `=`
alignment did not treat `' '` as a sign, and zero-padding was added after grouping.
Test: `python/tests/test_format.py::test_decimal_layout_matches_float`.
Fix: `apply_layout` takes the sign separately and `group_digits` (`python/src/format.rs:349`)
groups the padding zeros as CPython does.

## Per-report table

| Report | Cluster | Verdict |
|---|---|---|
| D26 | cbig-decimal-integer-truncated | real, fixed |
| D37 | py-ubig-del-bits | real, fixed |
| E36 | cbig-decimal-integer-truncated | duplicate of D26 (real, fixed) |
| E49 | py-ubig-del-bits | duplicate of D37 (real, fixed) |
| E51 | py-fbig-decimal-format-digits | real, fixed |
| E66 | py-int-dbig-rounds | real, fixed |
| E67 | py-auto-float-precision | real, fixed |
| E70 | py-gcd-lcm-of-zeros | real, fixed |
| E71 | py-hash-disagrees-with-equality | real, fixed |
| E81 | py-zero-division-panics | real, fixed |
| E84 | py-rbig-math-rounding-protocol | real, fixed |
| E95 | float-literal-negative-zero | real, fixed |
| E100 | py-rbig-format-digits | real, fixed |
| E119 | py-ubig-subtraction-underflow | real, fixed |
| E120 | py-general-format-never-scientific | real, fixed |
| E124 | py-zero-division-panics | real, fixed (same class as E81) |
| E129 | rbig-denominator-without-slash | real, fixed |
| E132 | py-inplace-operators-reject-operands | real, fixed |
| E140 | py-ubig-empty-slice-panics | real, fixed |
| E164 | py-missing-reflected-operators | real, fixed |
| E171 | py-cbig-two-argument-constructor | real, fixed |
| E177 | py-cbig-equality-raises | real, fixed |
| E183 | py-hex-format-sign-and-padding | real, fixed |
| E202 | py-decimal-format-layout | real, fixed |
| E215 | guide-unlimited-precision-example | docs |
| E220 | py-index-beyond-isize | real, fixed |
| E226 | ibig-repeated-signs | real, fixed |
| E247 | static-float-precision-depends-on-size | real, fixed |
| E254 | rbig-denominator-without-slash | real, fixed |

## Changelog entries

`macros/CHANGELOG.md`, under `## Unreleased`:

```markdown
### Fix
- `cbig!` / `static_cbig!`: decimal and octal integer coefficients are converted to base 2 exactly (`cbig!(9)` was 8, `cbig!(99)` was 98). A coefficient with `n` digits now gets as many bits of precision as the largest `n`-digit integer needs, so `cbig!(3 + 4i)` has precision 4 (was 3).
- `fbig!` / `dbig!` / `cbig!` and their static variants keep the sign of a negative zero literal (`dbig!(-0.0)` was `+0`).
- `rbig!` / `static_rbig!` accept `base N` without a denominator (`rbig!(a base 16)` failed to compile).
- `rbig!` / `static_rbig!` reject two numbers that are not separated by `/` (`rbig!(1 - 2)` was -1/2) and repeated signs.
- `ibig!` / `static_ibig!` reject repeated or mixed signs (`ibig!(--5)` was -5).

### Change
- `static_fbig!` / `static_dbig!` produce a value with unlimited precision for every literal, as documented. Literals whose significand fits in 32 bits used to carry the precision of the literal.
```

`python/CHANGELOG.md`, under `## Unreleased`:

```markdown
### Add
- Reflected `%`, `divmod`, `&`, `|`, `^` and `**` for `UBig` / `IBig` (`7 % UBig(3)`, `2 ** UBig(10)`), and `UBig` / `IBig` exponents and moduli in `pow`.
- `math.floor`, `math.ceil`, `math.trunc` and `round()` for `RBig` (exact).
- `CBig` arithmetic and comparison accept `UBig` / `IBig`.

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

### Change
- `UBig` has no `__isub__` any more: `n -= m` is `n = n - m`.
- `pow(UBig, e, negative modulus)` returns an `IBig` when the result is negative instead of raising `ValueError`.
- `CBig(1) == "1"` is `False` (strings are not parsed in comparisons).
```

Root `CHANGELOG.md` / guide: the guide fix (E215) has no crate changelog; if the root changelog
tracks documentation, add under `## Unreleased` → `### Fix`:
`- Guide: the "Unlimited precision" examples in the float precision chapter (English and Chinese) asserted a precision that parsed values do not have.`

## Cross-group notes

Files touched outside `macros/`, `python/` and the guide: only `tests/import.rs` (root crate
macro test, updated for E247). No file of `float/`, `integer/`, `rational/` or `complex/` was
changed.

Root causes found in other groups' areas while working here (not fixed here):

- **float-convert** (related to E128 / E222): `FBig::from_parts_const` returns `ZERO` (unlimited
  precision, positive) for a zero significand, ignoring `min_precision` and the sign. The macros
  work around the sign (E95); the precision of zero literals (`dbig!(0.00).precision() == 0`,
  pinned by existing tests) still differs from `from_str`.
- **float-convert**: `FBig::with_base` picks `floor(precision * log(B)/log(NewB))` digits, which
  cannot hold every integer of the source precision (the root of D26/E36). The macro no longer
  relies on it.
- **float-fmt** (B12 / C1 / E13): `{:.Ne}` prints one fractional digit too many when rounding
  carries into a new digit. Visible through the binding as `format(DBig("999999.96"), ".1e") ==
  '1.00e+06'`; the binding will be right once that fix lands (`'g'` is not affected).
- **rational** (A12 / D1 / E35): `RBig::to_float` is not correctly rounded. `RBig.__format__` no
  longer uses it (it rounds the exact value itself); `RBig.to_decimal()` / `to_float()` in the
  binding still show it.
- **int-kernels** (E3): `Reduced::pow(&0)` with modulus 1 trips a debug assertion, so
  `pow(IBig(5), 0, 1)` panics in a debug build of the binding (pre-existing, not addressed here).
- **float-arith**: `%` on `DBig` / `RBig` in the binding follows the Rust `Rem` of those types
  (`Decimal("7.5") % UBig(2)` gives `-0.5`, `Fraction(7, 2) % UBig(2)` gives `-1/2`), which is
  not Python's floored modulo. Pre-existing for the forward operators; the new reflected `%`
  inherits it.

Pre-existing, noticed but out of scope: `python/src/cache.rs` has a duplicated match arm that
fails `cargo clippy -D warnings` on `dashu-python` (the crate is excluded from the CI lint); a
Python int beyond 64 bits is always taken as an `IBig` by the binary operators, even when
positive (`UBig(1) + 2**70` is an `IBig`).

## Unsure

- **E247 direction.** I made static float literals uniformly unlimited (what the docs say). The
  opposite fix (every static literal carries the literal's precision, like `fbig!`/`dbig!` and
  like `static_cbig!` already does) is arguably nicer but needs a new const constructor in
  `dashu-float` and changes results for wide static literals.
- **`cbig!` precision.** The precision of decimal coefficients grew by up to one bit
  (`ceil` instead of `floor` of `digits * log2(10)`). An alternative that keeps the old numbers
  for non-integers would make the precision depend on whether the coefficient is an integer.
- **Rounding of ties.** `format()` of `FBig` / `DBig` / `RBig` rounds half away from zero (the
  mode of `DBig`), where Python's `float` / `Decimal` / `Fraction` formatting rounds half to
  even (`format(FBig(0.5), ".0f")` is `'1'`, Python gives `'0'`); likewise `round(RBig)` follows
  `RBig.round()` and the existing `round(FBig)` / `round(DBig)` (ties away), not Python's
  `round` (ties to even). No report covers ties; I kept the binding's existing convention and
  documented it in `USAGE.md`.
- **In-place operators.** They still mutate in place where possible (so aliases see the change),
  which is unlike Python's `int`; only the fallback cases rebind.
