# int-kernels

28 reports, 10 distinct root causes, all real and all fixed. Every cluster was
reproduced at `a0fc535` through its regression test (run against the unfixed
source, then against the fix). All changes are in `integer/`.

The "internal error: not enough memory allocated" reports do **not** share one
root cause: there are three (the modular multiplication budget, the
equal-operand squaring shortcut, and the `gcd_ext` post-processing budget),
plus one shared enabler — the scratch-budget "envelope" in `mul/mod.rs` was not
actually an upper bound over the shapes its callers assumed.

## Clusters

### ntt-carry — wrong products from the transform-based multiplication

Reports: D3, D19, E18

Verdict: real, fixed

Root cause: `add_shifted_to_prod` (`integer/src/mul/ntt/mod.rs`, at `a0fc535`
line 548) accumulates each CRT-recovered convolution coefficient into the
product. It folded the running carry into the next limb with
`word.wrapping_add(carry)` (the carry is lost when the limb is all ones), and
for packing widths that straddle word boundaries (56/48 bits with 64-bit words,
20 bits with 32-bit words) it re-applied the carry out of word `idx + 1` to the
*next limb before shifting*, i.e. at the wrong weight. It also computed the bit
offset in `u32` with `wrapping_mul`. Any product or square whose parameter
selection picks a word-crossing width was affected: balanced operands of
4001–4096 words (and other size buckets), the chunked unbalanced path
(2500 x 15000 words), and on 32-bit words 65537 words. The thresholds and the
transform itself are fine — only the final accumulation was wrong. D3's "exactly
2^320 too small" is one dropped carry.

Test: `integer/tests/mul.rs::test_sqr_two_pow_minus_d_large`,
`integer/tests/mul.rs::test_mul_all_ones_large` (closed-form oracles, sizes
4000–65537 words, balanced and unbalanced); unit test
`mul::ntt::tests::test_add_shifted_to_prod_carries` against a bit-by-bit
reference.

Fix: rewrote `add_shifted_to_prod` as a plain shifted add with an explicit spill
word and a boolean carry; the bit offset is computed in `u64`. The unused
`count` parameter was dropped.

