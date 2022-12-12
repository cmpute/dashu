//! Implementation of core::iter traits

use crate::rbig::{RBig, Relaxed};
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

impl_fold_iter!(RBig, Sum, sum, Add, add, ZERO);
impl_fold_iter!(Relaxed, Sum, sum, Add, add, ZERO);
impl_fold_iter!(RBig, Product, product, Mul, mul, ONE);
impl_fold_iter!(Relaxed, Product, product, Mul, mul, ONE);
