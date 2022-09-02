
/// Implement `impl Op<A> for FBig` by converting A to FBig. This macro includes operations taking by references.
macro_rules! impl_binop_with_primitive {
    (impl $trait:ident<$target:ty>, $method:ident) => {
        impl<R: Round, const B: Word> $trait<$target> for FBig<R, B> {
            type Output = FBig<R, B>;
            #[inline]
            fn $method(self, rhs: $target) -> Self::Output {
                self.$method(FBig::<R, B>::from(rhs))
            }
        }

        impl<'l, R: Round, const B: Word> $trait<$target> for &'l FBig<R, B> {
            type Output = FBig<R, B>;
            #[inline]
            fn $method(self, rhs: $target) -> Self::Output {
                self.$method(FBig::<R, B>::from(rhs))
            }
        }

        impl<'r, R: Round, const B: Word> $trait<&'r $target> for FBig<R, B> {
            type Output = FBig<R, B>;
            #[inline]
            fn $method(self, rhs: &$target) -> Self::Output {
                self.$method(FBig::<R, B>::from(rhs.clone()))
            }
        }

        impl<'l, 'r, R: Round, const B: Word> $trait<&'r $target> for &'l FBig<R, B> {
            type Output = FBig<R, B>;
            #[inline]
            fn $method(self, rhs: &$target) -> Self::Output {
                self.$method(FBig::<R, B>::from(rhs.clone()))
            }
        }
    };
}

/// Implement `impl Op<A> for FBig` and `impl Op<FBig> for A` by converting A to FBig.
macro_rules! impl_commutative_binop_with_primitive {
    (impl $trait:ident<$target:ty>, $method:ident) => {
        crate::helper_macros::impl_binop_with_primitive!(impl $trait<$target>, $method);

        impl<R: Round, const B: Word> $trait<FBig<R, B>> for $target {
            type Output = FBig<R, B>;
            #[inline]
            fn $method(self, rhs: FBig<R, B>) -> Self::Output {
                FBig::<R, B>::from(self).$method(rhs)
            }
        }

        impl<'l, R: Round, const B: Word> $trait<FBig<R, B>> for &'l $target {
            type Output = FBig<R, B>;
            #[inline]
            fn $method(self, rhs: FBig<R, B>) -> Self::Output {
                FBig::<R, B>::from(self.clone()).$method(rhs)
            }
        }

        impl<'r, R: Round, const B: Word> $trait<&'r FBig<R, B>> for $target {
            type Output = FBig<R, B>;
            #[inline]
            fn $method(self, rhs: &FBig<R, B>) -> Self::Output {
                FBig::<R, B>::from(self).$method(rhs)
            }
        }

        impl<'l, 'r, R: Round, const B: Word> $trait<&'r FBig<R, B>> for &'l $target {
            type Output = FBig<R, B>;
            #[inline]
            fn $method(self, rhs: &FBig<R, B>) -> Self::Output {
                FBig::<R, B>::from(self.clone()).$method(rhs)
            }
        }
    };
}

/// Implement `impl OpAssign<A> for FBig` by converting A to FBig. This macro
/// includes operation with &A
macro_rules! impl_binop_assign_with_primitive {
    (impl $trait:ident<$target:ty>, $method:ident) => {
        impl<R: Round, const B: Word> $trait<$target> for FBig<R, B> {
            #[inline]
            fn $method(&mut self, rhs: $target) {
                self.$method(FBig::from(rhs))
            }
        }
        impl<R: Round, const B: Word> $trait<&$target> for FBig<R, B> {
            #[inline]
            fn $method(&mut self, rhs: &$target) {
                self.$method(FBig::from(rhs.clone()))
            }
        }
    };
}

pub(crate) use impl_binop_with_primitive;
pub(crate) use impl_binop_assign_with_primitive;
pub(crate) use impl_commutative_binop_with_primitive;
