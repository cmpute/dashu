/// Implement `impl Op<B> for A` by calling `$impl`. The arguments for this
/// macro will be `(self.numerator, self.denominator, other.numerator, other.denominator,
/// &self.numerator, &self.denominator, &other.numerator, &other.denominator, $method)`.
macro_rules! impl_binop_with_macro {
    ($trait:ident, $method:ident, $impl:ident) => {
        crate::helper_macros::impl_binop_with_macro!($trait, $method, crate::rbig::RBig, $impl);
    };
    ($trait:ident, $method:ident, $t:ty, $impl:ident) => {
        impl $trait for $t {
            type Output = $t;
            fn $method(self, rhs: $t) -> $t {
                let (a, b) = self.into_parts();
                let (c, d) = rhs.into_parts();
                let (ra, rb, rc, rd) = (&a, &b, &c, &d);
                $impl!(a, b, c, d, ra, rb, rc, rd, $method)
            }
        }

        impl<'r> $trait<&'r $t> for $t {
            type Output = $t;
            fn $method(self, rhs: &$t) -> $t {
                let (a, b) = self.into_parts();
                let (c, d) = (rhs.numerator(), rhs.denominator());
                let (ra, rb, rc, rd) = (&a, &b, c, d);
                $impl!(a, b, c, d, ra, rb, rc, rd, $method)
            }
        }

        impl<'l> $trait<$t> for &'l $t {
            type Output = $t;
            fn $method(self, rhs: $t) -> $t {
                let (a, b) = (self.numerator(), self.denominator());
                let (c, d) = rhs.into_parts();
                let (ra, rb, rc, rd) = (a, b, &c, &d);
                $impl!(a, b, c, d, ra, rb, rc, rd, $method)
            }
        }

        impl<'l, 'r> $trait<&'r $t> for &'l $t {
            type Output = $t;
            fn $method(self, rhs: &$t) -> $t {
                let (a, b) = (self.numerator(), self.denominator());
                let (c, d) = (rhs.numerator(), rhs.denominator());
                let (ra, rb, rc, rd) = (a, b, c, d);
                $impl!(a, b, c, d, ra, rb, rc, rd, $method)
            }
        }
    };
}

/// Implement `impl Op<B> for A` by calling `$impl`. The arguments for this
/// macro will be `(self.numerator, self.denominator, other,
/// &self.numerator, &self.denominator, &other, $method)`.
macro_rules! impl_binop_with_int {
    (impl $trait:ident<$int:ty>, $method:ident, $impl:ident) => {
        crate::helper_macros::impl_binop_with_int!(impl $trait<$int>, $method, crate::rbig::RBig, $impl);
    };
    (impl $trait:ident<$int:ty>, $method:ident, $t:ty, $impl:ident) => {
        impl $trait<$int> for $t {
            type Output = $t;
            fn $method(self, rhs: $int) -> $t {
                let (a, b) = self.into_parts();
                let (ra, rb, ri) = (&a, &b, &rhs);
                $impl!(a, b, rhs, ra, rb, ri, $method)
            }
        }

        impl<'r> $trait<&'r $int> for $t {
            type Output = $t;
            fn $method(self, rhs: &$int) -> $t {
                let (a, b) = self.into_parts();
                let (ra, rb, ri) = (&a, &b, rhs);
                $impl!(a, b, rhs, ra, rb, ri, $method)
            }
        }

        impl<'l> $trait<$int> for &'l $t {
            type Output = $t;
            fn $method(self, rhs: $int) -> $t {
                let (ra, rb, ri) = (self.numerator(), self.denominator(), &rhs);
                let (a, b) = (ra.clone(), rb.clone());
                $impl!(a, b, rhs, ra, rb, ri, $method)
            }
        }

        impl<'l, 'r> $trait<&'r $int> for &'l $t {
            type Output = $t;
            fn $method(self, rhs: &$int) -> $t {
                let (ra, rb, ri) = (self.numerator(), self.denominator(), rhs);
                let (a, b) = (ra.clone(), rb.clone());
                $impl!(a, b, rhs, ra, rb, ri, $method)
            }
        }
    };
    (impl $trait:ident for $int:ty, $method:ident, $impl:ident) => {
        crate::helper_macros::impl_binop_with_int!(impl $trait for $int, $method, crate::rbig::RBig, $impl);
    };
    (impl $trait:ident for $int:ty, $method:ident, $t:ty, $impl:ident) => {
        impl $trait<$t> for $int {
            type Output = $t;
            fn $method(self, rhs: $t) -> $t {
                let (a, b) = rhs.into_parts();
                let (ra, rb, ri) = (&a, &b, &self);
                $impl!(a, b, self, ra, rb, ri, $method)
            }
        }

        impl<'r> $trait<&'r $t> for $int {
            type Output = $t;
            fn $method(self, rhs: &$t) -> $t {
                let (ra, rb, ri) = (rhs.numerator(), rhs.denominator(), &self);
                let (a, b) = (ra.clone(), rb.clone());
                $impl!(a, b, self, ra, rb, ri, $method)
            }
        }

        impl<'l> $trait<$t> for &'l $int {
            type Output = $t;
            fn $method(self, rhs: $t) -> $t {
                let (a, b) = rhs.into_parts();
                let (ra, rb, ri) = (&a, &b, self);
                $impl!(a, b, self, ra, rb, ri, $method)
            }
        }

        impl<'l, 'r> $trait<&'r $t> for &'l $int {
            type Output = $t;
            fn $method(self, rhs: &$t) -> $t {
                let (ra, rb, ri) = (rhs.numerator(), rhs.denominator(), self);
                let (a, b) = (ra.clone(), rb.clone());
                $impl!(a, b, self, ra, rb, ri, $method)
            }
        }
    }
}

/// Implement `impl OpAssign<B> for A` by forwarding to `*A = mem::take(A).op(B)`, including &B.
macro_rules! impl_binop_assign_by_taking {
    (impl $trait:ident<$t2:ty> for $t1:ty, $methodassign:ident, $method:ident) => {
        impl $trait<$t2> for $t1 {
            #[inline]
            fn $methodassign(&mut self, rhs: $t2) {
                *self = core::mem::take(self).$method(rhs);
            }
        }
        impl $trait<&$t2> for $t1 {
            #[inline]
            fn $methodassign(&mut self, rhs: &$t2) {
                *self = core::mem::take(self).$method(rhs);
            }
        }
    };
}

pub(crate) use impl_binop_assign_by_taking;
pub(crate) use impl_binop_with_int;
pub(crate) use impl_binop_with_macro;
