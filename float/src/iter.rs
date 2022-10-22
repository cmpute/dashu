//! Implementation of core::iter traits

use crate::{fbig::FBig, repr::Word, round::Round};
use core::{
    iter::{Product, Sum},
    ops::{Add, Mul},
};

// XXX: implement precise summation of multiple floats
impl<T, R: Round, const B: Word> Sum<T> for FBig<R, B>
where
    Self: Add<T, Output = Self>,
{
    fn sum<I: Iterator<Item = T>>(iter: I) -> Self {
        iter.fold(FBig::ZERO, FBig::add)
    }
}

impl<T, R: Round, const B: Word> Product<T> for FBig<R, B>
where
    Self: Mul<T, Output = Self>,
{
    fn product<I: Iterator<Item = T>>(iter: I) -> Self {
        iter.fold(FBig::ONE, FBig::mul)
    }
}
