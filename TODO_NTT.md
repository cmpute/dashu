# NTT multiplication for `UBig` — status & remaining work

## Implemented

### Core algorithm

- **Primes.** Three Proth primes of the form `K·2^N + 1` per word size:
  64-bit: `29·2^57+1`, `71·2^57+1`, `75·2^57+1` (MAX_LOG_N=57).
  32-bit: `7·2^26+1`, `15·2^27+1`, `17·2^27+1` (MAX_LOG_N=26).
  Defined in `integer/src/arch/generic_{64,32}_bit/ntt.rs` with per-prime
  reducer instances `P0`/`P1`/`P2`.

- **Modular arithmetic.** Delegates to `num_modular::Reducer<Lane>` (Proth
  Montgomery reduction).  Transform functions are generic over
  `R: Reducer<Lane>`, monomorphized per prime at the `process_prime` call
  site.  No per-prime wrapper functions.

- **NTT transforms.** Iterative in-place radix-2 DIT.  Forward:
  `bit_reverse → forward(ω)`.  Inverse: `bit_reverse → forward(ω⁻¹) → scale`.
  All arithmetic in Montgomery form; conversion at pipeline boundaries via
  `r.transform`/`r.residue`.

- **CRT.** Garner's algorithm combining `K` residues into a `TripleWord`
  (`[u64;3]` on 64-bit, `[u32;3]` on 32-bit).  `CrtAccum` trait in
  `crt.rs`, impl gated by `#[cfg]` per word size.  Standard-form arithmetic
  via `num_modular::ModularCoreOps::subm`/`mulm`.

- **Dispatch.** `THRESHOLD_NTT = 4 000` words (256 kbits).  Asymmetric
  chunking (`a > 2·b`): pre-transforms `b` once, reuses `b̂` across chunks of
  `a`.  Shared entry point `process_prime(a, b: BInput<'_>, ctx, r)` handles
  both raw `b` and cached `b̂`.

### Architecture

- NTT constants and reducer instances live in arch-specific `ntt.rs` files,
  re-exported through `arch/mod.rs` → `crate::arch::ntt`.
- 16-bit Word targets excluded at compile time (`#[cfg]` on `pub(crate) mod ntt`).

### Benchmarking

- `ubig_mul_asymmetric` in `integer/benches/primitive.rs` — fixed `b` (500 kbits),
  varying `a` (1 kbit – 5 Mbits).  Exercises all chunked-mul code paths.

---

## Remaining optimisation opportunities

### 1. Radix-4 NTT  (~25–33% fewer twiddle multiplies)

Attempted but reverted — the 4-point DFT output ordering within DIT/DIF
interacts non-trivially with bit-reversal.  Needs careful re-analysis.

### 2. Harvey lazy-reduction butterflies  (~10–15%)

Bypass `r.add`/`r.sub` normalization in `ntt_core`, deferring to a cleanup
pass.  Trickiest due to interaction with `num_modular`'s `Reducer` API.

