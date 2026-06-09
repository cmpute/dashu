# Prime-NTT multiplication for `UBig` — implementation plan

Goal: add an NTT-based large-integer multiplication path to `dashu-int` that
kicks in above the current Toom-Cook-3 range, using power-of-two Number
Theoretic Transforms over several 64-bit NTT-friendly primes combined with the
Chinese Remainder Theorem (CRT).

This document is the working spec + checklist. Keep it updated as the work
progresses. Each phase ends with a state where `cargo test -p dashu-int`
passes.

---

## 0. Background and decisions

- **Word layout.** `Word = u64`, `DoubleWord = u128` on the default build
  (`integer/src/arch/generic_64_bit/word.rs`). The 16-/32-bit `force_bits`
  builds use smaller `Word`, but `u128` is available on every Rust target, so
  the NTT lane arithmetic is always done in `u64`/`u128` regardless of `Word`.
- **Where it plugs in.** Multiplication is dispatched in
  `integer/src/mul/mod.rs` by the *smaller* operand length in words:
  - `len <= THRESHOLD_SIMPLE (24)` → `simple`
  - `<= THRESHOLD_KARATSUBA (192)` → `karatsuba`
  - else → `toom_3` (currently unbounded above)

  We add `THRESHOLD_NTT` (in words) above which `toom_3` hands off to the new
  `ntt` module. The exact value is tuned by benchmark (Phase 7); start with a
  conservative placeholder (e.g. `2048` words ≈ 130k bits).
- **Scheme.** Linear (acyclic) convolution of the two coefficient polynomials,
  realised as a cyclic convolution of length `N = next_pow2(La + Lb - 1)` with
  zero padding. Computed independently modulo `K` primes, then CRT-combined per
  coefficient, then carry-propagated into the output limbs.
- **Why ≥ 3 primes + CRT (recap).** A single ~2^64 prime forces tiny coefficient
  chunks. With `K` primes of product `P ≈ 2^(64K)` we can pack `b` bits per
  coefficient as long as the largest convolution coefficient
  `< N/2 · (2^b − 1)^2 < P`. Three primes (`P ≈ 2^189..192`) give comfortable
  headroom for any feasible input and let us pick a larger `b` (fewer
  coefficients → smaller transform). `K = 2` is provably sufficient for all
  inputs `UBig` can physically hold, but `K = 3` is the default for margin and
  speed.
  - **`K` vs `K_eff` (avoid const-generic monomorphization).** `K = 3` is the
    *fixed array size* of `PRIMES` (a plain `const`, not a const-generic type
    parameter), so the transform code is monomorphized once. `select_params`
    returns a runtime `K_eff ≤ K`; the per-prime loop iterates
    `PRIMES[..K_eff]`. The Phase-7 "drop to 2 primes" optimisation is just
    `K_eff = 2` at runtime — no extra monomorphization, no new generic
    instantiations.
- **Modulus family: Goldilocks-style** `p = 2^64 − 2^b + 1`. Every member
  satisfies the shift-based reduction identity `2^64 ≡ 2^b − 1 (mod p)`, so the
  *entire* lane arithmetic is multiplication-free in the reduction step — no
  Montgomery form anywhere. We use three members of this single family (below)
  so `modarith` is **one** routine parameterised by `b`.

### Chosen primes (verified)

Survey of documented choices considered and rejected:
- **"Ultimate NTT" prime** `9223372036737335297 = 549755813881·2^24 + 1`,
  `g = 3` (Codeforces entry 75326): `v2` only 24 caps `N` at `2^24`, and it is
  not a Solinas form (needs Barrett/Montgomery). Rejected.
- **Classic CRT trio** `998244353`, `985661441`, `754974721`, … : ~30-bit, so
  they waste 64-bit lanes and cap `N` low. Rejected.
- **`c·2^32+1` siblings** (e.g. `0xFFFFFFD300000001`): right size and `v2`, but
  not Solinas form → would force a Montgomery path. Rejected in favour of the
  uniform fast-reduction family below.

**Selected trio — all of the form `2^64 − 2^b + 1`** (verified prime by
deterministic Miller–Rabin, full-order `2^32`-th root checked):

