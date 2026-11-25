# Group `python` of run F (19 reports)

All 19 reports concern the PyO3 binding (`python/`). All 19 reproduced at `8137b6e` / `b0705ed`;
none was dismissed. They fall into 15 clusters, all real and all fixed in the binding. No core
crate was changed.

Provenance in short: 1 cluster is a **regression** of the earlier triage commit `740f086`
(F54); the other 14 clusters are **pre-existing** behaviour of `a0fc535` that the earlier runs
did not report (every reproducer fails the same way at the base). Three of them (F52, F53, F80)
had been noticed during the earlier triage and written down as out of scope in
`triage/python-macros.md` ("Cross-group notes" and "Unsure"). Two others (F62, F64) are the
neighbours of cases that the earlier commit fixed on the side, without a report asking for it:
it stopped where it did, and these reports are about what was left.

How things were verified:

- The binding was built with `maturin develop --features serde,rand,rkyv,zeroize` (debug
  profile, CPython 3.13) into three scratch virtualenvs: this branch, a copy of `b0705ed`
  (`git archive`), and a copy of the base `a0fc535` (`git archive`). Every reproducer was run
  against all three before any change (one script, `repro.py` in the scratch directory), which
  is where each "Base behaviour" line below comes from.
- `pytest python/tests`: 140 passed with all features, 102 passed and 4 skipped without optional
  features (118 passed before the changes with all features).
- A grid of mixed-type `+ - * / % //` (4536 operator applications) was diffed between the
  `b0705ed` build and this branch: the only differences are the intended ones (`%` on `FBig`/`DBig`/`RBig`,
  `/` between integers, the type of `UBig op big int`, and 24 panics that became
  `ZeroDivisionError`), and no pair panics any more. `%` was also checked against
  `fractions.Fraction` on 353 mixed pairs.
- `cargo fmt --all -- --check` and `cargo clippy --all-features --all-targets --workspace
  --exclude dashu-python -- -D warnings` are clean. `cargo clippy` on `dashu-python` itself shows
  the same warnings as on `b0705ed` (the duplicated match arms in `python/src/cache.rs`, which
  were there before) and no new one; `cargo fmt --check` on `python/` is clean.
- Not run: `mdbook build` (the guides include `python/USAGE.md` / `USAGE-zh.md`; no heading or
  anchor changed), and a release build of the binding.

## Clusters

### py-rational-float-modulo-convention
Reports: F52
Verdict: real, fixed
Root cause: `%` on `RBig`, `FBig` and `DBig` applied the Rust `Rem` of the type
(`python/src/rational.rs` `impl_rpy_divops!`, `python/src/float.rs` `float_rem`), whose result is
the remainder of the least magnitude for `RBig` and `DBig` (`5 % 3 == -1`) and the truncated one
for `FBig` (`-5.0 % 3 == -2`). The integers already implemented Python's floored modulo, so
`UBig(5) % 3 == 2` but `UBig(5) % RBig(3) == -1`.
Base behaviour: the same wrong values at `a0fc535` (`RBig(5) % 3 == -1`, `5 % RBig(3) == -1`,
`RBig(7/2) % 1 == -1/2`; verified by running).
Provenance: **pre-existing** for the forward operators. The reflected `%` of the integers
(`UBig(5) % RBig(3)`, new in `740f086`) inherited it; the earlier notes recorded it as a known
leftover under float-arith.
Test: `python/tests/test_ratio_ops.py::test_mod_is_floored`,
`python/tests/test_float_ops.py::test_mod_is_floored`.
Fix: `rbig_mod_floor` (`python/src/rational.rs:61`) computes `a - b * floor(a / b)`; `float_rem`
(`python/src/float.rs:109`) adds the divisor to a remainder of the other sign. The report names
`RBig` only; the float types have the same root cause and are fixed with it.
Notable: for floats the floored remainder can need more digits than the operands have (a tiny
negative dividend), and is then rounded to the precision of the operands, as for Python's
`float`.

