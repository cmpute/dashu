# Changelog

## 0.5.0

### Fix
- The inline `Display`/`FromStr` unit tests failed to compile under `no_std`
  (`cargo test --no-default-features`): the test modules now import `alloc::format`.

### Add
- New crate `dashu-cmplx` providing the arbitrary-precision complex number type [`CBig`], built on top of
  [`dashu-float`]'s `FBig`. Each `CBig` stores a real and an imaginary part (`Repr`) over a single shared
  precision and rounding mode, mirroring `FBig`'s `Repr`+`Context` layout.
- Two-layer API mirroring `FBig`: context-layer operations on [`Context`] return a `CfpResult`
  (`Result<CRounded<CBig>, FpError>`) carrying per-axis inexactness `(Rounding, Rounding)`, while the
  convenience layer (`CBig::add`, operators) unwraps to a plain `CBig` (panicking on `Indeterminate` /
  `OutOfDomain` / `InfiniteInput`, saturating `Overflow`/`Underflow` to signed infinity/zero).
- Field arithmetic: `add`/`sub`/`neg`/`sqr`/`mul`/`div`/`inv` plus scalar `mul`/`div` by a real `FBig`
  through mixed-type operators. `mul`/`div`/`sqr`/`inv` are near-correctly rounded via the guard-digit
  recipe (mirroring `FBig`'s transcendentals; a guaranteed-correct Ziv loop is deferred to 0.5.x).
- Integer power `powi` (repeated squaring) and complex power `powf` (`exp(w·log z)`).
- Decomposition & misc: `re`/`imag`/`into_parts`/`from_parts`, `conj`/`proj`/`mul_i`, `abs` (`hypot`),
  `norm` (squared modulus), `arg` (`atan2`).
- Transcendentals: `sqrt`, `exp`, `log`, `sin`/`cos`/`tan`/`sin_cos`, `asin`/`acos`/`atan`. Complex trig
  uses the real–imaginary decomposition reusing `FBig`'s `sinh`/`cosh`.
- Comparison surface mirroring `FBig`: lexicographic `Ord`/`PartialOrd` (by `re`, then `im`), `AbsOrd`,
  and `NumOrd`/`NumHash` (behind the `num-order` feature).
- Algebraic `"a+bi"` `Display`/`FromStr`, structured `Debug`, and the `I`/`ZERO`/`ONE` constants.
- The `cbig!` / `static_cbig!` literal macros (in `dashu-macros`) for creating `CBig` from a complex
  literal (`a+bi` or `re, im`); exposed as `dashu::cbig!` in the meta-crate.
- Random generation via `rand`: the `rand` feature (aliasing `rand_v08`, with `rand_v09`/`rand_v010`
  opt-in, matching the other crates). `UniformCBig` samples the box `[low, high)`; the builtin
  `Standard`/`StandardUniform`/`Open01`/`OpenClosed01` sample the unit square `[0,1)²` (each part an
  independent uniform `FBig`). Reuses `dashu-float`'s `UniformFBig` — no bespoke sampling algorithm.
- No-NaN policy: C99 NaN-producing cases are mapped to `FpError` at the context layer (and panics at the
  convenience layer), consistent with `FBig`. Signed zero and the C99 Annex G / Kahan branch-cut model
  are first-class (reusing `FBig`'s signed-zero predicates).
