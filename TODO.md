## dashu-int Improvements

### High impact

- **`submul_1` fused primitive** — Multiply-and-subtract in one pass for the division inner loop correction step. Currently done as separate mul + sub, doubling memory passes. Reference: `ramp/src/ll/mul.rs:134-182`.

- **`div_preinv` / 3-by-2 division with pre-inverted divisor** — Ramp computes `invert_pi(d1, d0)` for fast approximate quotient without the x86 `div` instruction, with separate `divrem_1` (single-limb) and `divrem_2` (two-limb) fast paths. Could replace the `num-modular` dependency. Affects many hot paths: modular arithmetic, formatting, base conversion. Reference: `ramp/src/ll/limb.rs:768-783`, `ramp/src/ll/div.rs:208-253`.

- **Dedicated Toom-2 squaring** — `sqr_toom2` exploits symmetry: uses `z1 = x0*x1` instead of `(x0-x0)*(y1-y0)`, eliminating subtraction operations — only 3 sub-products instead of 4, and the cross term is `2*z1` without signed arithmetic. dashu has Karatsuba and Toom-3 general multiplication but no squaring-specific variant that takes advantage of `x == y`. dashu only uses specialized squaring up to 30 words. Reference: `ramp/src/ll/mul.rs:473-512`.

- **Toom-22 as intermediate multiplication** — Ramp uses Toom-22 above 20 limbs before falling back to unbalanced mul. dashu goes Karatsuba → Toom-3 at 192 words. Toom-22 could fill the 24–192 word gap. Reference: `ramp/src/ll/mul.rs:243-390`.

### Medium impact

- **Trailing zero stripping in GCD loop** — Strip trailing zeros after each subtraction, not just at initialization. Helps for random inputs where intermediate results often gain trailing zeros. Reference: `ramp/src/ll/gcd.rs:20-86`.

- **Trailing zero stripping in pow** — Factor out `(m * 2^k)^exp = m^exp * 2^(k*exp)` to reduce operand size. Reference: `ramp/src/ll/pow.rs:41-118`.

### Low impact / ergonomics

- **Build-time BASES table** — Pre-compute `digits_per_limb` and `big_base` per base via `build.rs` so base-10 conversion avoids repeated division. dashu's `integer/src/fmt/non_power_two.rs` uses simpler chunking (`CHUNK_LEN = 16`) without precomputed powers. Reference: `ramp/src/ll/base.rs:31-40`, `ramp/build.rs`.

- **Scratch allocator improvements** — Ramp's `TmpAllocator` uses a linked list of dynamic allocations freed on drop, vs. dashu's pre-computed layout approach. Might be simpler for algorithms with hard-to-predict memory needs. Reference: `ramp/src/mem.rs`.

## dashu-ratio Improvements

- GCD: An idea of fast gcd check for rational number: don't do gcd reduction after every operation.
  For small numerators or denominators, we can directly do a gcd, otherwise, we first do gcd with a primorial that
  fits in a word (min is u16), and only remove these small divisors.
  Further improvement: store a const divisor for the prime factors in the primorial, thus supports a fast factorial of
  the gcd result, and then divide with these const divisor.

## dashu-float Improvements

- **Trig (`sin`/`cos`) — current baseline** — `float/src/math/trig.rs` uses dynamic guard-digit
  work precision, simple `x mod (π/2)` range reduction, and Taylor series on the reduced
  argument `r`. Adequate for moderate precision and moderate `|x|`; items below target large
  arguments and very high precision. Reference: MPFR `mpfr_sin` / `mpfr_sin_cos`.

### High impact

- **Payne–Hanek range reduction** — For large `|x|`, replace `k = round(x/(π/2)); r = x - k·(π/2)` with
  multiplication by precomputed blocks of `2/π`, extracting the integer part without a full high-precision
  division. Avoids catastrophic cancellation that currently forces `work_precision ≈ precision + log|x| + guards`
  (`compute_work_context`). This is the main gap vs. MPFR for huge arguments. Reference: `float/src/math/trig.rs`.

- **Binary splitting for Taylor core** — At high precision, evaluate the `sin`/`cos` series via binary splitting
  (same technique as Chudnovsky π in `float/src/math/consts.rs`) instead of naive term-by-term accumulation.
  Reduces cost from O(p²) to roughly O(M(p) log p) for p-bit results.

### Medium impact

- **Remez minimax polynomial + Clenshaw (low/medium p)** — For `p ≲ 512`, use a fixed-degree minimax polynomial
  on `[-π/4, π/4]` evaluated with Clenshaw recurrence instead of Taylor. MPFR uses this for its fast path; switch
  to series/binary splitting only when p is large.

- **Cody–Waite π/2 split** — Represent `π/2 = hi + lo` and compute `r = ((x - k·hi) - k·lo)` to reduce guard
  digit pressure for moderate `|x|` before Payne–Hanek is needed. Complements the existing `reduce_to_quadrant`.

- **Argument shrinking for `|r| > π/4`** — Use `sin(r) = cos(π/2 - r)` (and the cosine analogue) so the Taylor
  series runs on a smaller interval, needing fewer terms when `r` is near ±π/2.

### Low impact / ergonomics

- **Cache π at common precisions** — Avoid recomputing Chudnovsky π on every trig call when work precision repeats.
  TODO already noted in `float/src/math/consts.rs`.

- **Precomputed `2/π` block table** — Storage for Payne–Hanek: blocks of `2/π` bits (e.g. 32/64 bits per entry),
  generated once or lazily on first use at a given precision.