| name | `b` | `p` (hex) | `p` (dec) | `v2(p−1)` | gen `g` | `2^32`-th root ω |
|---|---|---|---|---|---|---|
| GL | 32 | `0xFFFFFFFF00000001` | `18446744069414584321` | 32 | 7  | `1753635133440165772` |
| P1 | 34 | `0xFFFFFFFC00000001` | `18446744056529682433` | 34 | 5  | `11315553352654630047` |
| P2 | 40 | `0xFFFFFF0000000001`  | `18446742974197923841` | 40 | 19 | `551857376737322389` |

- `min(v2) = 32` ⇒ transform length up to `2^32` coefficients (≈ ~1 GB operands
  at `b_pack = 16`); `P = GL·P1·P2 ≈ 2^192` of CRT headroom.
- All three reduce via `2^64 ≡ 2^bᵢ − 1`. GL's `b = 32` is the cleanest (splits
  a 128-bit product into 32-bit limbs, `φ²=φ−1`); `b = 34, 40` need one extra
  shift/fold because the split crosses the 32-bit boundary, but stay
  multiply-free. Implement the reduction generically over `b` with the GL case
  as the well-trodden reference.
- `ω⁻¹` and `N⁻¹` are derived per call (cold, once per prime — not lazy):
  `ω_N = pow(ω, 2^32 / N)` (the stored `ω` has exact order `2^32`); the inverse
  root is just `ω_N⁻¹ = pow(ω_N, N − 1)` (since `ω_N^N = 1`, no `inv` needed),
  and `N⁻¹ = Reducer::inv(N mod p)` once. Use `num_modular::Reducer::{pow, inv}`
  for all three. Commit a `verify_primes()` test
  (Miller–Rabin + `v2` + exact root order + reduction-identity self-check)
  rather than trusting these literals blindly.

### Open decisions to lock during Phase 1
- [ ] Final `K` (start 3).
- [ ] Coefficient bit width `b_pack` (**default 16**: 4 coeffs/word, trivial
      shift/mask packing). Larger `b_pack` = fewer coefficients (smaller
      transform) but needs more headroom and must satisfy
      `(N/2)·(2^{b_pack}−1)^2 < P` for the max supported `N`. Candidate values:
      - `16` — divides 64, byte-aligned, 4 coeffs/word. **Default unless a
        benchmark proves a larger value wins.**
      - `24` — divides 64? no, but byte-aligned (3 bytes), 8 coeffs/3 words.
      - `21` — **avoid**: does not divide 64 and is not byte-aligned, so
        coefficients straddle word *and* byte boundaries → slower, buggier
        pack/unpack. Only revisit if its transform-size win clearly beats the
        pack cost in benchmarks.
- [ ] Whether to gate the NTT path on `cfg(target_pointer_width)` / `Word`
      width, or always enable it (preferred: always enable, since lane math is
      `u64`/`u128`). Document the chosen rule.

### Word-width targets (future work, not near-term)

The `2^64 − 2^b + 1` primes are correct on every `Word` width (`u64`/`u128` are
universal types), but on narrow targets the `u64×u64→u128` lane multiply is
emulated and slow. Plan:

- **64-bit `Word`**: primary target, the chosen 3 primes above.
- **32-bit `Word`**: **select a separate set of three ~32-bit Solinas primes
  (`2^32 − 2^b + 1`, via `FixedTrinomialSolinas32`)** so the lane multiply is a
  native `u32×u32→u64`. Feasible: 3 such primes give `P ≈ 2^96`, which (with
  `b_pack = 16`, max coefficient `≈ N·2^32`) is far more headroom than needed —
  the transform-length ceiling comes from each prime's `v2(p−1)`, which is ample
  for any input a 32-bit target would handle. Requires extending
  `FixedTrinomialSolinas32` to `P1 = 32` (same `checked_shl` fix already done for
  the 64-bit type) plus a prime/root search. **Not intended for implementation
  in the near future** — design the NTT core generic over the prime set so this
  can be added later as configuration, not a rewrite.
- **16-bit `Word`**: do not implement an NTT path; fall back to Toom-3.

---

## 1. Module layout

New directory `integer/src/mul/ntt/` (declare `mod ntt;` in
`integer/src/mul/mod.rs`):