### py-inplace-operators-mutate
Reports: F53
Verdict: real, fixed
Root cause: `UBig` / `IBig` defined `__iadd__`, `__imul__`, `__iand__`, `__ior__`, `__ixor__`,
`__ilshift__`, `__irshift__` (and `IBig.__isub__`) as in-place updates of the Rust value
(`python/src/int.rs`), so `a = b = UBig(5); a += 1` changed `b`, and a member of a `set` changed
under the set (the types are hashable).
Base behaviour: the same at `a0fc535`, and `UBig -=` mutated in place there too (verified by
running: alias and set member changed).
Provenance: **pre-existing**. `740f086` only removed `UBig.__isub__` and made the remaining
operators fall back for operands they cannot take; it also wrote "as for Python's int" into
`USAGE.md`, which the remaining in-place updates contradict. The earlier notes listed it under
"Unsure".
Test: `python/tests/test_int_math.py::test_augmented_assignment_leaves_other_references_alone`.
Fix: the in-place operators are removed from both types (and `UnsignedOperand` / `SignedOperand`
from `python/src/convert.rs`), so `n op= m` is `n = n op m` and binds a new object.
`python/dashu.pyi`, `USAGE.md` and `USAGE-zh.md` updated.
Notable: this is a behaviour change that needs a maintainer's decision (it costs a copy for
`n += 1` on a large integer). The explicit mutators (`n[i] = True`, `del n[a:b]`, `n.remove(f)`,
`n.zeroize()`) still change the object, as their names say.

### py-rbig-format-negative-zero
Reports: F54
Verdict: real, fixed
Root cause: `RBig.__format__` for `'f'` / `'F'` / `'%'` rounds the exact value to an integer
significand (`python/src/rational.rs`, the `'f' | 'F' | '%'` arm), and a zero significand has no
sign, so a negative value that rounds to zero printed as `'0.00'`.
Base behaviour: correct at `a0fc535` (`format(RBig(-1/1000), '.2f') == '-0.00'`, `'-0.0%'`,
`'-0'`; verified by running): the base converted to a float first, which kept the sign.
Provenance: **regression**, introduced by `740f086` (cluster py-rbig-format-digits, E100), which
replaced the float conversion by the exact rounding.
Test: `python/tests/test_format.py::test_rational_negative_rounding_to_zero_keeps_sign`.
Fix: a negative value whose scaled rounding is zero is rendered from `Repr::neg_zero()`
(`python/src/rational.rs:190`).

### py-math-operand-precision
Reports: F55, F60, F73
Verdict: real, fixed
Root cause: (a) the module-level functions converted an exact operand with `into_fpy()`, which
gives an integer its own bit length as precision, so `dashu.sqrt(2)` was computed at 2 bits
(`python/src/math.rs`); (b) `hypot` / `atan2` / `powf`, and the `atan2` / `powf` methods of
`FBig` / `DBig` (`python/src/float.rs`), took the context of the first operand only, so the
result depended on the order of the operands.
Base behaviour: the same at `a0fc535` (`dashu.sqrt(2) == 1`, `hypot(3, 4.0) == 4`,
`y.atan2(x)` at 4 bits; verified by running).
Provenance: **pre-existing**.
Test: `python/tests/test_math.py::{test_exact_operands_use_the_default_precision,
test_two_operand_functions_use_the_larger_precision}`.
Fix: `UniInput::into_math_fpy` (`python/src/convert.rs:347`) takes an exact operand (`int`,
`UBig`, `IBig`, `RBig`, `Fraction`) at the module default precision, or exactly if it has more
digits; the two-operand functions and methods use `Context::max` of the two operands, like the
arithmetic operators. `USAGE.md` / `USAGE-zh.md` updated.
Notable: a rational that is not a binary fraction (`dashu.sqrt(Fraction(1, 3))`) used to raise
"precision loss"; it is now rounded to the default precision first. The *methods* are unchanged
for a single operand: `FBig(2).sqrt()` still runs at the 2 bits of `FBig(2)`, as documented.
`CBig.powf` still uses the precision of the receiver, like the Rust `CBig::powf` (not reported,
left alone).

