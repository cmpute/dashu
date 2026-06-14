# Radix-4 NTT for `dashu-int`

## Context

The NTT multiplication path in `integer/src/mul/ntt/` currently uses an iterative in-place **radix-2** DIT transform. `TODO_NTT.md` lists "Radix-4 or split-radix NTT" as optimisation opportunity #1 — roughly halving the number of memory passes (`log₄(N)` stages instead of `log₂(N)`), with a small reduction in arithmetic work. Memory traffic, not arithmetic, is the dominant cost on the Goldilocks-style primes used here, so the pass count is the win that matters.

This plan implements radix-4 in place of the current radix-2 core, while keeping the rest of the pipeline (`pack`, `bit_reverse`, `pointwise_mul`, CRT, Garner, and the recently-introduced `NttGeometry` / `prepare_b_hat_and_twiddles` / `run_ntt_pipeline` / `process_prime` layering) untouched. A legacy copy of the radix-2 core is kept during development as a differential test oracle, then removed before the final commit.

Work happens in a new worktree `ntt-radix4` branching off the current `ssa` branch.

---

## Current state of the code (post-refactor)

The NTT module has been re-organised so that the conv and chunked paths share a single symmetric pipeline, with geometry constants factored into a small value type:

- `add_signed_mul` (mod.rs:143) dispatches: `a.len() > 2 * b.len()` → `add_signed_mul_chunked`; otherwise `add_signed_mul_conv`.
- **Both paths pre-transform `b` and precompute twiddles up front**, then call `run_ntt_pipeline`. There is no "raw b" vs "cached b" distinction — `b_hat`, `fwd_tw_cache`, `inv_tw_cache` are always populated by `prepare_b_hat_and_twiddles` (mod.rs:412) and passed in.
- `NttGeometry` (mod.rs:372) is a small value struct holding `nn`, `b_pack`, `k_eff`, `output_coeffs`. It's passed by reference into `prepare_b_hat_and_twiddles` and `run_ntt_pipeline`, and embedded inside `TransformCtx`.
- `TransformCtx` (mod.rs:380) is now just four scratch buffer slices (`a_lane`, `b_lane`, `fwd_twiddles`, `inv_twiddles`) plus a `geom: NttGeometry`. `prod` and `residues` are not in the ctx — they are allocated in `run_ntt_pipeline` (mod.rs:267–268) and passed as separate `&mut` arguments to `process_prime` and `do_crt`.
- `run_ntt_pipeline` (mod.rs:251) owns the per-call scratch allocation, runs the per-prime loop calling `process_prime`, then calls `do_crt` and signed-accumulates into `c_out`.
- `process_prime` (mod.rs:464) takes `(a, b_hat_slice, ctx, residues, pi, r)`. It transforms `a` from raw words, copies the pre-transformed `b_hat_slice` into `b_lane`, pointwise-multiplies, inverse-transforms, and writes residues. No twiddle precompute happens here.
- `transform_b_forward` (mod.rs:391) is the helper used by `prepare_b_hat_and_twiddles` to pack/Montgomery/bit-reverse/forward-transform `b`.

**Implication for radix-4:** the transform-level changes (radix-4 butterfly, expanded twiddle table) live entirely in `transform::forward` / `transform::inverse` / `transform::precompute_twiddles` / `transform::ntt_core`. Because every code path reaches the transform through these, the speedup propagates everywhere for free. The only multi-site edits in `mod.rs` are the twiddle *allocation sizes* and the cache-offset arithmetic, which now live in a small number of well-defined places.

---

## Math summary (verified)

Radix-4 DIT takes bit-reversed input and produces natural-order output — same I/O contract as the existing radix-2 DIT, so `bit_reverse`, `inverse()`, and all callers are unchanged.

For each butterfly on quad `(a0, a1, a2, a3)` at positions `(k, k+q, k+2q, k+3q)` within a length-`sub_len` group, with `q = sub_len/4` and `step = n/sub_len`:

```
b1 = a1 · ω_n^(k·step)
b2 = a2 · ω_n^(2k·step)
b3 = a3 · ω_n^(3k·step)
e0 = a0 + b2
e1 = a0 − b2
e2 = b1 + b3
e3 = b1 − b3                  // order matters: b1 − b3, not b3 − b1
y0 = e0 + e2
y1 = e1 + j·e3                // j = ω_n^(n/4), read from twiddles[n/4]
y2 = e0 − e2
y3 = e1 − j·e3
```

Iterative structure:
- If `log₂(n)` is even, stages run with `sub_len = 4, 16, 64, …, n` (pure radix-4).
- If `log₂(n)` is odd (n = 2·4^L), run **one** radix-2 stage with `sub_len = 2` (uses only `twiddles[0] = 1`), then radix-4 stages with `sub_len = 8, 32, …, n`.
- `n = 2` is a degenerate case — emit a single radix-2 butterfly with twiddle 1 and return early.

