# Group `rational-macros`, run F (10 reports)

Area: `dashu-ratio` (`rational/`) and `dashu-macros` (`macros/`). Reports F29, F31, F32, F35,
F66, F83, F91 (rational) and F40, F59, F87 (macros). 9 clusters: 8 real and fixed, 1 judged a
documentation issue (F83, with a related real defect found and fixed alongside).

How things were verified. Every reproducer was run in a debug build against three trees: a
pristine copy of the tree the run was made against (`git archive b0705ed`, identical to
`8137b6e` outside `triage/`), the base checkout `a0fc535`, and this worktree after the fixes
(throwaway crates with path dependencies; the macro reports as compile probes). "Verified by
running" below means exactly that. Release builds were run where a cluster says so. Line
numbers under "Root cause" refer to the tree before the fixes (`b0705ed`).

The new tests were also run against the unfixed tree (the pristine copy with only the test
files copied over): every test named under a cluster fails there, except
`random.rs::test_uniform01_smallest_limits` and `io.rs::test_from_str_expanded_large_exponent`
(supporting properties that already held), `macros/tests/int.rs::test_base_marker` (accepted
forms only; the rejection is covered by the parser's unit test and the `compile_fail` doctests,
and was confirmed on the unfixed tree by a compile probe), and `macros/tests/cmplx.rs`, which
does not compile there (the literals are rejected). `test_from_str_expanded_unrepresentable_*`
and the two `pow` tests fail in debug (a different panic: arithmetic overflow) and in release
(no panic: the value 1). With the fixes: `cargo test --workspace --exclude dashu-python` passes;
`dashu-ratio` also passes with `--all-features --features dashu-int/num-order` and with
`--no-default-features`; `cargo fmt --all -- --check` and the workspace clippy command are
clean. Not run: the Dylint house lints (`cargo dylint` is not installed here), an MSRV build
(the new code was only reviewed for newer APIs), and the `fuzz/` crate (nothing here concerns
`FBig`/`CBig` arithmetic).

Provenance summary: 8 clusters **pre-existing** (the base fails the same way), 1 **regression**
(F66, introduced by the fix for E111). No cluster is in code that the earlier fixes added.

## Clusters

### serde-struct-zero-denominator
Reports: F29
Verdict: real, fixed
Root cause: the struct form of the serde deserializer (`ReprVisitor::visit_seq` / `visit_map`, `rational/src/third_party/serde.rs:109-155`) builds the `Repr` from the two integers as they come, with no check of the denominator. `RBig` then reduced `3/0` to the invalid `1/0`, and `Relaxed` panicked in `Repr::reduce2` (`trailing_zeros().unwrap()` of a zero denominator, `rational/src/repr.rs:58`).
Base behaviour: the same for the binary form (verified by running: `Ok(1 / 0)` and the same panic). The string form `"3/0"` through serde_json was *accepted* at the base; it is rejected now because the parser fix for A11/D41/E76 also sits under the human-readable path. So the earlier fix made the two forms disagree, which is what the report noticed.
Provenance: **pre-existing**. The binary path was not touched by the earlier work and was not reported before; it is the same class of defect as `parse-zero-denominator` (A11), whose fix covered the text parsers only.
Test: `rational/tests/serde.rs::test_serde_zero_denominator` (postcard for both types and numerators 3, -4, 0; the named-field form through `serde_test`; a non-reduced valid payload still reads).
Fix: `deserialize_repr` returns `invalid_value` ("a zero denominator") for a zero denominator in the struct form (`rational/src/third_party/serde.rs`).
Notable: the rkyv impls (derived on the struct) were not changed and not tested for this; an archive holding a zero denominator presumably still deserializes to an invalid value.

