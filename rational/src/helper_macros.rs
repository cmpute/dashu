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

pub(crate) use impl_binop_with_macro;
pub(crate) use impl_binop_assign_by_taking;