### py-unlimited-precision-transcendentals
Reports: F85, F96
Verdict: real, fixed
Root cause: the Rust transcendentals panic on a context of unlimited precision (0), and the
binding handed them such values: `trunc` / `floor` / `ceil` / `round` / `fract` return a result
of precision 0 for a zero and wherever no digit of the number is left (documented in
`float/src/round_ops.rs`), `round(DBig, n)` likewise, and `zeroize()` zeroes the precision with
the buffers (`float/src/third_party/zeroize.rs`).
Base behaviour: the same panics at `a0fc535` (verified by running both reports' cases through
the binding; F96 itself gives a Rust program, because the pipeline could not build the binding).
Provenance: **pre-existing**.
Test: `python/tests/test_float_ops.py::{test_rounding_methods_keep_a_precision,
test_unlimited_precision_raises_instead_of_panicking}`,
`python/tests/test_zeroize.py::test_zeroized_float_keeps_its_precision`.
Fix: in the binding only. `keep_precision` (`python/src/float.rs:166`) gives the result of a
rounding method the precision of the receiver where the Rust method returns precision 0 (exact:
the result never has more digits); `zeroize()` of `FBig` / `DBig` / `CBig` keeps the precision;
and `limited` / `limited_complex` (`python/src/cache.rs:38`) turn the remaining unlimited
operands (`x.with_precision(0)`, deserialized values) into a `ValueError` for every function
that rounds its result (transcendentals, roots, `powf`, `atan2`, `hypot`, negative `powi`,
`CBig.abs` / `arg`). `USAGE.md` / `USAGE-zh.md` updated.
Notable: the guard is uniform, so a few special cases that happened to work at precision 0
because their result is exact (`FBig(0.0).with_precision(0).sin()`, `ln(1)`, `sqrt(0)`) now
raise `ValueError` as well; a non-negative `powi` and `CBig.norm` are exact and still work. The
core behaviour (rounding methods and `Zeroize` returning precision 0) was not changed; whether
`Zeroize for Context` should reset the precision is a question for the float group.

