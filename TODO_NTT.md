# NTT multiplication for `UBig` — status & remaining work

## Implemented

### Core algorithm (Phases 1–6)

- **Primes.** Three Goldilocks-style Solinas primes `p = 2^64 − 2^b + 1` with
  `b ∈ {32, 34, 40}`.  All support shift-based reduction via `2^64 ≡ 2^b − 1`.
  Stored in `integer/src/mul/ntt/primes.rs` with a full `verify_primes()` test
  (Miller–Rabin, exact root order, reduction-identity self-check).

- **Modular arithmetic.** Lane arithmetic delegates to
  `num_modular::FixedTrinomialSolinas64` for `add`/`sub`/`mul`/`reduce_double`.
  `add`/`sub`/`mul` are monomorphized per `B` via const generics so the
  compiler fully inlines each prime's hot path.

- **NTT transforms.** Iterative in-place radix-2 decimation-in-time
  (`integer/src/mul/ntt/transform.rs`).  Forward transform: `bit_reverse →
  forward(ω)`.  Inverse transform: `bit_reverse → forward(ω⁻¹) → scale by
  N⁻¹`.  Twiddle tables precomputed once per prime per call.  Pointwise
  multiply in the transform domain.

- **Packing / unpacking.** Bit-level `pack` slices `&[Word]` into `b_pack`-bit
  coefficients (`integer/src/mul/ntt/pack.rs`).  CRT-recovered coefficients are
  accumulated into the output limb array via shifted addition
  (`add_shifted_to_prod`).

- **CRT.** Garner's algorithm combining `K` residues modulo `K` primes into a
  `U192` (3 × u64) integer (`integer/src/mul/ntt/crt.rs`).  All Garner
  precomputed constants are hardcoded in `primes.rs` (`CRT_INV_IJ`).  Uses an
  object-safe `ModOps` trait (subset of `Reducer<u64>`) for dynamic dispatch
  over the per-prime reducers.

- **Dispatch.**  Multiplication above `THRESHOLD_NTT` words routes to the NTT
  path (`integer/src/mul/mod.rs`).  `add_signed_mul` (unequal lengths) and
  `add_signed_mul_same_len` (equal lengths) share a single `add_signed_mul_impl`
  that does one NTT convolution — no chunking for equal/similar lengths.

- **Memory.** Scratch space carved from the linear `Memory` arena.  Worst-case
  bound computed in `memory_requirement_up_to` using `B_PACK_MIN = 16`
  (largest possible N for a given operand size).

### Phase 7 optimisations (completed)

- **K_eff = 2 auto-selection.**  `select_params` checks headroom against
  `P0·P1` (≈2^128).  For `b_pack = 16`, `max_coeff < 2^63 ≪ 2^128`, so two
  primes always suffice.  Third-prime fallback (`K_eff = 3`) is retained as a
  safety net for larger `b_pack`.

- **Threshold calibrated.**  `THRESHOLD_NTT = 40 000` words (~2.6 M bits),
  the first measured crossover where NTT beats pure toom-3 on Apple M4 Pro.

- **Env-var overrides.**  `DASHU_THRESHOLD_SIMPLE`, `DASHU_THRESHOLD_KARATSUBA`,
  `DASHU_THRESHOLD_NTT` override the compile-time defaults at runtime.  Gated
  behind the `tuning` feature (implies `std`).

- **Crossover benchmark.**  `#[ignore]` test `crossover()` in
  `integer/src/mul/ntt/mod.rs` compares NTT against toom-3 at key sizes.
  Run with `DASHU_THRESHOLD_NTT=99999999` to force pure toom-3.

---

## Remaining optimisation opportunities

### 1. Radix-4 or split-radix NTT  (~25–33% fewer twiddle multiplies)

Radix-4 processes 4 elements per butterfly with 3 twiddle multiplies and
`log₄(N)` stages (half as many passes through memory).  Split-radix pushes
the savings closer to 33%.

**Work items:**
- Rewrite `ntt_core` in `transform.rs` with a radix-4 butterfly.
- Handle N that is a power of 2 but not a power of 4: do one radix-2 stage
  followed by radix-4 stages.
- Update twiddle indexing; the twiddle table layout changes.
- A primitive 4-th root `j = ω_N^{N/4}` is needed for the butterfly core;
  derive it from the existing `ω_2_32` root.

### 2. Harvey lazy-reduction butterflies  (~10–15%)

Currently every `add_mod` / `sub_mod` fully normalizes to `[0, p)`.  Harvey's
approach keeps values in `[0, 2p)` across multiple butterfly stages, deferring
the conditional subtract to the end (or to the next `mul_mod`).  This replaces
a branch + subtract with a no-op in the inner loop.

**Work items:**
- Change `add_mod` / `sub_mod` to allow `[0, 2p)` outputs.
- Add a normalization pass at the end of `pointwise_mul` and `inverse`.
- Verify no overflow in the radix-2 structure (each stage at most doubles the
  dynamic range, so worst-case after log₂(N) stages is `[0, N·p)` — we need a
  cleanup before it overflows `u64`).

### 3. Merge `bit_reverse` with `pack`  (~5–10%)

Currently `pack` writes coefficients in natural order, then `bit_reverse`
permutes them in a second pass.  Write packed coefficients directly to their
bit-reversed positions, saving one full array pass.

### 4. Shift-expressible twiddle factors  (stage-dependent)

In Goldilocks primes, `2^k mod p = 2^k` when `2^k < p`.  The first few NTT
stages have twiddle factors that are pure powers of 2, so `mul_mod(t, 2^k)`
reduces to a shift + conditional subtract — no `u128` multiply needed.

### 5. Specialize `b = 32` lane  (~5%)

The `b = 32` prime (`0xFFFFFFFF00000001`) has the cleanest reduction identity
(splits a `u128` product exactly into 32-bit limbs).  A dedicated code path
for this prime alone could squeeze out a few more cycles vs. the generic
`match B` dispatch in `mul_mod`.

### 6. Asymmetric operand chunking  (conditional)

When `a ≫ b`, chunk the long operand, forward-transform the short operand
once, and reuse `b̂` (the transformed short operand) across all chunks.  Only
matters for extremely lopsided inputs.

### 7. u32-word support via u32 Solinas primes

The NTT path currently requires `Word = u64` and uses three 64-bit Solinas
primes.  For 32-bit (and potentially 16-bit) targets, we need a separate set
of u32-friendly Solinas primes of the form `2^32 − 2^b + 1`.

**Work items:**
- Find 2–3 primes `p = 2^32 − 2^b + 1` with `v2(p-1) ≥ 16` (enough for N up
  to 2^16) and distinct `b` values.
- Implement `FixedTrinomialSolinas32` (or equivalent) in `num-modular`, or
  hand-roll the 32-bit reduction inline.
- Generalize the NTT pipeline over `Word` size: the packing, transform, and
  CRT layers need to work with `u32` coefficients instead of `u64`.
- Assert `Word = u32` or `Word = u64` at entry and dispatch to the appropriate
  prime set.