| File | Responsibility |
|---|---|
| `ntt/mod.rs` | Public entry `add_signed_mul` / `add_signed_mul_same_len`, `memory_requirement_up_to`, `THRESHOLD_NTT`, parameter selection (`b`, `N`, `K`). |
| `ntt/primes.rs` | Const table of the `K` primes: value, `b`, primitive root, `v2(p−1)`, precomputed `2^32`-th root. Includes a `verify_primes` unit test. |
| `ntt/modarith.rs` | Lane arithmetic for the prime `2^64 − 2^b + 1`. **Reuse `num_modular::FixedTrinomialSolinas64<64, b, 1>::{reduce_single, reduce_double}` for the reduction step** (already P1=64-correct and unrolled to the verified fold counts; both are `pub` as of the `checked_shl` fix — verify on the pinned version, see fallback below). Write our **own** lazy `add` / `sub` / `mul` on top so we control deferred reduction; reuse num-modular's `Reducer::{pow, inv}` (fully normalized, called once per prime per call — no lazy variant needed). See note below. |
| `ntt/transform.rs` | Iterative in-place radix-2 forward/inverse NTT, twiddle precomputation, bit-reversal, pointwise multiply. |
| `ntt/pack.rs` | Bit-slice an operand `&[Word]` into `N` coefficients of `b` bits (mod each prime), and the inverse: CRT-combine residues + carry-propagate into the output limbs. |
| `ntt/crt.rs` | Garner CRT for `K` residues → a small (≤ `K`-word) integer per coefficient. |

Mirror the existing modules' conventions: `#[must_use]` on the `add_signed_mul*`
functions, return `SignedWord` carry, doc comments with complexity, `Buffer` /
`Memory` for scratch (no `Vec<Word>`), no `std`.

---

## 2. Math reference (for reviewers and tests)

Operands `A = sum_i a_i 2^{ib}`, `B = sum_j b_j 2^{jb}` with `0 <= a_i, b_j < 2^b`.
Product `C = A·B = sum_k c_k 2^{kb}` where `c_k = sum_{i+j=k} a_i b_j` is exactly
the linear convolution coefficient, `0 <= c_k < (k+1)·(2^b−1)^2 <= N·(2^b−1)^2`.

Compute `c_k mod p_t` for each prime `p_t` via length-`N` cyclic convolution
(forward NTT, pointwise product, inverse NTT). Because `N >= La + Lb − 1`, the
cyclic and linear convolutions coincide. CRT recovers exact `c_k < P`. Finally
`C = sum_k c_k 2^{kb}` with carry propagation (coefficients overlap whenever
`bitlen(c_k) > b`).

Roots: `omega_N = g^{(p−1)/N} mod p` is a primitive `N`-th root of unity; require
`N | 2^{v2(p−1)}`. Inverse transform uses `omega_N^{-1}` and a final scale by
`N^{-1} mod p`.

---

## 3. Phase plan (each phase is independently testable)

> **Scheduling note.** Phases 2 (transform arithmetic) and 3 (pack / unpack /
> CRT) have no dependency on each other — both only need Phase 1's `modarith`
> and `primes`. They can be built and tested in parallel, then joined in
> Phase 4. Phase 1 must land first; Phases 5–7 follow Phase 4.

### Phase 1 — Primes, modular arithmetic, parameter selection
- [ ] `ntt/primes.rs`: define `const PRIMES: [NttPrime; K]` from the "Chosen
      primes" table. Each entry stores `p`, the form exponent `b`, primitive
      root `g`, `v2(p−1)`, and the precomputed `2^32`-th root `ω`. (No Montgomery
      constants — the family needs none.)
- [ ] Add `#[test] fn verify_primes()` re-checking each entry: primality
      (Miller–Rabin over fixed bases), `p == 2^64 − 2^b + 1 < 2^64`,
      `v2(p−1) >= MAX_LOG_N (=32)`, stored `g` generates the order-`2^{v2}`
      subgroup, `ω` has exact order `2^32`, and the reduction identity
      `2^64 ≡ 2^b − 1 (mod p)` holds. Do not trust the literals without it.