The constant `j = ω_n^(n/4)` is read once from `twiddles[n/4]` at the top of `ntt_core`. For inverse transforms, `twiddles[n/4]` holds `ω_n^(−n/4) = −j`, which is the *other* primitive 4th root; the same butterfly formula applies with it (a sign flip on the `j·e3` terms). No special handling needed — the symmetry falls out naturally.

The maximum twiddle index touched is `3k·step ≤ 3(n/4 − 1) ≈ 3n/4` at the final stage, which exceeds the current `n/2`-long table. **Fix: expand the twiddle table from `n/2` to `n` lanes.** Memory overhead ≈ +`n` lanes per table.

---

## Files to modify

### `integer/src/mul/ntt/transform.rs` (primary rewrite)

1. **`precompute_twiddles`** — change `assert!(out.len() >= n / 2)` to `assert!(out.len() >= n)`, and extend the fill loop from `1..(n/2)` to `1..n`. Output is now `ω_n^k` for `k ∈ [0, n)`.

2. **`ntt_core` (rewrite)** — replace with the radix-4 algorithm:
   ```rust
   fn ntt_core<R: Reducer<Lane>>(a: &mut [Lane], twiddles: &[Lane], r: &R) {
       let n = a.len();
       debug_assert!(n.is_power_of_two() && twiddles.len() >= n);
       if n == 1 { return; }
       if n == 2 {
           // Radix-2 fallback: twiddle = twiddles[0] = 1.
           let u = a[0]; let v = a[1];
           a[0] = r.add(&u, &v);
           a[1] = r.sub(&u, &v);
           return;
       }
       let j_mont = twiddles[n / 4];

       let log_n = n.trailing_zeros();
       let mut sub_len = if log_n & 1 == 1 {
           // One radix-2 stage with step = n/2 (only k=0, twiddle = 1).
           for i in (0..n).step_by(2) {
               let u = a[i]; let v = a[i + 1];
               a[i]     = r.add(&u, &v);
               a[i + 1] = r.sub(&u, &v);
           }
           8
       } else {
           4
       };

       // Radix-4 stages.
       while sub_len <= n {
           let q = sub_len / 4;
           let step = n / sub_len;
           for i in (0..n).step_by(sub_len) {
               // k = 0: twiddles are all 1, skip the multiplies.
               butterfly_radix4(a, i, q, twiddles[0], twiddles[0], twiddles[0], j_mont, r);
               for k in 1..q {
                   let t1 = twiddles[k * step];
                   let t2 = twiddles[2 * k * step];
                   let t3 = twiddles[3 * k * step];
                   butterfly_radix4(a, i + k, q, t1, t2, t3, j_mont, r);
               }
           }
           sub_len *= 4;
       }
   }
   ```
   `butterfly_radix4` is a small `#[inline(always)]` helper that performs the four muls (`b1, b2, b3` + `j·e3`) and writes back to the four positions. Make sure reads of `a[idx]` happen before any writes.

3. **Keep a private `ntt_core_radix2_legacy`** during development — the current body of `ntt_core`, renamed. It reads only `twiddles[0..n/2]` so it works fine on the expanded table. Used only by the cross-check test (below) and deleted before merge.

4. **Tests** — update local allocations in tests from `n/2` to `n` lanes. Extend `test_forward_correctness` to cover `n ∈ {2, 4, 8, 16, 32}` (currently only `{2, 4, 8}`). Add:
   - `test_radix4_matches_legacy` — for each prime and `n ∈ {2, 4, 8, 16, 32, 64, 128, 256}`, run forward via both `ntt_core` and `ntt_core_radix2_legacy` on identical random bit-reversed input, assert byte-equal output. Delete together with the legacy fn before merge.

### `integer/src/mul/ntt/mod.rs` (memory layout — four logical sites)

Because of the recent refactor, twiddle allocation is centralised. The current code uses `nn / 2` for twiddle lengths in a small number of well-defined places, all of which need to become `nn`:

1. **`memory_requirement_up_to` (line 98)** — worst-case scratch bound. The `twiddles = n_max` constant assumes two tables of size `n/2` (forward + inverse). Bump to `2 * n_max`:
   ```rust
   let twiddles = 2 * n_max;   // was n_max
   ```

2. **`run_ntt_pipeline` (lines 271–272, 283, 284–287)** — per-call scratch allocation + cache slicing inside the per-prime loop:
   ```rust
   // line 271-272
   let (fwd_twiddles, mut m) = m.allocate_slice_fill::<Lane>(nn, 0);       // was nn / 2
   let (inv_twiddles, _) = m.allocate_slice_fill::<Lane>(nn, 0);           // was nn / 2
   // line 283
   let tw_off = pi * nn;                                                    // was pi * (nn / 2)
   // line 284-287
   ctx.fwd_twiddles.copy_from_slice(&fwd_tw_cache[tw_off..tw_off + nn]);   // was nn / 2
   ctx.inv_twiddles.copy_from_slice(&inv_tw_cache[tw_off..tw_off + nn]);   // was nn / 2
   ```

