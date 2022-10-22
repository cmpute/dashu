//! Implementation of core::iter traits

use crate::{ibig::IBig, ubig::UBig};
use core::{
    iter::{Product, Sum},
    ops::{Add, Mul},
};

macro_rules! impl_fold_iter {
    ($t:ty, $fold_trait:ident, $fold:ident, $op_trait:ident, $op:ident, $init:ident) => {
        impl<T> $fold_trait<T> for $t
        where
            $t: $op_trait<T, Output = $t>,
        {
            fn $fold<I: Iterator<Item = T>>(iter: I) -> Self {
                iter.fold(<$t>::$init, <$t>::$op)
            }
        }
    };
}

impl_fold_iter!(UBig, Sum, sum, Add, add, ZERO);
impl_fold_iter!(IBig, Sum, sum, Add, add, ZERO);
impl_fold_iter!(UBig, Product, product, Mul, mul, ONE);
impl_fold_iter!(IBig, Product, product, Mul, mul, ONE);