- [ ] `ntt/modarith.rs`: **delegate the reduction** to
      `num_modular::FixedTrinomialSolinas64<64, b, 1>` — its `reduce_single`
      (≤ `2^64` → `[0, p)`) and `reduce_double` (`u128` product → `[0, p)`) are
      already correct for `P1 = 64` and straight-line unrolled (3 folds for
      `b = 32`, 4 for `b = 34, 40`). Do **not** re-derive the shift/fold here.
      Both methods are generated `pub` by the `impl_fixed_trinomial_solinas!`
      macro, so they are directly callable from `dashu-int`.
      - **Fallback if upstream visibility ever regresses:** the reduction is
        ~20 lines per arm; copy it verbatim into `modarith.rs` (it is simple
        enough to own in-tree, and the only coupling point). Pin/assert the
        `num-modular` version in `integer/Cargo.toml` so a downgrade can't
        silently break the `pub` assumption.
      - We write our **own lazy `add` / `sub` / `mul`** (not num-modular's)
        because its `Reducer` API fully normalizes to `[0, p)` after every op
        and exposes no partially-reduced form. Ours keep values lazily in
        `[0, 2p)` (or `[0, 4p)`) and only call `reduce_*` / a final conditional
        subtract when needed — this is the Harvey-style lazy reduction that the
        NTT butterflies depend on. `mul` = `u128` widening multiply →
        `reduce_double`; `add` / `sub` = wrapping add/sub with deferred
        normalization.
      - **`pow` / `inv` are NOT lazy and are NOT ours.** They are called once
        per prime per multiplication (`ω_N = g^{(p−1)/N}`, `N^{-1}`), never in
        the butterfly hot loop, so use `num_modular::Reducer::{pow, inv}`
        directly (fully normalized). No partial-reduction benefit there.
      - Rationale: reduction is the subtle, already-tested part (reuse it); the
        lazy add/sub/mul wrapper is trivial and must be ours to control the
        normalization schedule; pow/inv are cold and reused as-is.
- [ ] `ntt/mod.rs`: `select_params(la_bits, lb_bits) -> (b_pack, N, K_eff)` with
      the headroom assertion `(N as u128 / 2) * (2^{b_pack} − 1)^2 < P` (may drop
      to fewer primes for smaller inputs later).
- [ ] Unit tests: our lazy `add`/`sub`/`mul` (after a final normalize) agree
      with `FixedTrinomialSolinas64`'s fully-reduced `add`/`sub`/`mul` and a
      `u128`/`u256` reference, across the `[0, 2p)` input range for each `b`;
      `Reducer::{pow, inv}` round-trip (`inv(x)·x ≡ 1`, `pow(g, p−1) ≡ 1`);
      `verify_primes`.

### Phase 2 — Forward/inverse NTT
- [ ] `ntt/transform.rs`: iterative Cooley–Tukey radix-2 forward NTT in place,
      decimation-in-time with bit-reversal permutation; inverse NTT
      (conjugate twiddles + scale by `N^{-1}`).
- [ ] Twiddle factors: precompute the `omega_N^k` table per prime into scratch
      once per call (length `N/2`).
- [ ] `pointwise_mul(a_hat, b_hat)` via `modarith::mul` (lazy; normalize at the
      end of the inverse transform).
- [ ] Tests: `inverse(forward(x)) == x`; NTT-based cyclic convolution of small
      random vectors equals the schoolbook cyclic convolution mod `p`; check the
      length-2 and length-power-of-two edge cases.

### Phase 3 — Packing / unpacking + CRT
- [ ] `ntt/pack.rs::pack`: read `b`-bit coefficients out of `&[Word]`
      (bit-level slicing across word boundaries; works for any `Word` width),
      reduce mod each prime (a `b_pack`-bit value is already `< p`, so this is a
      copy), write into the length-`N` (zero-padded) lane buffers.
- [ ] `ntt/crt.rs`: Garner combine `K` residues of one coefficient → an integer
      of ≤ `K` words (value `< P`).
- [ ] `ntt/pack.rs::unpack_accumulate`: for each `k`, add `c_k << (k·b)` bits
      into the output limbs with carry propagation. Implement as a streaming
      shifted add (reuse `add::add_*` helpers / `shift`).
- [ ] Tests: `unpack_accumulate(pack(x)) == x` identity for the
      no-multiplication case (coefficients copied straight through CRT), and a
      direct check that pack→CRT→unpack reconstructs a known convolution.

### Phase 4 — Wire the full multiply
- [ ] `ntt/mod.rs::add_signed_mul_same_len` and `add_signed_mul`: orchestrate
      select_params → per-prime (pack, forward, pointwise, inverse) → CRT per
      coefficient → unpack/accumulate into a temp product buffer → fold into `c`
      via `add::add_signed_*` honoring `sign`. Return the carry as the other
      algorithms do.
- [ ] **Unequal-length entry point — do NOT blindly copy `toom_3`'s chunking.**
      Unlike Toom-3/Karatsuba (defined on equal-length operands, hence
      `helpers::add_signed_mul_split_into_chunks` slices the long operand into
      balanced pieces), a single NTT convolution handles unequal lengths
      natively: pad both operands to one `N = next_pow2(La + Lb − 1)`, one
      forward transform each, pointwise product, one inverse. So the *default*
      unequal path is a single transform — **no chunking**.
      - Dispatch keys on the smaller operand `b.len()`, so when the NTT path is
        entered `b` is already huge. Chunking `a` into `b.len()`-sized pieces
        via the stock helper would run `⌈La/Lb⌉` separate NTTs **and
        re-transform `b` on every chunk**, roughly doubling work for lopsided
        large×large products — it throws away NTT's single-big-transform win.
      - `ntt::add_signed_mul` (unequal) and `ntt::add_signed_mul_same_len`
        (equal) therefore share one core that takes `(La, Lb)` and transforms
        over `N = next_pow2(La + Lb − 1)` directly. Honor the same contract as
        the other algorithms: `c.len() == La + Lb`, accumulate `sign * a * b`
        into `c`, return the `SignedWord` carry.
      - **Only** fall back to chunking for *extreme* imbalance (`La ≫ Lb`, e.g.
        `La > c · Lb` for some tuned `c`), where many balanced `~2·Lb` NTTs beat
        one padded `~La` transform. If/when we do, forward-transform `b` **once**
        and reuse the cached `b_hat` across chunks — i.e. a purpose-built loop,
        not the stock `add_signed_mul_split_into_chunks` (which re-transforms
        `b`). Treat this as a Phase-7 tuning option, not the initial wiring.
- [ ] `ntt/mod.rs::memory_requirement_up_to(n)`: deterministic upper bound on
      scratch, mirroring the style of `toom_3::memory_requirement_up_to`
      (returns a `Layout`). It **must** be an exact upper bound — `Memory`
      `expect`s on underflow.
      - **Draft closed-form bound (words):**
        `2·N` (one `a`-lane + one `b`-lane buffer, processed one prime at a
        time so they are reused across the `K_eff` primes — not `K·N`)
        `+ N/2` (twiddle table for the current prime)
        `+ K` (per-coefficient CRT temp)
        `+ (La + Lb)` (product accumulation buffer)
        `≈ 2.5·N + La + Lb + K`.
        If lanes for all primes are kept live simultaneously (simpler, no
        re-pack per prime) the lane term becomes `2·K·N`; decide which during
        implementation and bound accordingly.
      - **Worst-case over `b_pack`.** `N = next_pow2(ceil((La+Lb)·WORD_BITS /
        b_pack) + 1)`. `memory_requirement_up_to(n)` is called before
        `select_params` runs, so it must bound `N` over **every** `b_pack` the
        selector may choose for inputs up to `n` words — i.e. use the
        *smallest* admissible `b_pack` (largest `N`). Pin a `B_PACK_MIN`
        constant (= 16) and compute the bound from it; `select_params` may then
        only ever pick `b_pack ≥ B_PACK_MIN`.
- [ ] **Carving scratch from the linear `Memory` arena.** `Memory`
      (`integer/src/memory.rs`) is a *linear bump allocator*, not a pool of
      independent `Buffer`s: each `allocate_slice`/`allocate_slice_fill` hands
      out the next region and returns the remainder. Order matters. Plan the
      carve explicitly: allocate longer-lived regions first (twiddle table,
      product buffer) then the per-prime lane buffers from the remaining region
      inside the prime loop (so they are reused each iteration). `Buffer` is
      only for *owned* growable word arrays (e.g. a returned product); transform
      scratch lives in the `Memory` arena. Document the carve order next to
      `memory_requirement_up_to` so the two stay in sync.

### Phase 5 — Dispatch + thresholds
- [ ] In `integer/src/mul/mod.rs`: add `THRESHOLD_NTT`, declare `mod ntt;`.
- [ ] Extend `add_signed_mul`, `add_signed_mul_same_len`, and both
      `memory_requirement_*` to route `len > THRESHOLD_NTT` to `ntt`.
- [ ] Keep `toom_3` as the fallback if NTT parameter selection fails any
      precondition (defensive; should not happen below `2^32` coefficients).

### Phase 6 — Correctness validation
- [ ] Extend `integer/tests/mul.rs` with cases straddling `THRESHOLD_NTT`
      (lengths `T−1`, `T`, `T+1`, `2T`, asymmetric `a`/`b` lengths, operands
      with high/low zero limbs, near power-of-two `N`).
- [ ] Differential test: random `UBig`s of increasing size, assert
      `ntt_product == reference_product` where reference is the existing
      `multiply` forced through Toom-3 (or compute via a smaller-threshold
      build). Cover the coefficient-overflow boundary explicitly (all-ones
      operands at the max supported `N`).
- [ ] Run on a 32-bit lane build too: `cargo test -p dashu-int` with
      `RUSTFLAGS="--cfg force_bits=\"32\""` (and `16`) to confirm packing is
      `Word`-width agnostic.

### Phase 7 — Tuning + optimisation (after correctness is green)
- [ ] Benchmark `THRESHOLD_NTT` crossover against Toom-3 using
      `integer/benches/primitive.rs` (extend the mul benchmark to larger sizes);
      pick the value where NTT wins.
- [ ] Optimisations to layer in, measuring each:
  - [ ] Harvey lazy-reduction butterflies (defer mod in inner loops).
  - [ ] Specialise the `b = 32` (Goldilocks) lane's reduction to 32-bit-limb
        form (`φ²=φ−1`), since it avoids the extra cross-boundary fold that
        `b = 34, 40` need.
  - [ ] Use the shift-expressible roots of unity where applicable (powers of two
        are roots in this family) to replace some twiddle multiplies.
  - [ ] Radix-4 / split-radix transform.
  - [ ] Drop to `K_eff = 2` primes automatically when headroom allows (smaller
        inputs) to halve the transform work.
  - [ ] **Squaring specialization.** `UBig::square()` / the `a == b` case needs
        only one forward transform per prime (not two), then a pointwise
        *square* and one inverse — roughly 2/3 the transform cost. Wire an
        `ntt::square` path (and route `UBig::square` to it above
        `THRESHOLD_NTT`) once the general multiply is correct.
  - [ ] Reuse/cancel allocations; ensure scratch stays within the `Memory`
        arena.

---

## 4. Constraints & pitfalls (project-specific)

- **`no_std`**: only `core` + `alloc`. `u128` arithmetic is fine. The lane
  *reduction* is reused from `num_modular::FixedTrinomialSolinas64<64, b, 1>`
  (`reduce_single` / `reduce_double`; already a dependency of `dashu-int`, and
  `no_std`). The lazy `add`/`sub`/`mul` wrapper around it is ours, in-tree, so
  no Montgomery and no extra dependency. Requires the `num-modular` version that
  (a) supports `P1 = 64` (the `checked_shl` fix + `S64_4` Goldilocks tests) and
  (b) exposes `reduce_single` / `reduce_double` as `pub` (it does as of that
  fix). Pin this version in `integer/Cargo.toml`; if the `pub` assumption ever
  regresses, fall back to the in-tree reduction copy (see Phase 1).
- **MSRV**: keep within the README MSRV (do not bump). Avoid `const` features
  newer than MSRV; plain `const` tables are fine.
- **Scratch**: use `Buffer` / `MemoryAllocation` / the threaded `Memory` arena,
  never `Vec<Word>` (per `AGENTS.md`). `memory_requirement_*` must be an exact
  upper bound or the arena `expect` will panic.
- **Sign / accumulate contract**: the entry points are `c += sign * a * b`
  returning a `SignedWord` carry — match `toom_3`/`karatsuba` exactly so the
  recursive callers and `multiply()` keep working.
- **Changelog**: add an `### Add` entry under `## Unreleased` in
  `integer/CHANGELOG.md` ("NTT-based multiplication for very large integers")
  as part of the same commit (per `AGENTS.md`).
- **CI parity**: before declaring done, run
  - `cargo test --workspace --exclude dashu-python`
  - `cargo clippy --all-features --all-targets --workspace --exclude dashu-python -- -D warnings`
  - `cargo fmt --all -- --check`

---

## 5. Definition of done

- [ ] All phases checked off; NTT path active above `THRESHOLD_NTT`.
- [ ] Differential tests pass on 64-, 32-, and 16-bit lane builds.
- [ ] Benchmarks show NTT beats Toom-3 at and above the chosen threshold.
- [ ] Clippy clean, fmt clean, changelog updated.
- [ ] No `std` usage, no `Vec<Word>` scratch, MSRV preserved.