3. **`prepare_b_hat_and_twiddles` (lines 411 docstring, 428–429, 452, 454–455)** — per-prime scratch during precompute + cache write-back:
   ```rust
   // docstring at line ~411: "fwd_tw_cache and inv_tw_cache each geom.k_eff * geom.nn"
   //                                                            (was geom.k_eff * (geom.nn / 2))
   // line 428-429
   let (fwd_tw, mut rest) = rest.allocate_slice_fill::<Lane>(nn, 0);       // was nn / 2
   let (inv_tw, _) = rest.allocate_slice_fill::<Lane>(nn, 0);              // was nn / 2
   // line 452
   let tw_off = pi * nn;                                                    // was pi * (nn / 2)
   // line 454-455
   fwd_tw_cache[tw_off..tw_off + nn].copy_from_slice(fwd_tw);              // was nn / 2
   inv_tw_cache[tw_off..tw_off + nn].copy_from_slice(inv_tw);              // was nn / 2
   ```

4. **Cache length computations** — two sites that derive the total cache size from `nn`:
   - `add_signed_mul_chunked` line 184: `let twiddle_len = k_eff * nn_chunk;` (was `k_eff * (nn_chunk / 2)`)
   - `add_signed_mul_conv` line 334: `let twiddle_len = k_eff * nn;` (was `k_eff * (nn / 2)`)

`NttGeometry` itself does **not** store twiddle size — only `nn`, `b_pack`, `k_eff`, `output_coeffs`. The `nn / 2` → `nn` change is local to the four sites above; no field needs adding to the geometry struct.

### `integer/CHANGELOG.md`

Add under `## Unreleased` → `### Improve`:
> NTT inner transform rewritten as radix-4 DIT (with one radix-2 stage when N is not a power of 4), halving the number of passes over the coefficient array. ~20–30% faster large-integer multiplication above the NTT threshold.

### `TODO_NTT.md`

Mark section "1. Radix-4 or split-radix NTT" as completed (move from "Remaining" to "Implemented", or strike through with a dated note). Leave split-radix as a possible future improvement.

### No changes

- `integer/src/arch/generic_64_bit/ntt.rs` and `integer/src/arch/generic_32_bit/ntt.rs` — primes and `OMEGA_MAX` are already sufficient. `MAX_LOG_N ≥ 2` is all radix-4 needs.
- `integer/src/mul/ntt/pack.rs`, `crt.rs` — unaffected.
- All callers in `mod.rs` (`add_signed_mul_conv`, `add_signed_mul_chunked` closure, `prepare_b_hat_and_twiddles`, `transform_b_forward`, `process_prime`, `do_crt`, `NttGeometry`, `TransformCtx`) — unaffected because `forward`/`inverse` signatures are unchanged.

---

## Implementation sequence

1. **Enter the worktree** `ntt-radix4` off the current `ssa` HEAD (via the worktree tool).
2. **Commit 1 — Twiddle table expansion.** Bump allocation sizes and cache-offset arithmetic in `transform.rs::precompute_twiddles`, and at all the mod.rs sites listed above. Also bump test-local allocations. The old radix-2 still works correctly on the now-oversized table. Run `cargo test -p dashu-int mul::ntt` — everything should pass.
3. **Commit 2 — Radix-4 core + legacy differential test.** Rename the existing `ntt_core` body to `ntt_core_radix2_legacy`. Write the new `ntt_core` (radix-4). Add `test_radix4_matches_legacy`. Extend `test_forward_correctness` to n ∈ {2, 4, 8, 16, 32}. Run the full NTT test suite; the schoolbook comparison tests in `mod.rs` are the strongest correctness gate.
4. **Commit 3 — Cleanup + docs.** Delete `ntt_core_radix2_legacy` and `test_radix4_matches_legacy`. Update `CHANGELOG.md` and `TODO_NTT.md`.
5. **Verification.** Run the existing `crossover_ntt` ignored test (`cargo test -p dashu-int --release -- mul::threshold_tests::crossover_ntt --ignored --nocapture`) and compare timings before/after by checking out `ssa` temporarily. A 20–30% drop in the NTT column at sizes ≥ 4096 words confirms the optimisation landed.

## Verification end-to-end

- `cargo check --all-features --tests`
- `cargo test --workspace --exclude dashu-python`
- `cargo clippy --all-features --all-targets --workspace --exclude dashu-python -- -D warnings`
- `cargo fmt --all -- --check`
- `cargo test -p dashu-int --release -- mul::threshold_tests::crossover_ntt --ignored --nocapture` — for end-to-end timing sanity.
