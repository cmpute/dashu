//! Advanced mathematical functions.
//!
//! Mirroring `dashu-float`'s `math` module, the transcendental functions live under here: [`trig`]
//! (complex `sin`/`cos`/`tan`/`asin`/`acos`/`atan`) and [`hyper`] (complex
//! `sinh`/`cosh`/`tanh`/`asinh`/`acosh`/`atanh`). The hyperbolic family is built from the circular
//! one via the rotation identities `sinh z = -i·sin(i·z)` etc., and `dashu-cmplx` reuses
//! `dashu-float`'s hyperbolic and constant-cache machinery directly rather than redefining it.

pub mod hyper;
pub mod trig;
