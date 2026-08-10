`FBig` and `DBig` are arbitrary-precision floats: a value is a significand
times the base raised to an exponent, with a **precision** cap on how many
significant digits the significand may keep and a **rounding mode** that
decides what happens when a result does not fit. This chapter explains how
those two knobs are defined and how they propagate through your computations:

- [Precision](./precision.md) — where a float's precision comes from at
  construction, how operators combine operand precisions, unlimited precision,
  and the complex case.
- [Rounding](./rounding.md) — the rounding-mode type parameter, the
  correctly-rounded guarantee, and the Ziv retry loop that certifies
  transcendentals.
- [Cached Arithmetic](./cached.md) — `CachedFBig` / `CachedCBig`, which reuse
  mathematical constants across a computation chain.
