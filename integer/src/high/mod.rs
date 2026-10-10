//! High-part (truncated) products with certified error bounds.
//!
//! These functions compute only the most significant words of a product — the
//! part that arbitrary-precision floating-point arithmetic actually needs —
//! together with a flag telling whether anything was dropped below the window.
//! They trade a certified, one-sided error bound for roughly half the work of a
//! full product, and are the building blocks for correctly-rounded
//! floating-point multiplication in `dashu-float`.
//!
//! # Error contract
//!
//! With `n = out_words` (after the clamping described on each function),
//! `wa = words(a)`, `wb = words(b)` and `s = WORD_BITS * (wa + wb - n)`, the
//! returned value `v` and the true product satisfy
//!
//! ```text
//! v  <=  (a * b) >> s  <  v + (n + 2)
//! ```
//!
//! i.e. `v` never over-estimates the truncated product and under-estimates it
//! by less than `n + 2` units in the last place of `v` (one ulp per multiplier
//! sweep, plus slack for the operand truncation). The returned flag is `true`
//! exactly when some dropped contribution is nonzero, so a `false` flag
//! guarantees `v == (a * b) >> s` exactly.
//!
//! # Algorithm
//!
//! Small windows use a windowed column sweep: each multiplier word (consumed
//! two at a time by the shared double-word kernel) only visits the suffix of
//! the other operand whose products reach the window; everything below is
//! provably unable to influence it except through the bounded error. Larger
//! windows split into a large high part and a small low part: an exact product
//! of the two high blocks, two recursive high-part cross products, and a
//! dropped low-low block that lies entirely below the window. The error
//! analysis follows Harvey & Zimmermann, "Short Division of Long Integers",
//! ARITH-20 (2011).

mod mul;
mod sqr;

pub use mul::mul_high;
pub use sqr::sqr_high;
