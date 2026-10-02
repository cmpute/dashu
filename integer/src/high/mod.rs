//! High-part (truncated) products and quotients with certified error bounds.
//!
//! These functions compute only the most significant words of a product or
//! quotient — the part that arbitrary-precision floating-point arithmetic
//! actually needs. They trade a certified error bound for roughly half the
//! work of the full operation, and are the building blocks for
//! correctly-rounded floating-point multiplication and division in
//! `dashu-float`.
//!
//! # Product error contract
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
//! # Quotient error contract
//!
//! [`div_high`] approximates the high `n + 1` words of a quotient with a
//! **two-sided** bound (`E = 2n + 2` ulps, no exactness flag) — see the
//! function documentation for the precise scale. A caller that must round
//! exactly declines whenever the error band straddles a rounding boundary.
//!
//! # Algorithm
//!
//! Small windows use a windowed column sweep (products) or an exact division
//! (quotients): each multiplier word (consumed two at a time by the shared
//! double-word kernel) only visits the suffix of the other operand whose
//! products reach the window; everything below is provably unable to
//! influence it except through the bounded error. Larger windows split into a
//! large high part and a small low part: an exact operation on the two high
//! blocks, high-part cross terms between the halves, and a dropped low-low
//! block that lies entirely below the window. The error analyses follow
//! Harvey & Zimmermann, "Short Division of Long Integers", ARITH-20 (2011).

mod div;
mod mul;
mod sqr;

pub use div::div_high;
pub use mul::mul_high;
pub use sqr::sqr_high;