### in-expanded-padding
Reports: F31
Verdict: real, fixed
Root cause: `Display` / `LowerExp` / `UpperExp` for `InExpanded` wrote the sign and the digits straight to the formatter (`rational/src/fmt/expanded.rs:187`, `:316`) and never looked at the width, fill, alignment or `0` flag.
Base behaviour: the same (`"0.50" "0.50" "5.00e-1"`, verified by running).
Provenance: **pre-existing**. (E111 fixed the same omission for `in_radix` and the fraction formats, not for `in_expanded`; it was not reported for `in_expanded` before.)
Test: `rational/tests/io.rs::test_rbig_in_expanded_width_fill_alignment` (the report's three cases, comparison with `f64`, default right alignment, centre, `+`, the exact `#` form, sign-aware zero padding, zero, a non-decimal radix, `Relaxed`).
Fix: the two renderers write the magnitude to any `fmt::Write` (`write_fixed`, `write_scientific`); `fmt_padded` writes sign and magnitude directly when there is no width (no allocation, as before) and otherwise renders the magnitude to a `String` and lets `Formatter::pad_integral` place sign, fill and zeros (`rational/src/fmt/expanded.rs`). The struct docs state it.

### numhash-relaxed-modulus-factor
Reports: F32
Verdict: real, fixed
Root cause: `NumHash for Repr` (`rational/src/third_party/num_order.rs:89-113`) hashes `n * d^-1 mod (2^127 - 1)` and falls back to the "infinity" hash when the denominator is divisible by the modulus. That is right for a reduced fraction, but a `Relaxed` only cancels powers of two, so `p/p` (or `3p/5p`) took the infinity branch although its value is 1 (or 3/5).
Base behaviour: the same (verified by running: `eq=true same_hash=false` for `k = p`, also for `(1/p) * p`).
Provenance: **pre-existing**.
Test: `rational/tests/num_order.rs::test_hash_relaxed_common_factor_of_modulus` (the report's three `k`, the arithmetic route, and a table of values with the modulus cancelling once or twice, including integers divisible by the modulus and denominators that keep the modulus after cancelling).
Fix: when the modulus divides both the numerator and the denominator, it is cancelled (repeatedly) before hashing (`rational/src/third_party/num_order.rs`). Only that rare case does any extra work; reduced fractions hash exactly as before.

### uniform01-empty-limit
Reports: F35, F91
Verdict: real, fixed (F91's expected behaviour — that `new_closed(&1)` yields 0 and 1 — is not what was implemented, see Notable)
Root cause: `Uniform01` never validated its limit (`rational/src/third_party/rand.rs:43-125`). In `(0, 1)` the sampler rejects a zero numerator in a loop; with limit 1 the numerator is always 0, so it never ends (F35). In `[0, 1]` the denominator is `limit - 1`, i.e. 0 for limit 1, and `0/0` reduces to 0 on every draw (F91). A zero limit only failed on a `debug_assert!` inside `dashu-int`.
Base behaviour: the same (verified by running: `open 1` killed after 5 s, `closed 1` always `0 / 1`, limit 0 fails the same debug assertion).
Provenance: **pre-existing**.
Test: `rational/tests/random.rs::test_uniform01_smallest_limits` (the exact support of each interval for the smallest valid limits and the next one) and `::test_uniform01_{open_limit_one, open_limit_zero, closed_limit_one, closed_limit_zero, half_open_limit_zero, open_closed_limit_zero}` (`#[should_panic]` with the message).
Fix: the four constructors panic when the limit leaves no rational to choose from: below 1 for `[0, 1)` and `(0, 1]`, below 2 for `[0, 1]` and `(0, 1)` (`Uniform01::with_bounds`). Each constructor documents the values it generates and its `# Panics`.
Notable: F91 expects `new_closed(&1)` to return both 0 and 1. The module docs state that the closed interval uses the denominator `limit - 1` (so that every interval has `limit` or `limit - 1` equally likely values), which leaves no valid denominator for limit 1; I kept that documented scheme and made the degenerate limit an error instead of silently returning 0. Changing the closed interval to `k / limit, 0 <= k <= limit` would give the report's answer but changes the output of `new_closed` for every limit — a maintainer decision. The panic now happens in the constructor, not at the first sample (for limit 0 it used to be a debug assertion in `dashu-int` at sampling time; what a release build did with a zero limit was not checked).

### fraction-zero-pad-prefix
Reports: F66
Verdict: real, fixed
Root cause: `fmt_fraction` (`rational/src/fmt/mod.rs:18-35`) renders `0x1/0x2` into one string and hands it to `Formatter::pad_integral` with an empty prefix, so with the `0` flag the zeros went in front of the radix prefix of the numerator (`000000x1/0x2`).
Base behaviour: different (verified by running). The base forwarded the formatter to the numerator and the denominator separately: `{:#012x}` of 1/2 was `0x0000000001/0x0000000002` — the width applied twice (that is E111), but the zeros were after each prefix and the text parsed back.
Provenance: **regression**, introduced by `da6a507` ("ratio: apply format flags to a fraction as a whole", the fix for E111). Precisely: the base output was wrong in another way (25 characters for a width of 12), but it satisfied the property of this report; the E111 fix made the width right and misplaced the zeros.
Test: `rational/tests/io.rs::test_format_zero_padding_after_radix_prefix` (all the report's cases, `+`, `Relaxed`, integer values, the unchanged non-`#` and explicit-alignment forms, and a round trip through `from_str_with_radix_prefix` for `x`/`X`/`b`/`o`).
Fix: `fmt_fraction` takes the radix prefix of the format (`0b` / `0o` / `0x`, empty for `Display` and `in_radix`) and passes it to `pad_integral` separately, which writes sign, prefix, zeros, digits in that order: `0x000001/0x2`, `-0x00003/0x4`.

### expanded-parse-huge-exponent
Reports: F83
Verdict: docs for the reported out-of-memory panic (the behaviour is inherent; the docs did not say so); plus a related wrong value found while reproducing — real, fixed.
Root cause: `Repr::from_str_expanded` builds the exact scale factor `radix^|exponent|` (`rational/src/parse.rs:201-202`). `1e999999999999` denotes an integer of 10^12 digits (about 415 GB); no algorithm can return that value, and `ParseError` has no variant for "too large" (the enum is exhaustive, so adding one is a breaking change in `dashu-base`). The parser therefore fails the way every other oversized integer operation in the library does (`UBig::pow`, shifts): with the allocation panic. An exponent cap would be an arbitrary policy (the value is representable on a large enough machine), so none was added. This is the leftover that the E178 notes recorded ("fixed for zero literals only"); the report objects to it, and I think the earlier decision stands, but it should have been documented — it was not.
  The related defect: `UBig::pow` / `IBig::pow` computed the bit count of the power-of-two part as `exp * shift` unchecked (`integer/src/pow.rs:30`, `:59`). For `16^(2^62)` the product wraps to 0, so in a release build `UBig::from(16u8).pow(1 << 62) == 1` and `RBig::from_str_expanded("1@4611686018427387904", 16) == Ok(1)` — a wrong value where the huge-exponent failure should be (debug builds panic with "attempt to multiply with overflow").
Base behaviour: the nonzero huge-exponent path is the same code at the base (`rational/src/parse.rs:164-165` there); I did not run `1e999999999999` at the base, see Notable. The wrap was verified by running a release build of the base: `pow 16 4611686018427387904 -> 1`, `1@4611686018427387904 (radix 16) -> Ok((1, 1))`, same as before the fix here.
Provenance: **pre-existing** (both parts). For the out-of-memory part it is the known, deliberately unfixed remainder of E178.
Test: `rational/tests/io.rs::test_from_str_expanded_large_exponent` (moderately large exponents give the exact value), `::test_from_str_expanded_unrepresentable_exponent` and `::test_from_str_expanded_unrepresentable_negative_exponent` (`#[should_panic(expected = "try to allocate too much memory")]`; they returned 1 in release before), `integer/tests/pow.rs::test_pow_ubig_shift_overflow`, `::test_pow_ibig_shift_overflow`. The pow tests were run in debug and release.
Fix: `from_str_expanded` gained an "Exponent size" section and a second `# Panics` item (`rational/src/parse.rs`, docs only). `integer/src/pow.rs`: the bit count goes through `pow_shift`, which panics with the usual "try to allocate too much memory" when it does not fit a `usize`.
Notable: what the report calls a panic is machine dependent. On the reporter's machine the allocation fails at once ("out of memory"). On this machine (macOS) the allocation is granted lazily and the parse kept running: I killed `1e999999999999` after more than 5 minutes of CPU time, at 2.5 GB resident. That is why no test runs such a literal, and it is the strongest argument for a maintainer to consider a hard limit plus a `ParseError` variant; see "Unsure" in the final report.

### cbig-imaginary-signed-exponent
Reports: F40
Verdict: real, fixed
Root cause: `parse_algebraic` in the `cbig!` parser split the text at the last `+`/`-` (`macros/src/parse/cmplx.rs:111`, `prefix.rfind(['+', '-'])`), so the sign of an exponent in the imaginary coefficient was taken as the separator (`1+2e` | `-3i`). The runtime `CBig::from_str` tries the splits from the right and takes the first at which both sides are numbers; the macro's module docs claim the same grammar.
Base behaviour: the same (verified: `cbig!(1 + 2e-3i)`, `cbig!(1 + 0x1p-2i)`, `cbig!(0x1p-2i)` and `cbig!(1e-5i)` each fail to compile with "Incorrect syntax", at the base and before the fix).
Provenance: **pre-existing**.
Test: `macros/tests/cmplx.rs::test_cbig_signed_exponent` (the report's three literals, a pure imaginary `1e-5i`, signs on both exponents, `± i` after an exponent, the static form, and `0x1e - 3i` where `e` is a hex digit).
Fix: `parse_algebraic` mirrors the runtime parser: the rightmost split at which both sides parse (`try_parse_coeff`, the fallible form of `parse_coeff`), else the whole text is the imaginary coefficient. `macros/docs/cbig.md` shows exponents in both coefficients.

### zero-literal-precision
Reports: F59
Verdict: real, fixed
Root cause: `FBig::from_parts_const` returned `Self::ZERO` (unlimited precision) for a zero significand before looking at `min_precision` (`float/src/fbig.rs:283-285`), against its own documentation ("if `min_precision` is provided, the higher one … will be used"). `dbig!` / `fbig!` expand a small literal to `from_parts_const(sign, significand, exponent, Some(digits))`, so a zero literal lost the precision of its digits.
Base behaviour: the same (verified by running: `dbig!(0.000).precision() == 0`, `fbig!(0.00).precision() == 0`, `(dbig!(0.000) + 1) / 3 == 0.3`).
Provenance: **pre-existing**. The earlier notes mention it under `float-literal-negative-zero` (E95) as "not changed … pinned by the existing `test_dbig` / `test_fbig`".
Test: `macros/tests/float.rs::test_zero_literal_precision` (the report's cases against `from_str`, const position, the arithmetic example), `float/tests/io.rs::test_from_parts_const_zero_precision` (precisions 1/20/50/100/500, `None` and `Some(0)` stay unlimited).
Fix: `from_parts_const` returns a zero with the precision `min_precision` when one is given (`float/src/fbig.rs`, docs updated); `None` still gives `ZERO`. No macro code changed. `macros/docs/dbig.md` / `fbig.md` gained a zero example.
Notable (behaviour change, needs a look): the two pinned assertions `fbig!(0).precision() == 0` and `dbig!(0).precision() == 0` in `macros/tests/float.rs` (present at the base) were changed to 1. A bare `0` is one digit — `FBig::from_str("0")` has precision 1, as `dbig!(1)` does — so the rule "precision = number of digits" now holds for every literal. The observable difference for a bare `dbig!(0)`: added to a value of *unlimited* precision the result is now rounded to 1 digit, as with `dbig!(1)`, where it used to stay exact (verified by running: `dbig!(0) + 12345` with the second operand at unlimited precision is `10000`, it was `12345`). If the bare zero is meant to stay an exact constant, that has to be special-cased in the macros.
Not fixed: a *negative* zero literal (`dbig!(-0.000)`) still has unlimited precision. It expands to `from_repr_const(Repr::neg_zero())` (E95), and there is no `const` constructor in `dashu-float` that takes both a `Repr` and a precision; this needs new public API. `static_dbig!` / `static_fbig!` are unlimited by design.

### int-macro-repeated-base
Reports: F87
Verdict: real, fixed
Root cause: `parse_integer_with_error` accepted the identifier `base` whenever no base had been read yet (`macros/src/parse/int.rs:78`, `base.is_none() && ident == "base"`), also when the marker had been seen already.
Base behaviour: the same (verified: `ubig!(5 base base 36)`, `ibig!(-5 base base 36)`, `ubig!(z base base 36)` compile and print `5 -5 35`).
Provenance: **pre-existing**.
Test: unit test `macros/src/parse/int.rs::tests::test_single_base_marker_only` (the parser is private to the proc-macro crate; rejected and accepted forms, including `base base 36`, where the first `base` is the digit string), `macros/tests/int.rs::test_base_marker` (accepted forms through the macros), `compile_fail` doctests in `macros/docs/ubig.md` and `ibig.md`.
Fix: the marker is accepted only once (`!base_marked`).

## Per-report table

| Report | Cluster | Verdict | Provenance |
|---|---|---|---|
| F29 | serde-struct-zero-denominator | real, fixed | pre-existing |
| F31 | in-expanded-padding | real, fixed | pre-existing |
| F32 | numhash-relaxed-modulus-factor | real, fixed | pre-existing |
| F35 | uniform01-empty-limit | real, fixed | pre-existing |
| F40 | cbig-imaginary-signed-exponent | real, fixed | pre-existing |
| F59 | zero-literal-precision | real, fixed (negative zero literals not covered) | pre-existing |
| F66 | fraction-zero-pad-prefix | real, fixed | regression (`da6a507`, fix for E111) |
| F83 | expanded-parse-huge-exponent | docs (out of memory is inherent, now documented); related `pow` wrap real, fixed | pre-existing (leftover of E178) |
| F87 | int-macro-repeated-base | real, fixed | pre-existing |
| F91 | uniform01-empty-limit | duplicate of F35's cluster (real, fixed by rejecting the limit, not by the report's expected output) | pre-existing |

## Changelog entries

`rational/CHANGELOG.md`, under `## Unreleased`:

```markdown
### Change
- The constructors of `rand::Uniform01` panic for a denominator limit that leaves no rational to choose from: below 1 for `new` and `new_open_closed`, below 2 for `new_closed` and `new_open`.

### Fix
- Deserializing an `RBig` / `Relaxed` from the struct form (binary serde formats) rejects a zero denominator instead of returning an invalid `RBig` or panicking.
- `in_expanded` honours the width, the fill, the alignment and the `0` flag (`{:>10.2}` of 1/2 is `"      0.50"`), in the fixed, exact (`#`) and scientific formats.
- With both the `#` and the `0` flags, `Binary` / `Octal` / `LowerHex` / `UpperHex` of a fraction put the padding zeros after the radix prefix (`{:#012x}` of 1/2 is `0x000001/0x2`, it was `000000x1/0x2`).
- `NumHash` of a `Relaxed` whose numerator and denominator share the factor `2^127 - 1` agrees with the hash of its value (`Relaxed::from_parts(p, p)` hashes like 1).
- Sampling from `Uniform01::new_open` with the limit 1 no longer loops forever, and `Uniform01::new_closed` with the limit 1 no longer returns 0 on every draw (both limits are rejected by the constructor).
- Docs of `from_str_expanded` / `from_str_decimal`: the cost of a literal grows with its exponent, and an exponent too large for the memory panics.
```

`macros/CHANGELOG.md`, under `## Unreleased`:

```markdown
### Change
- A zero literal of `fbig!` / `dbig!` has the precision of its digits, like every other literal and like `from_str` (`dbig!(0.000)` has precision 4 and `dbig!(0)` precision 1; both used to have unlimited precision).

### Fix
- `cbig!` accepts an exponent with a sign in the imaginary coefficient (`cbig!(1 + 2e-3i)`, `cbig!(0x1p-2i)`), as `CBig::from_str` does.
- `ubig!` / `ibig!` (and the static variants) reject a repeated `base` marker (`ubig!(5 base base 36)`).
```

`float/CHANGELOG.md`, under `## Unreleased`:

```markdown
### Fix
- `FBig::from_parts_const` applies `min_precision` to a zero significand (it returned a zero of unlimited precision).
```

`integer/CHANGELOG.md`, under `## Unreleased`:

```markdown
### Fix
- `UBig::pow` / `IBig::pow` panic for a power whose bit length overflows `usize` instead of returning a wrong value in release builds (`UBig::from(16u8).pow(1 << 62)` was 1).
```

## Cross-group notes

- `float/src/fbig.rs` (`FBig::from_parts_const`, float-convert area) and `float/tests/io.rs`: changed for F59. The earlier cluster `from_parts_const precision` (E128/E222) touched the same function; this change is confined to the `significand == 0` early return and its doc paragraph.
- `integer/src/pow.rs` and `integer/tests/pow.rs` (int area): the `exp * shift` overflow found while reproducing F83. Not in any report of this group; another group may have a report on `pow` — the change is two call sites plus the helper `pow_shift`.
- `macros/tests/float.rs`: two existing assertions changed (see `zero-literal-precision`).
- No report of this group belongs to another group's root cause. F29 is the binary-format sibling of the earlier A11/D41/E76; F66 comes from the E111 fix; F83 is the documented remainder of E178; F59 was noted as a leftover under E95.
- `ParseError` (in `base/`) still has no "value too large" variant; if one is added centrally, `Repr::from_str_expanded` is where an exponent limit would go.
