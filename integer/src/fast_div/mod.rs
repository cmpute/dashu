//! Prepared divisor types for fast division

mod barret;
mod const_div;
pub(crate) use barret::{FastDivideNormalized, FastDivideNormalized2, FastDivideSmall};
pub(crate) use const_div::ConstDivisor; // TODO: implement related API and then make it public
pub(crate) use const_div::{
    ConstDivisorRepr, ConstDoubleDivisor, ConstLargeDivisor, ConstSingleDivisor,
};

// XXX: Add implementation for exact division check, the prepared divisor type could be called `ExactDivisor`