### py-infinity-arithmetic-panics
Reports: F62
Verdict: real, fixed
Root cause: the arithmetic dispatchers called the Rust operators directly, which panic on an
infinite operand and on `0 / 0` (`impl_float_binops!` in `python/src/float.rs`, the float arms
of `impl_binops!` in `python/src/int.rs`, `impl_cpy_binops!` in `python/src/complex.rs`).
Base behaviour: the same panics at `a0fc535` for `+ - * /` and for `0 / 0`; there `%` with an
infinity panicked too (verified by running).
Provenance: **pre-existing**. The earlier cluster py-zero-division-panics (E81 `x % 0`, E124
`pow(x, e, 0)`) made `%` with an infinite operand raise `ValueError` on the side and kept "`/` by
a float zero gives an infinity"; nobody had reported `+ - * /` with an infinity or the float
`0 / 0`, and `740f086` did not touch them.
Test: `python/tests/test_float_ops.py::{test_arithmetic_with_infinity_raises,
test_zero_over_zero_raises}`.
Fix: `float_add` / `float_sub` / `float_mul` / `float_div` (`python/src/float.rs:29`–) check the
operands first: an infinite operand raises `ValueError("arithmetic with infinity")` (what `%`
and the transcendentals already do), `0 / 0` raises `ZeroDivisionError`, and a division of two
operands of unlimited precision raises `ValueError`. The integer dispatchers promote the integer
and call the same functions; `CBig` has the same checks (`python/src/complex.rs:99`).
`USAGE.md` / `USAGE-zh.md` updated.
Notable: the report would also accept IEEE results (`inf + 1 == inf`). I followed the convention
the binding already had for `%` and the transcendentals (and the core library's "arithmetic
with the infinity is not allowed"), because there is no NaN for `inf - inf` / `inf * 0`. This
is a maintainer's decision. `x / 0` for a non-zero float is still an infinity.

### py-infinity-method-panics
Reports: F64
Verdict: real, fixed
Root cause: `.trunc()` / `.floor()` / `.ceil()` / `.round()` / `.fract()` of `FBig` / `DBig`,
`round(DBig, n)`, `.to_int()` and `.digits()` called the Rust methods, which panic on an
infinity (`python/src/float.rs`).
Base behaviour: the same panics at `a0fc535`, and `math.floor(x)` / `round(x)` / `int(x)`
panicked there as well (verified by running).
Provenance: **pre-existing**. The fix for py-int-dbig-rounds (E66, about `int(DBig)` rounding)
made `int`, `math.floor` / `ceil` / `trunc` and `round(x)` of an infinity raise `OverflowError`
on the side, which is why those now differ from the methods of the same names; the methods,
`round(x, n)`, `to_int()` and `digits()` were not reported then and not changed.
Test: `python/tests/test_float_ops.py::test_methods_of_infinity_raise`.
Fix: `finite` (`python/src/float.rs:152`) raises `ValueError("arithmetic with infinity")` for
the rounding methods, `round(x, n)` and `digits()`; `to_int()` raises `OverflowError` like
`int(x)`.
Notable: `.to_int()` and `.digits()` are not in the report; same root cause, found while
checking every method on an infinity.

### py-dbig-from-float-infinity
Reports: F67
Verdict: real, fixed
Root cause: a Python float is converted to a `DBig` through its shortest decimal string
(`into_dpy` / `construct_dpy` in `python/src/convert.rs`), and `"inf"` is not a number for the
parser, hence `SyntaxError: invalid digit for the given radix`.
Base behaviour: the same `SyntaxError` at `a0fc535` (verified by running).
Provenance: **pre-existing**.
Test: `python/tests/test_float_ops.py::test_dbig_from_float_infinity`.
Fix: `dbig_from_f64` (`python/src/convert.rs:236`) maps a float infinity to
`DBig::INFINITY` / `NEG_INFINITY`.
Notable: `DBig(1) + float('inf')` now raises `ValueError` (arithmetic with infinity, see
py-infinity-arithmetic-panics) instead of `SyntaxError`; the report expected an infinity there.

### py-int-true-division-truncates
Reports: F63
Verdict: real, fixed
Root cause: `/` between integers used the Rust `Div` of `UBig` / `IBig`, the division truncated
toward zero (`impl_binops!(.., div)` in `python/src/int.rs`): `UBig(7) / 2 == 3`, and
`IBig(-7) / 2 == -3` while `IBig(-7) // 2 == -4`.
Base behaviour: the same at `a0fc535` (verified by running).
Provenance: **pre-existing**.
Test: `python/tests/test_int_math.py::test_integer_true_division_is_exact`.
Fix: `int_truediv` / `int_rtruediv` (`python/src/int.rs:338`) return the exact quotient as an
`RBig` for two integers. `python/dashu.pyi`, `USAGE.md`, `USAGE-zh.md` updated.
Notable: needs a maintainer's decision. Python's `int / int` gives a `float`; an arbitrary
precision library has an exact answer, so I chose the rational (it compares equal to `3.5`,
converts with `float()`, and agrees with `//` under `math.floor`). The alternative is an `FBig`
at the default precision. Either way the result type changes from `UBig`/`IBig`.

### py-ubig-large-int-type
Reports: F80
Verdict: real, fixed
Root cause: a Python int that does not fit in 64 bits is extracted as `UniInput::OBInt` (a
signed big integer), and every `UBig` operator returned an `IBig` for that variant whatever its
sign (`impl_binops!`, `impl_ubig_bit_binops!`, `upy_sub_any`, `upy_rsub_any` in
`python/src/int.rs`).
Base behaviour: the same types at `a0fc535` for the report's five subtractions (verified by
running), and likewise for `+`, `*`, `|`, `^`.
Provenance: **pre-existing** behaviour; the rule it violates ("also with a non-negative `int` on
either side") was written by `740f086` (`unsigned_sub`), which handled only ints below 2^63.
The earlier notes recorded it as noticed and out of scope.
Test: `python/tests/test_int_math.py::test_ubig_with_large_int_stays_ubig`.
Fix: a non-negative `OBInt` is treated like a `UBig` operand in `+`, `*`, `|`, `^` and in both
orders of `-` (`python/src/int.rs:79`, `:169`, `:216`, `:224`).

### py-ibig-chunks-sign
Reports: F61
Verdict: real, fixed
Root cause: `IBig.to_chunks` chunked `unsigned_abs()` without saying so
(`python/src/int.rs:1220`), so the round trip through `from_chunks` turned `x` into `abs(x)`.
Base behaviour: the same at `a0fc535` (verified by running).
Provenance: **pre-existing**.
Test: `python/tests/test_int.py::test_ibig_chunks_of_negative_raise`.
Fix: `IBig.to_chunks` of a negative number raises `OverflowError` ("can't convert negative int
to unsigned"), as `to_bytes(signed=False)` does. `python/dashu.pyi` notes it.
Notable: the chunks are `UBig`s and cannot carry a sign, so the alternative (a signed round
trip) would need a new format. A caller who wants the magnitude writes `abs(x).to_chunks(k)`.

### py-ibig-to-bytes-unsigned
Reports: F74
Verdict: real, fixed
Root cause: `IBig.to_bytes` returned the two's complement encoding of the `IBig` whatever
`signed` was (`python/src/int.rs:1267`); `signed=False` only rejected negative values.
Base behaviour: the same at `a0fc535` (`IBig(200).to_bytes('big', False) == b'\x00\xc8'`;
verified by running).
Provenance: **pre-existing**.
Test: `python/tests/test_int.py::test_ibig_to_bytes_unsigned_has_no_sign_byte`.
Fix: with `signed=False` the bytes of the magnitude are returned, as `UBig.to_bytes` gives them.
Notable: `signed` is a required positional argument of `IBig.to_bytes` (it has no default in the
PyO3 signature, so `IBig(5).to_bytes('big')` is a `TypeError`); not reported, left alone.

### py-rbig-to-float-zero-precision
Reports: F70
Verdict: real, fixed
Root cause: `RBig.to_float(0)` / `to_decimal(0)` passed the precision to `RBig::to_float`, which
asserts `precision > 0` (`python/src/rational.rs`).
Base behaviour: the same panic at `a0fc535` (verified by running).
Provenance: **pre-existing**.
Test: `python/tests/test_ratio_ops.py::test_to_float_needs_a_precision`.
Fix: `check_precision` (`python/src/rational.rs:79`) raises `ValueError`.

### py-format-fill-with-zero-width
Reports: F84
Verdict: real, fixed
Root cause: the spec parser took a `0` before the width for the zero-padding flag even after an
explicit fill character (`python/src/format.rs`, `parse`), and the flag then replaced the fill
and the alignment. In CPython the `0` is only the first digit of the width in that case.
Base behaviour: the same output at `a0fc535` (`'1.50000000'`, `'000000-2.5'`; verified by
running).
Provenance: **pre-existing** (the layout code was reworked by `740f086` for E183 / E202, but the
parser was not, and the output is the same before and after).
Test: `python/tests/test_format.py::test_explicit_fill_with_leading_zero_width`.
Fix: the `0` sets the zero-padding flag only when no fill character was given
(`python/src/format.rs:43`). Applies to `FBig`, `DBig`, `RBig` and the hex presentation.

### py-cbig-format-layout
Reports: F65
Verdict: real, fixed
Root cause: `CBig.__format__` formatted the real and the imaginary part separately with the
whole spec (`python/src/complex.rs`), so width, fill and alignment were applied twice and inside
the number, and the `' '` sign option reached the imaginary part (`'+     -4.00'`).
Base behaviour: the same output at `a0fc535` (verified by running).
Provenance: **pre-existing**.
Test: `python/tests/test_format.py::test_complex_layout`.
Fix: `format_complex` (`python/src/format.rs:209`) formats each part without layout (the
imaginary part with a forced sign) and pads the whole `(re+imj)` text once. Zero-padding and the
`=` alignment raise `ValueError`, as Python's `complex` does. `USAGE.md` / `USAGE-zh.md` updated.
Notable: the padding goes outside the parentheses (as Python does for the parentheses it adds
itself, `format(3+4j, '12')`); the report's own test would put it inside for a width above the
length of the number, which is the one detail where I did not follow the report.

### py-words-repeat-huge
Reports: F95
Verdict: real, fixed
Root cause: `Words.__mul__` computed `count * len` unchecked and reserved it with
`Vec::with_capacity` (`python/src/words.rs`): a "capacity overflow" panic, or an abort of the
interpreter when the allocation fails.
Base behaviour: the same at `a0fc535`: `Words([1]) * 2**62` raises `PanicException` and
`Words([1]) * 2**58` aborts the process (both verified by running). The report's `2**40` was not
run on either tree: on this machine (macOS) an 8 TiB reservation can succeed lazily, and filling
it is not a safe experiment.
Provenance: **pre-existing**.
Test: `python/tests/test_words.py::{test_words_repeat, test_words_repeat_too_large}` (each case
in a subprocess).
Fix: `checked_mul` and `try_reserve_exact`, both mapped to `MemoryError`
(`python/src/words.rs:232`).
Notable: on a system that overcommits, a count such as `2**40` can still pass the reservation
and then exhaust memory while filling, exactly as `[1] * 2**40` can; the test uses counts that
no address space can hold.

## Per-report table

| Report | Cluster | Verdict | Provenance |
|---|---|---|---|
| F52 | py-rational-float-modulo-convention | real, fixed | pre-existing |
| F53 | py-inplace-operators-mutate | real, fixed | pre-existing |
| F54 | py-rbig-format-negative-zero | real, fixed | regression (`740f086`) |
| F55 | py-math-operand-precision | real, fixed | pre-existing |
| F60 | py-math-operand-precision | duplicate of py-math-operand-precision | pre-existing |
| F61 | py-ibig-chunks-sign | real, fixed | pre-existing |
| F62 | py-infinity-arithmetic-panics | real, fixed | pre-existing |
| F63 | py-int-true-division-truncates | real, fixed | pre-existing |
| F64 | py-infinity-method-panics | real, fixed | pre-existing |
| F65 | py-cbig-format-layout | real, fixed | pre-existing |
| F67 | py-dbig-from-float-infinity | real, fixed | pre-existing |
| F70 | py-rbig-to-float-zero-precision | real, fixed | pre-existing |
| F73 | py-math-operand-precision | duplicate of py-math-operand-precision | pre-existing |
| F74 | py-ibig-to-bytes-unsigned | real, fixed | pre-existing |
| F80 | py-ubig-large-int-type | real, fixed | pre-existing |
| F84 | py-format-fill-with-zero-width | real, fixed | pre-existing |
| F85 | py-unlimited-precision-transcendentals | real, fixed | pre-existing |
| F95 | py-words-repeat-huge | real, fixed | pre-existing |
| F96 | py-unlimited-precision-transcendentals | duplicate of py-unlimited-precision-transcendentals | pre-existing |

## Changelog entries

`python/CHANGELOG.md`, under `## Unreleased`:

### Change
- `/` between integers (`UBig` / `IBig` / `int`) returns the exact quotient as an `RBig` instead of the quotient truncated toward zero (`UBig(7) / 2` was `3`).
- `UBig` and `IBig` have no in-place operators any more: `n += m` (and `-=`, `*=`, `&=`, `|=`, `^=`, `<<=`, `>>=`) is `n = n + m` and binds a new object, so aliases and `set` members keep their value.
- `%` on `RBig`, `FBig` and `DBig` is Python's floored modulo (the result has the sign of the divisor), as it already was for the integers (`RBig(5) % 3` was `-1`, `FBig(-5.0) % 3` was `-2`).
- The module-level math functions take an exact operand (`int`, `UBig`, `IBig`, `RBig`, `Fraction`) at the default precision (`dashu.sqrt(2)` was `1`), and `hypot` / `atan2` / `powf` (also the `atan2` / `powf` methods) work at the larger precision of their two operands.
- `IBig.to_chunks` of a negative number raises `OverflowError` instead of returning the chunks of its absolute value.
- `trunc()` / `floor()` / `ceil()` / `round()` / `fract()` of `FBig` / `DBig`, `round(DBig, n)` and `zeroize()` keep the precision of the float where they returned a value of unlimited precision.
- `format(CBig, spec)` rejects zero-padding and the `=` alignment, as Python's `complex` does.

### Fix
- `format(RBig, '.2f')` keeps the sign of a negative value that rounds to zero (`'-0.00'`).
- `+ - * /` with an infinite `FBig` / `DBig` / `CBig` operand raise `ValueError`, and a float `0 / 0` raises `ZeroDivisionError`, instead of a Rust panic.
- `.trunc()` / `.floor()` / `.ceil()` / `.round()` / `.fract()` / `.digits()` and `round(x, n)` of an infinity raise `ValueError`, and `.to_int()` raises `OverflowError`, instead of a Rust panic.
- Transcendentals, roots and `/` on a value of unlimited precision (`with_precision(0)`) raise `ValueError` instead of a Rust panic.
- `DBig(float('inf'))` is an infinity instead of raising `SyntaxError`.
- `RBig.to_float(0)` / `RBig.to_decimal(0)` raise `ValueError` instead of a Rust panic.
- `UBig` combined with a non-negative `int` of 2^63 or more gives a `UBig` (it gave an `IBig`).
- `IBig.to_bytes(byteorder, False)` returns the unsigned encoding, without the extra sign byte (`IBig(200)` gave `b'\x00\xc8'`).
- `format()` of `FBig` / `DBig` / `RBig` keeps an explicit fill character when the width starts with `0` (`'x<010.2f'`).
- `format(CBig, spec)` applies the width, fill and alignment once to the whole number, and the `' '` sign option to the real part only.
- `Words * n` raises `MemoryError` for a length that cannot be allocated, instead of a panic or an abort of the interpreter.

## Cross-group notes

- Files touched: only `python/` (`src/`, `tests/`, `dashu.pyi`, `USAGE.md`, `USAGE-zh.md`) and
  this notes file. No core crate, no `CHANGELOG.md`.
- No report in this group has its root cause in another group's area. Two core behaviours sit
  behind py-unlimited-precision-transcendentals and were worked around in the binding rather
  than changed: `Zeroize for Context` (`float/src/third_party/zeroize.rs`) resets the precision
  to 0 (unlimited), and the rounding methods of `FBig` return precision 0 for a zero result
  (documented). If the float group changes either, the binding's workarounds become no-ops.
- `python/src/cache.rs` still has the duplicated `ZivRetryLimitExceeded` match arms (clippy
  "unreachable pattern", pre-existing, listed in `SUMMARY.md` as a known leftover); not touched.

## Unsure

- **Infinities (F62, F64).** I chose `ValueError` for any arithmetic or rounding on an infinity,
  following what `%` and the transcendentals already did. IEEE-style results (`inf + 1 == inf`,
  `floor(inf) == inf`, `1 / inf == 0`) with an error only for the indeterminate forms would be
  the other consistent choice.
- **`/` between integers (F63).** `RBig` versus `FBig` at the default precision, see the cluster.
- **Removing the in-place operators (F53).** Correct for a hashable value type, but it removes an
  optimisation the author may have wanted.
- **Unlimited precision (F85, F96).** A `ValueError` for every rounding function at precision 0
  is uniform but turns a handful of exact special cases that worked into errors. Falling back to
  the default precision for `FBig` would be friendlier, but `DBig` has no default.
- **`IBig.to_chunks` (F61).** Raising for a negative number versus a signed chunk format.
