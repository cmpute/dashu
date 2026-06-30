//! Advanced mathematical functions.
//!
//! Mirroring `dashu-float`'s `math` module, the transcendental functions live under here. Currently
//! only [`trig`] (complex `sin`/`cos`/`tan`/`asin`/`acos`/`atan`); `dashu-cmplx` reuses
//! `dashu-float`'s hyperbolic and constant-cache machinery directly rather than redefining it.

pub mod trig;