Notes: verified under `--cfg force_bits="32"` too (the tests fail there before
the fix at 4001 words and pass after, including 65537 words — D19's size).
E18's report is correct; so are D3 and D19.

### mul-high-sticky — `mul_high` sticky flag misses dropped cross products

Reports: B5, C4, D4, E12

Verdict: real, fixed

Root cause: `mul_high_into` (`integer/src/high/mul.rs`, at `a0fc535` line 216)
splits `n`-word operands at `l = n/4`, `k = n - l` and only flagged the dropped
low-low block `ap[..l] * bp[..l]`. The products `ap[..l] * bp[l..k]` and
`ap[l..k] * bp[..l]` (columns `l..n-1`, entirely below the window) are never
computed and were never flagged. The returned value was always right; only the
flag was wrong, and only above the 96-word base-case threshold.

Test: `integer/tests/high_mul.rs` (new file) —
`test_mul_high_sticky_sparse_reported`, `test_mul_high_sticky_sparse_sweep`.

Fix: the dropped-product check is now
`(ap[..l] != 0 && bp[..k] != 0) || (ap[..k] != 0 && bp[..l] != 0)`.
`sqr_high` was checked and is already correct.

Notes: D4 also shows the consequence in `dashu-float` (`Context::mul` off by one
ulp at 1510 decimal digits, because the float layer trusts the flag). That is
this root cause; no float-level test was added (the integer-level tests cover
it), see Cross-group notes.

### div-high-overflow — `div_high` with a window whose bit size overflows `usize`

Reports: E218

Verdict: real, fixed

Root cause: `div_high` (`integer/src/high/div.rs`, at `a0fc535` line 219)
computed `n * WORD_BITS` unchecked. Debug builds panic on the overflow; in
release the wrapped value can equal the operands' bit-length difference
(`out_words = 2^58 + 1`), so the sizing check passes and a `3n`-word buffer is
requested ("out of memory").

Test: `integer/tests/high_div.rs::test_div_high_unsized_window`.

Fix: `checked_mul` / `checked_add`, returning `None` as documented.

### modular-mul-memory — Reduced / Montgomery multiplication of short residues

Reports: A4, A7, B8, D9, E23, E107, E250

Verdict: real, fixed

Root cause: `modular::mul::mul_memory_requirement`
(`integer/src/modular/mul.rs`, at `a0fc535` line 81) and
`monty::mul::mul_memory_requirement` (`integer/src/monty/mul.rs`, line 107)
size the scratch for a full `n x n` product, but `mul_normalized` /
`mul_normalized_large` trim the operands' leading zero words first. The
multiplication budget is not monotone in the operand lengths: a `96 x 145`
product lands in the Toom-4x2 band and needs more scratch than `145 x 145`
(Toom-3). `MontgomeryRepr::reduce` hits it too because entering Montgomery form
is a multiplication by `R^2 mod m`.

Test: `integer/tests/monty.rs::large_ring_operands_shorter_than_modulus` (both
rings, moduli of 62–248 words, operand lengths swept across the band; covers
reduce, mul, mul_assign, sqr, self-mul and pow).

Fix: both budgets now use the new `mul::memory_requirement_up_to_operand_len(n)`
(= `memory_chain_budget_envelope(2n, n)`), which covers every product with
operands of at most `n` words. This needed the envelope itself to be made
sound — see "Notable" below.

Notable: `mul::memory_chain_budget_envelope` was documented as dominating every
shape "with total length <= total_cap", but it only evaluated shapes whose
total *equals* the cap (its test did the same), and it was not monotone in
either argument: e.g. `envelope(336, 96)` was 206 words while a `239 x 96`
product is budgeted at 1278. It is now a monotone bound built from the ladder budget of
the smaller side, a recursive bound for Toom-4x2 shapes, and the
transform budget (plus the tail of the chunk loop where the transform path
chunks). Two new unit tests check it against the per-shape model exhaustively
for all caps up to a total of 700 words and by sampling up to 50000 words.
Consequences for the existing callers (division, GCD): their reservations are
now true upper bounds, and somewhat larger for mid-size operands. To keep the
bound tight in the transform range, the per-shape model no longer adds a
chunk-tail budget for transform products that are not chunked (ratio below 3 —
those go through a single convolution, which is covered by the transform budget
alone); the predicate is shared with the dispatcher as `ntt::is_chunked`.

### modular-self-mul-memory — `&a * &a` on a Reduced value

Reports: D13, E50

Verdict: real, fixed

Root cause: `modular::mul::mul_in_place` (`integer/src/modular/mul.rs`, at
`a0fc535` line 139) switches to `sqr_normalized` when both operands are equal,
but its callers allocate with `mul_memory_requirement`, which did not include
the squaring budget. Squaring needs more scratch than multiplication in the
Karatsuba band (a fact already recorded on `sqr::sqr_memory_requirement`, which
`Reduced::sqr` uses — hence `a.sqr()` worked).

Test: `integer/tests/modular.rs::test_mul_equal_operands_large` (also covered
by the "self-mul" assertions of the monty.rs test above).

Fix: `mul_memory_requirement` now takes the maximum with
`sqr::memory_requirement_exact(n)` (which is monotone in the length — checked
by brute force up to 20000 words — so it also covers a trimmed operand).
Montgomery's budget already included the squaring term.

Notes: same commit as modular-mul-memory.

### gcd-ext-memory — `gcd_ext` with a large common factor

Reports: A9, B11, D12 (the panic half), E7

Verdict: real, fixed

Root cause: `gcd_ext_large` (`integer/src/gcd_ops.rs`, at `a0fc535` lines
281–282) recovers the second cofactor as `a = (g - rhs * b) / lhs`. It budgeted
the product as an `lhs_len x rhs_len` multiplication although it is
`rhs_len x b_len`, where the coefficient `b` can be much shorter than `lhs`
(it has about `lhs_len - g_len` words), and it budgeted the exact division with
the wrong divisor length (`rhs_len` instead of `lhs_len`). With a common factor
of about half the operand length the product falls in the Toom-4x2 band.

Test: `integer/tests/gcd.rs::test_gcd_ext_large_cofactor_shapes`.

Fix: the product uses `memory_chain_budget_envelope(lhs_len + rhs_len, rhs_len)`
and the division `div::memory_requirement_exact(lhs_len + rhs_len + 1, lhs_len)`.
In `lehmer::memory_requirement_ext_up_to` the coefficient-update product is now
budgeted for the full `lhs_len + 1`-word coefficient buffer (it was `lhs_len`;
hardening, no failing input found).

### gcd-ext-cofactor — `gcd_ext` returns cofactors violating the Bezout identity

Reports: D12 (the wrong-value half)

Verdict: real, fixed

Root cause: in the Euclidean step of `gcd_ext_in_place`
(`integer/src/gcd/lehmer.rs`, at `a0fc535` lines 404–426) the update
`t0 += q * t1` assumed `t0` is no longer than `q * t1`. When the quotient is a
single top word (`q_lo` empty, `q_top = 1`) and `t0` is one word longer than
`t1`, the addition only covered `t0[..t1_len]` and the new length was then
taken from that range (or the carry word *overwrote* `t0[t1_len]`), truncating
the coefficient. Minimal failing input found: `a = B^35`,
`b = 3*B^18 + B^17 + 1` with `B = 2^64`.

Test: `integer/tests/gcd.rs::test_gcd_ext_large_cofactor_shapes` (includes the
reported input and the minimal one).

Fix: new helper `settle_coeff_carry` propagates the carry through the
coefficient's upper words and keeps its length; it is used for the Euclidean
step and for the final single-word reduction (same pattern).

Notes: validated with a scratch sweep (not committed): all
`a = B^i, b = 3*B^j + B^k + B^l` for `i < 64`, and 6000 random sparse/dense
pairs up to 420 words, with and without a common factor — no failures.

### const-divisor-dword — reducing a double-word value by a normalized single-word modulus

Reports: E104

Verdict: real, fixed

Root cause: `ConstSingleDivisor::rem_dword` (`integer/src/div_const.rs`, at
`a0fc535` line 80) passes the double word straight to `div_rem_2by1` when the
modulus has its top bit set (shift 0), but that division requires the high word
to be below the divisor. Debug builds panic inside `num-modular`
(`a_hi < self.divisor`), release builds return a wrong remainder.

Test: `integer/tests/const_div.rs::test_const_div` (extended with top-bit-set
single-word divisors for 16/32/64-bit words, double-word dividends with a large
high word, and a `reduce(..).residue()` check). Fails in both debug and release
before the fix.

Fix: one conditional subtraction of the divisor from the high word first.

### reducer-check — `Reducer<UBig>` add/dbl return the modulus

Reports: B4, D5, E2

Verdict: real, fixed

Root cause: `<ConstDivisor as Reducer<UBig>>::check`
(`integer/src/modular/reducer.rs`, at `a0fc535` line 108) used `is_le()` for
moduli of three or more words, so the modulus itself counted as reduced and
`reduce_once` (used by `add` and `dbl`) did not subtract it.

Test: `integer/tests/reducer.rs::test_reducer_sum_equal_to_modulus`.

Fix: `is_lt()`.

### pow-modulus-one — `Reduced::pow(&0)` in the ring modulo 1

Reports: E3

Verdict: real, fixed

Root cause: `ReducedWord::one` (`integer/src/modular/repr.rs`, at `a0fc535`
line 111) returned `1 << shift` without reducing it; for the modulus 1 that
equals the normalized modulus, i.e. it is not a valid residue (debug assertion
`raw.is_valid(ring)`; residue 1 in release). The crate supports this ring
(`test_inv` builds `ConstDivisor::new(1)`).

Test: `integer/tests/modular.rs::test_pow` (extended).

Fix: `one` returns 0 when the modulus is 1. The debug-only
`ReducedLarge::is_valid` was also tightened from `<=` to `<` the modulus (the
same slip as reducer-check; no behaviour change in release).

### nth-root-slow — `nth_root` with a large order

Reports: D28, E21

Verdict: real, fixed

Root cause: `newton_nth_root` (`integer/src/root_ops.rs`, at `a0fc535` line
307) started Newton's method from the power-of-two *underestimate*
`2^(bits / n)`. Its first step overshoots to about `num / (n * x^(n-1))`, which
can be astronomically large, and from far above the iteration only shrinks by a
factor `1 - 1/n` per step, each step raising a huge number to the power `n-1`.
`(2^2016).nth_root(1009)` did not terminate in practice. The same happened
after the small-factor peeling whenever the remaining root was 1.

Test: `integer/tests/root.rs::test_nth_root_large_order`,
`integer/tests/root.rs::test_nth_root_tight` (tightness `r^n <= x < (r+1)^n`
over orders 3–2003 and roots from 1 bit to a few thousand bits).

Fix: the root is computed by precision doubling — the root of the leading part
gives a starting point just above the true root, from which Newton decreases
monotonically and converges quadratically — with a bit-by-bit search when the
root has at most `log2(n) + 2` bits, and an early `1` when `bits <= n`.

Notes: measured in release (not a committed benchmark): the reported inputs now
take about 1 µs; cube/5th/11th/13th roots of 300k–3M-bit numbers are 3–5x
faster than before (e.g. cube root of `3^2000000`: 1.9 s -> 0.39 s); small
inputs are unchanged.

## Per-report table

| Report | Cluster | Verdict |
|---|---|---|
| A4 | modular-mul-memory | real, fixed |
| A7 | modular-mul-memory | real, fixed |
| A9 | gcd-ext-memory | real, fixed |
| B4 | reducer-check | real, fixed |
| B5 | mul-high-sticky | real, fixed |
| B8 | modular-mul-memory | real, fixed |
| B11 | gcd-ext-memory | real, fixed |
| C4 | mul-high-sticky | real, fixed |
| D3 | ntt-carry | real, fixed |
| D4 | mul-high-sticky | real, fixed |
| D5 | reducer-check | real, fixed |
| D9 | modular-mul-memory | real, fixed |
| D12 | gcd-ext-cofactor (wrong cofactors) + gcd-ext-memory (panic) | real, fixed |
| D13 | modular-self-mul-memory | real, fixed |
| D19 | ntt-carry | real, fixed |
| D28 | nth-root-slow | real, fixed |
| E2 | reducer-check | real, fixed |
| E3 | pow-modulus-one | real, fixed |
| E7 | gcd-ext-memory | real, fixed |
| E12 | mul-high-sticky | real, fixed |
| E18 | ntt-carry | real, fixed |
| E21 | nth-root-slow | real, fixed |
| E23 | modular-mul-memory | real, fixed |
| E50 | modular-self-mul-memory | real, fixed |
| E104 | const-divisor-dword | real, fixed |
| E107 | modular-mul-memory | real, fixed |
| E218 | div-high-overflow | real, fixed |
| E250 | modular-mul-memory | real, fixed |

## Changelog entries

`integer/CHANGELOG.md`, under `## Unreleased`:

```markdown
### Fix
- Fix wrong results of large multiplications and squarings that go through the transform-based (NTT) kernel with a coefficient width that straddles word boundaries (e.g. operands of 4001–4096 64-bit words, unbalanced products such as 2500 x 15000 words, and 65537 words with 32-bit words): carries were lost while accumulating the coefficients into the product.
- Fix `high::mul_high` returning `sticky = false` for windows above 96 words when a product of one operand's low quarter and the other's middle part is nonzero below the window.
- Fix `high::div_high` panicking (or attempting a huge allocation) instead of returning `None` when `out_words * Word::BITS` overflows `usize`.
- Fix "not enough memory allocated" panics in `Reduced` and `Montgomery` multiplication, `pow` and `MontgomeryRepr::reduce` when a residue has fewer words than a large modulus, and in `Reduced` multiplication of a value by itself.
- Fix "not enough memory allocated" panics in `gcd_ext` for large operands with a large common factor.
- Fix `gcd_ext` returning cofactors that do not satisfy the Bezout identity for some large operands (a word of a coefficient was dropped).
- Fix `ConstDivisor::reduce` and `UBig % &ConstDivisor` panicking (debug) or returning a wrong remainder (release) for a double-word dividend and a single-word modulus with its top bit set.
- Fix `<ConstDivisor as num_modular::Reducer<UBig>>::check` accepting the modulus itself for moduli of three or more words, which made `add` and `dbl` return the modulus instead of zero.
- Fix `Reduced::pow` with a zero exponent returning 1 (or failing a debug assertion) in the ring of integers modulo 1.

### Change
- `nth_root` now uses Newton's method with precision doubling: roots of a large (prime) order, which previously could take minutes or not terminate in practice, are immediate, and large cube and other small-order roots are several times faster.
```

## Cross-group notes

- All source changes are in `integer/`; no other crate was touched.
- `integer/src/div_const.rs` (const-divisor-dword) and
  `integer/src/modular/` are plausibly also in the area of whichever group owns
  ConstDivisor / modular reports; the changes there are small and local
  (`rem_dword`, `Reducer::check`, `ReducedWord::one`, `ReducedLarge::is_valid`,
  `mul_memory_requirement`).
- `integer/src/mul/mod.rs`: `memory_chain_budget_envelope` was rewritten, and
  `memory_chain_budget_closed_form` removed. Division
  (`div::memory_requirement_exact`) and GCD reservations go through it, so any
  other group's "not enough memory allocated" report in division, GCD, modular
  inverse or float code that divides is probably fixed by commit `e1ed3c2`
  and should be re-tested on top of it.
- D4's second half (a `DBig` product one ulp off at precision 1510 under
  `HalfAway`) is a `dashu-float` symptom of mul-high-sticky. Any float-group
  report of an off-by-one-ulp product at precisions needing more than 96 words
  is likely a duplicate of it.
- Test files added or extended: `integer/tests/{mul,high_mul (new),high_div,
  monty,modular,gcd,const_div,reducer,root}.rs`. If other groups also append
  to these files, the merge may need trivial conflict resolution at their ends.

## Verification

Run on the final tree (aarch64 macOS, stable toolchain):

- `cargo test -p dashu-int` — debug and `--release`: pass.
- `RUSTFLAGS='--cfg force_bits="32"' cargo test -p dashu-int`: pass.
- `RUSTFLAGS='--cfg force_bits="16"' cargo test -p dashu-int`: pass.
- `cargo test --workspace --exclude dashu-python`: pass.
- `cargo fmt --all -- --check`: clean.
- `cargo clippy --all-features --all-targets --workspace --exclude dashu-python -- -D warnings`: clean.
- `cargo check -p dashu-int --no-default-features`: ok.
- `fuzz/tests/integer.rs` (GMP differentials, `--ignored`): 16 passed.

Not verified by running: MSRV 1.68 (no such toolchain installed; the changes
use only long-stable language features), and the house Dylint lints.
