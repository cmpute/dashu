//! Prepared divisor types for fast division

mod barret;
mod const_divisor;
pub(crate) use barret::{FastDivideNormalized, FastDivideNormalized2, FastDivideSmall};
pub(crate) use const_divisor::{ConstSingleDivisor, ConstDoubleDivisor, ConstLargeDivisor, ConstDivisorRepr};
pub use const_divisor::ConstDivisor;
