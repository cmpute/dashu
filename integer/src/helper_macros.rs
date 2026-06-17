/// Execute the expression, and assert the return value is zero in debug mode.
/// The expression is still executed in release mode. It's usually used for assertions on overflow.
macro_rules! debug_assert_zero {
    ($($arg:tt)*) => {{
        let __check__ = $($arg)*;
        debug_assert_eq!(__check__ as $crate::arch::word::DoubleWord, 0);
    }};
}

/// Implement `impl Op<B> for A` by converting B to A. This macro includes operations
/// with A, B swapped and taking by references.
macro_rules! impl_binop_with_primitive {
    (impl $trait:ident<$target:ty> for $t:ty, $method:ident) => {
        crate::helper_macros::impl_binop_with_primitive!(impl $trait<$target> for $t, $method -> $t);
    };
    (impl $trait:ident<$target:ty> for $t:ty, $method:ident -> $omethod:ty) => {
        impl $trait<$target> for $t {
            type Output = $omethod;
            #[inline]
            fn $method(self, rhs: $target) -> $omethod {
                self.$method(<$t>::from(rhs)).try_into().unwrap()
            }
        }

        impl<'l> $trait<$target> for &'l $t {
            type Output = $omethod;
            #[inline]
            fn $method(self, rhs: $target) -> $omethod {
                self.$method(<$t>::from(rhs)).try_into().unwrap()
            }
        }

        impl<'r> $trait<&'r $target> for $t {
            type Output = $omethod;
            #[inline]
            fn $method(self, rhs: &$target) -> $omethod {
                self.$method(<$t>::from(*rhs)).try_into().unwrap()
            }
        }

        impl<'l, 'r> $trait<&'r $target> for &'l $t {
            type Output = $omethod;
            #[inline]
            fn $method(self, rhs: &$target) -> $omethod {
                self.$method(<$t>::from(*rhs)).try_into().unwrap()
            }
        }
    };
}

macro_rules! impl_commutative_binop_with_primitive {
    (impl $trait:ident<$target:ty> for $t:ty, $method:ident) => {
        crate::helper_macros::impl_commutative_binop_with_primitive!(impl $trait<$target> for $t, $method -> $t);
    };
    (impl $trait:ident<$target:ty> for $t:ty, $method:ident -> $omethod:ty) => {
        crate::helper_macros::impl_binop_with_primitive!(impl $trait<$target> for $t, $method -> $omethod);

        impl $trait<$t> for $target {
            type Output = $omethod;
            #[inline]
            fn $method(self, rhs: $t) -> $omethod {
                <$t>::from(self).$method(rhs).try_into().unwrap()
            }
        }

        impl<'l> $trait<$t> for &'l $target {
            type Output = $omethod;
            #[inline]
            fn $method(self, rhs: $t) -> $omethod {
                <$t>::from(*self).$method(rhs).try_into().unwrap()
            }
        }

        impl<'r> $trait<&'r $t> for $target {
            type Output = $omethod;
            #[inline]
            fn $method(self, rhs: &$t) -> $omethod {
                <$t>::from(self).$method(rhs).try_into().unwrap()
            }
        }

        impl<'l, 'r> $trait<&'r $t> for &'l $target {
            type Output = $omethod;
            #[inline]
            fn $method(self, rhs: &$t) -> $omethod {
                <$t>::from(*self).$method(rhs).try_into().unwrap()
            }
        }
    };
}

/// Implement `impl OpAssign<B> for A` by converting B to A. This macro
/// includes operation with &B
macro_rules! impl_binop_assign_with_primitive {
    (impl $trait:ident<$target:ty> for $t:ty, $method:ident) => {
        impl $trait<$target> for $t {
            #[inline]
            fn $method(&mut self, rhs: $target) {
                self.$method(<$t>::from(rhs))
            }
        }
        impl $trait<&$target> for $t {
            #[inline]
            fn $method(&mut self, rhs: &$target) {
                self.$method(<$t>::from(*rhs))
            }
        }
    };
    // this branch is currently only used for DivRemAssign
    (impl $trait:ident<$target:ty> for $t:ty, $method:ident, $output:ident = $ty_output:ty) => {
        impl $trait<$target> for $t {
            type $output = $ty_output;
            #[inline]
            fn $method(&mut self, rhs: $target) -> $ty_output {
                self.$method(<$t>::from(rhs)).try_into().unwrap()
            }
        }
        impl $trait<&$target> for $t {
            type $output = $ty_output;
            #[inline]
            fn $method(&mut self, rhs: &$target) -> $ty_output {
                self.$method(<$t>::from(*rhs)).try_into().unwrap()
            }
        }
    };
}

/// Implement `impl Op<UBig> for UBig` by forwarding to `lhs.repr().op(rhs.repr())`, including &UBig.
/// The output type is UBig.
///
/// For two output cases, the $impl argument takes a function-like macro as input, it should takes the
/// (repr0, repr1) as input
macro_rules! forward_ubig_binop_to_repr {
    // normal operator
    (impl $trait:ident, $method:ident) => {
        crate::helper_macros::forward_ubig_binop_to_repr!(impl $trait, $method, $method);
    };
    // operator with different forwarded function
    (impl $trait:ident, $method:ident, $forward:ident) => {
        impl $trait<UBig> for UBig {
            type Output = UBig;

            #[inline]
            fn $method(self, rhs: UBig) -> UBig {
                UBig(self.into_repr().$forward(rhs.into_repr()))
            }
        }

        impl<'r> $trait<&'r UBig> for UBig {
            type Output = UBig;

            #[inline]
            fn $method(self, rhs: &UBig) -> UBig {
                UBig(self.into_repr().$forward(rhs.repr()))
            }
        }

        impl<'l> $trait<UBig> for &'l UBig {
            type Output = UBig;

            #[inline]
            fn $method(self, rhs: UBig) -> UBig {
                UBig(self.repr().$forward(rhs.into_repr()))
            }
        }

        impl<'l, 'r> $trait<&'r UBig> for &'l UBig {
            type Output = UBig;

            #[inline]
            fn $method(self, rhs: &UBig) -> UBig {
                UBig(self.repr().$forward(rhs.repr()))
            }
        }
    };
    (impl $trait:ident, $method:ident -> $omethod:ty, $o1:ident = $ty_o1:ty, $o2:ident = $ty_o2:ty, $impl:ident) => {
        impl $trait<UBig> for UBig {
            type $o1 = $ty_o1;
            type $o2 = $ty_o2;

            #[inline]
            fn $method(self, rhs: UBig) -> $omethod {
                let (repr0, repr1) = (self.into_repr(), rhs.into_repr());
                $impl!(repr0, repr1)
            }
        }

        impl<'r> $trait<&'r UBig> for UBig {
            type $o1 = $ty_o1;
            type $o2 = $ty_o2;

            #[inline]
            fn $method(self, rhs: &UBig) -> $omethod {
                let (repr0, repr1) = (self.into_repr(), rhs.repr());
                $impl!(repr0, repr1)
            }
        }

        impl<'l> $trait<UBig> for &'l UBig {
            type $o1 = $ty_o1;
            type $o2 = $ty_o2;

            #[inline]
            fn $method(self, rhs: UBig) -> $omethod {
                let (repr0, repr1) = (self.repr(), rhs.into_repr());
                $impl!(repr0, repr1)
            }
        }

        impl<'l, 'r> $trait<&'r UBig> for &'l UBig {
            type $o1 = $ty_o1;
            type $o2 = $ty_o2;

            #[inline]
            fn $method(self, rhs: &UBig) -> $omethod {
                let (repr0, repr1) = (self.repr(), rhs.repr());
                $impl!(repr0, repr1)
            }
        }
    };
}

// TODO(next): modify this macro so that the $impl function can be shared between ibig op ibig and ubig op ibig

/// Implement `impl Op<IBig> for IBig` by forwarding to the function-like macro `$impl` with arguments
/// `(lhs_sign, lhs_repr, rhs_sign, rhs_repr)`, including &IBig.
/// The output type is IBig.
macro_rules! forward_ibig_binop_to_repr {
    (impl $trait:ident, $method:ident, $output:ident = $ty_output:ty, $impl:ident) => {
        impl $trait<IBig> for IBig {
            type $output = $ty_output;

            #[inline]
            fn $method(self, rhs: IBig) -> $ty_output {
                let (sign0, mag0) = self.into_sign_repr();
                let (sign1, mag1) = rhs.into_sign_repr();
                $impl!(sign0, mag0, sign1, mag1)
            }
        }

        impl<'r> $trait<&'r IBig> for IBig {
            type $output = $ty_output;

            #[inline]
            fn $method(self, rhs: &IBig) -> $ty_output {
                let (sign0, mag0) = self.into_sign_repr();
                let (sign1, mag1) = rhs.as_sign_repr();
                $impl!(sign0, mag0, sign1, mag1)
            }
        }

        impl<'l> $trait<IBig> for &'l IBig {
            type $output = $ty_output;

            #[inline]
            fn $method(self, rhs: IBig) -> $ty_output {
                let (sign0, mag0) = self.as_sign_repr();
                let (sign1, mag1) = rhs.into_sign_repr();
                $impl!(sign0, mag0, sign1, mag1)
            }
        }

        impl<'l, 'r> $trait<&'r IBig> for &'l IBig {
            type $output = $ty_output;

            #[inline]
            fn $method(self, rhs: &IBig) -> $ty_output {
                let (sign0, mag0) = self.as_sign_repr();
                let (sign1, mag1) = rhs.as_sign_repr();
                $impl!(sign0, mag0, sign1, mag1)
            }
        }
    };
    (impl $trait:ident, $method:ident -> $omethod:ty, $o1:ident = $ty_o1:ty, $o2:ident = $ty_o2:ty, $impl:ident) => {
        impl $trait<IBig> for IBig {
            type $o1 = $ty_o1;
            type $o2 = $ty_o2;

            #[inline]
            fn $method(self, rhs: IBig) -> $omethod {
                let (sign0, mag0) = self.into_sign_repr();
                let (sign1, mag1) = rhs.into_sign_repr();
                $impl!(sign0, mag0, sign1, mag1)
            }
        }

        impl<'r> $trait<&'r IBig> for IBig {
            type $o1 = $ty_o1;
            type $o2 = $ty_o2;

            #[inline]
            fn $method(self, rhs: &IBig) -> $omethod {
                let (sign0, mag0) = self.into_sign_repr();
                let (sign1, mag1) = rhs.as_sign_repr();
                $impl!(sign0, mag0, sign1, mag1)
            }
        }

        impl<'l> $trait<IBig> for &'l IBig {
            type $o1 = $ty_o1;
            type $o2 = $ty_o2;

            #[inline]
            fn $method(self, rhs: IBig) -> $omethod {
                let (sign0, mag0) = self.as_sign_repr();
                let (sign1, mag1) = rhs.into_sign_repr();
                $impl!(sign0, mag0, sign1, mag1)
            }
        }

        impl<'l, 'r> $trait<&'r IBig> for &'l IBig {
            type $o1 = $ty_o1;
            type $o2 = $ty_o2;

            #[inline]
            fn $method(self, rhs: &IBig) -> $omethod {
                let (sign0, mag0) = self.as_sign_repr();
                let (sign1, mag1) = rhs.as_sign_repr();
                $impl!(sign0, mag0, sign1, mag1)
            }
        }
    };
}

/// Implement `impl OpAssign<B> for A` by forwarding to `*A = mem::take(A).op(B)`, including &B.
macro_rules! impl_binop_assign_by_taking {
    (impl $trait:ident<$t2:ty> for $t1:ty, $methodassign:ident, $method:ident) => {
        // the .into() conversion at the end is to convert UBig to IBig in bitand_assign
        impl $trait<$t2> for $t1 {
            #[inline]
            fn $methodassign(&mut self, rhs: $t2) {
                *self = core::mem::take(self).$method(rhs).into();
            }
        }
        impl $trait<&$t2> for $t1 {
            #[inline]
            fn $methodassign(&mut self, rhs: &$t2) {
                *self = core::mem::take(self).$method(rhs).into();
            }
        }
    };
    // this branch is currently only used for DivRemAssign
    (impl $trait:ident<$t2:ty> for $t1:ty, $methodassign:ident, $output:ident = $ty_output:ty, $method:ident) => {
        impl $trait<$t2> for $t1 {
            type $output = $ty_output;
            #[inline]
            fn $methodassign(&mut self, rhs: $t2) -> $ty_output {
                let (a, b) = core::mem::take(self).$method(rhs);
                *self = a;
                b
            }
        }
        impl $trait<&$t2> for $t1 {
            type $output = $ty_output;
            #[inline]
            fn $methodassign(&mut self, rhs: &$t2) -> $ty_output {
                let (a, b) = core::mem::take(self).$method(rhs);
                *self = a;
                b
            }
        }
    };
}

/// Implement `impl Op<IBig> for UBig` by forwarding to the macro `$impl` with arguments
/// `(self_repr, rhs_sign, rhs_repr)`
macro_rules! forward_ubig_ibig_binop_to_repr {
    (impl $trait:ident, $method:ident, $output:ident = $ty_output:ty, $impl:ident) => {
        impl $trait<IBig> for UBig {
            type $output = $ty_output;

            #[inline]
            fn $method(self, rhs: IBig) -> $ty_output {
                let (lhs_sign, lhs_mag) = (dashu_base::Sign::Positive, self.into_repr());
                let (rhs_sign, rhs_mag) = rhs.into_sign_repr();
                $impl!(lhs_sign, lhs_mag, rhs_sign, rhs_mag)
            }
        }

        impl<'r> $trait<&'r IBig> for UBig {
            type $output = $ty_output;

            #[inline]
            fn $method(self, rhs: &IBig) -> $ty_output {
                let (lhs_sign, lhs_mag) = (dashu_base::Sign::Positive, self.into_repr());
                let (rhs_sign, rhs_mag) = rhs.as_sign_repr();
                $impl!(lhs_sign, lhs_mag, rhs_sign, rhs_mag)
            }
        }

        impl<'l> $trait<IBig> for &'l UBig {
            type $output = $ty_output;

            #[inline]
            fn $method(self, rhs: IBig) -> $ty_output {
                let (lhs_sign, lhs_mag) = (dashu_base::Sign::Positive, self.repr());
                let (rhs_sign, rhs_mag) = rhs.into_sign_repr();
                $impl!(lhs_sign, lhs_mag, rhs_sign, rhs_mag)
            }
        }

        impl<'l, 'r> $trait<&'r IBig> for &'l UBig {
            type $output = $ty_output;

            #[inline]
            fn $method(self, rhs: &IBig) -> $ty_output {
                let (lhs_sign, lhs_mag) = (dashu_base::Sign::Positive, self.repr());
                let (rhs_sign, rhs_mag) = rhs.as_sign_repr();
                $impl!(lhs_sign, lhs_mag, rhs_sign, rhs_mag)
            }
        }
    };
    (impl $trait:ident, $method:ident -> $omethod:ty, $o1:ident = $ty_o1:ty, $o2:ident = $ty_o2:ty, $impl:ident) => {
        impl $trait<IBig> for UBig {
            type $o1 = $ty_o1;
            type $o2 = $ty_o2;

            #[inline]
            fn $method(self, rhs: IBig) -> $omethod {
                let (lhs_sign, lhs_mag) = (dashu_base::Sign::Positive, self.into_repr());
                let (rhs_sign, rhs_mag) = rhs.into_sign_repr();
                $impl!(lhs_sign, lhs_mag, rhs_sign, rhs_mag)
            }
        }

        impl<'r> $trait<&'r IBig> for UBig {
            type $o1 = $ty_o1;
            type $o2 = $ty_o2;

            #[inline]
            fn $method(self, rhs: &IBig) -> $omethod {
                let (lhs_sign, lhs_mag) = (dashu_base::Sign::Positive, self.into_repr());
                let (rhs_sign, rhs_mag) = rhs.as_sign_repr();
                $impl!(lhs_sign, lhs_mag, rhs_sign, rhs_mag)
            }
        }

        impl<'l> $trait<IBig> for &'l UBig {
            type $o1 = $ty_o1;
            type $o2 = $ty_o2;

            #[inline]
            fn $method(self, rhs: IBig) -> $omethod {
                let (lhs_sign, lhs_mag) = (dashu_base::Sign::Positive, self.repr());
                let (rhs_sign, rhs_mag) = rhs.into_sign_repr();
                $impl!(lhs_sign, lhs_mag, rhs_sign, rhs_mag)
            }
        }

        impl<'l, 'r> $trait<&'r IBig> for &'l UBig {
            type $o1 = $ty_o1;
            type $o2 = $ty_o2;

            #[inline]
            fn $method(self, rhs: &IBig) -> $omethod {
                let (lhs_sign, lhs_mag) = (dashu_base::Sign::Positive, self.repr());
                let (rhs_sign, rhs_mag) = rhs.as_sign_repr();
                $impl!(lhs_sign, lhs_mag, rhs_sign, rhs_mag)
            }
        }
    };
}

/// Forward a modular-type binary op to the canonical `OpAssign<&T>` impl, where the real
/// per-repr work lives. `$target` must be a type parameterized by a single lifetime `'a`
/// (e.g. `Montgomery`, `Reduced`). Generates three universally-uniform forwarding impls:
/// - `Op<T> for T` → `self.op(&rhs)`
/// - `Op<&T> for T` → `self.op_assign(rhs); self`
/// - `OpAssign<T> for T` → `self.op_assign(&rhs)`
///
/// The caller still writes `OpAssign<&T>` (the real work), plus `Op<T> for &T` (e.g. via
/// [`impl_modular_commutative_op_for_ref!`] or a specialized impl) and `Op<&T> for &T`
/// (e.g. via [`impl_modular_binop_ref_ref_by_clone!`] or a specialized impl).
macro_rules! forward_modular_binop_to_assign {
    (impl $trait:ident, $method:ident, $trait_assign:ident, $method_assign:ident for $target:ident) => {
        impl<'a> $trait<$target<'a>> for $target<'a> {
            type Output = $target<'a>;

            #[inline]
            fn $method(self, rhs: $target<'a>) -> $target<'a> {
                self.$method(&rhs)
            }
        }

        impl<'a> $trait<&$target<'a>> for $target<'a> {
            type Output = $target<'a>;

            #[inline]
            fn $method(mut self, rhs: &$target<'a>) -> $target<'a> {
                self.$method_assign(rhs);
                self
            }
        }

        impl<'a> $trait_assign<$target<'a>> for $target<'a> {
            #[inline]
            fn $method_assign(&mut self, rhs: $target<'a>) {
                self.$method_assign(&rhs)
            }
        }
    };
}

/// For commutative modular binary ops (Add, Mul), implement `Op<T> for &T` as `rhs.op(self)`.
macro_rules! impl_modular_commutative_op_for_ref {
    (impl $trait:ident, $method:ident for $target:ident) => {
        impl<'a> $trait<$target<'a>> for &$target<'a> {
            type Output = $target<'a>;

            #[inline]
            fn $method(self, rhs: $target<'a>) -> $target<'a> {
                rhs.$method(self)
            }
        }
    };
}

/// Implement `Op<&T> for &T` as `self.clone().op(rhs)`. Skip this for ops that have a
/// specialized clone-free path for certain repr variants (e.g. Mul's Large+Large shortcut
/// that builds the product in place instead of cloning).
macro_rules! impl_modular_binop_ref_ref_by_clone {
    (impl $trait:ident, $method:ident for $target:ident) => {
        impl<'a> $trait<&$target<'a>> for &$target<'a> {
            type Output = $target<'a>;

            #[inline]
            fn $method(self, rhs: &$target<'a>) -> $target<'a> {
                self.clone().$method(rhs)
            }
        }
    };
}

/// Forward a modular-type binary op to the canonical `Op<&T> for &T` impl, where the real
/// work lives (used for Div, whose impl needs a fresh output rather than an in-place
/// mutation). `$target` must be a type parameterized by a single lifetime `'a`. Generates
/// five forwarding impls:
/// - `Op<T> for T` → `(&self).op(&rhs)`
/// - `Op<&T> for T` → `(&self).op(rhs)`
/// - `Op<T> for &T` → `self.op(&rhs)`
/// - `OpAssign<T>` → `self.op_assign(&rhs)`
/// - `OpAssign<&T>` → `*self = (&*self).op(rhs)`
///
/// The caller still writes `Op<&T> for &T` (the real work).
macro_rules! forward_modular_binop_to_ref_ref {
    (impl $trait:ident, $method:ident, $trait_assign:ident, $method_assign:ident for $target:ident) => {
        impl<'a> $trait<$target<'a>> for $target<'a> {
            type Output = $target<'a>;

            #[inline]
            fn $method(self, rhs: $target<'a>) -> $target<'a> {
                (&self).$method(&rhs)
            }
        }

        impl<'a> $trait<&$target<'a>> for $target<'a> {
            type Output = $target<'a>;

            #[inline]
            fn $method(self, rhs: &$target<'a>) -> $target<'a> {
                (&self).$method(rhs)
            }
        }

        impl<'a> $trait<$target<'a>> for &$target<'a> {
            type Output = $target<'a>;

            #[inline]
            fn $method(self, rhs: $target<'a>) -> $target<'a> {
                self.$method(&rhs)
            }
        }

        impl<'a> $trait_assign<$target<'a>> for $target<'a> {
            #[inline]
            fn $method_assign(&mut self, rhs: $target<'a>) {
                self.$method_assign(&rhs)
            }
        }

        impl<'a> $trait_assign<&$target<'a>> for $target<'a> {
            #[inline]
            fn $method_assign(&mut self, rhs: &$target<'a>) {
                *self = (&*self).$method(rhs);
            }
        }
    };
}

/// Implement `impl Op<UBig> for IBig` by forwarding to the macro `$impl` with arguments
/// `(self_sign, self_repr, rhs_repr)`
macro_rules! forward_ibig_ubig_binop_to_repr {
    (impl $trait:ident, $method:ident, $output:ident = $ty_output:ty, $impl:ident) => {
        impl $trait<UBig> for IBig {
            type $output = $ty_output;

            #[inline]
            fn $method(self, rhs: UBig) -> $ty_output {
                let (lhs_sign, lhs_mag) = self.into_sign_repr();
                let (rhs_sign, rhs_mag) = (dashu_base::Sign::Positive, rhs.into_repr());
                $impl!(lhs_sign, lhs_mag, rhs_sign, rhs_mag)
            }
        }

        impl<'r> $trait<&'r UBig> for IBig {
            type $output = $ty_output;

            #[inline]
            fn $method(self, rhs: &UBig) -> $ty_output {
                let (lhs_sign, lhs_mag) = self.into_sign_repr();
                let (rhs_sign, rhs_mag) = (dashu_base::Sign::Positive, rhs.repr());
                $impl!(lhs_sign, lhs_mag, rhs_sign, rhs_mag)
            }
        }

        impl<'l> $trait<UBig> for &'l IBig {
            type $output = $ty_output;

            #[inline]
            fn $method(self, rhs: UBig) -> $ty_output {
                let (lhs_sign, lhs_mag) = self.as_sign_repr();
                let (rhs_sign, rhs_mag) = (dashu_base::Sign::Positive, rhs.into_repr());
                $impl!(lhs_sign, lhs_mag, rhs_sign, rhs_mag)
            }
        }

        impl<'l, 'r> $trait<&'r UBig> for &'l IBig {
            type $output = $ty_output;

            #[inline]
            fn $method(self, rhs: &UBig) -> $ty_output {
                let (lhs_sign, lhs_mag) = self.as_sign_repr();
                let (rhs_sign, rhs_mag) = (dashu_base::Sign::Positive, rhs.repr());
                $impl!(lhs_sign, lhs_mag, rhs_sign, rhs_mag)
            }
        }
    };
    (impl $trait:ident, $method:ident -> $omethod:ty, $o1:ident = $ty_o1:ty, $o2:ident = $ty_o2:ty, $impl:ident) => {
        impl $trait<UBig> for IBig {
            type $o1 = $ty_o1;
            type $o2 = $ty_o2;

            #[inline]
            fn $method(self, rhs: UBig) -> $omethod {
                let (lhs_sign, lhs_mag) = self.into_sign_repr();
                let (rhs_sign, rhs_mag) = (dashu_base::Sign::Positive, rhs.into_repr());
                $impl!(lhs_sign, lhs_mag, rhs_sign, rhs_mag)
            }
        }

        impl<'r> $trait<&'r UBig> for IBig {
            type $o1 = $ty_o1;
            type $o2 = $ty_o2;

            #[inline]
            fn $method(self, rhs: &UBig) -> $omethod {
                let (lhs_sign, lhs_mag) = self.into_sign_repr();
                let (rhs_sign, rhs_mag) = (dashu_base::Sign::Positive, rhs.repr());
                $impl!(lhs_sign, lhs_mag, rhs_sign, rhs_mag)
            }
        }

        impl<'l> $trait<UBig> for &'l IBig {
            type $o1 = $ty_o1;
            type $o2 = $ty_o2;

            #[inline]
            fn $method(self, rhs: UBig) -> $omethod {
                let (lhs_sign, lhs_mag) = self.as_sign_repr();
                let (rhs_sign, rhs_mag) = (dashu_base::Sign::Positive, rhs.into_repr());
                $impl!(lhs_sign, lhs_mag, rhs_sign, rhs_mag)
            }
        }

        impl<'l, 'r> $trait<&'r UBig> for &'l IBig {
            type $o1 = $ty_o1;
            type $o2 = $ty_o2;

            #[inline]
            fn $method(self, rhs: &UBig) -> $omethod {
                let (lhs_sign, lhs_mag) = self.as_sign_repr();
                let (rhs_sign, rhs_mag) = (dashu_base::Sign::Positive, rhs.repr());
                $impl!(lhs_sign, lhs_mag, rhs_sign, rhs_mag)
            }
        }
    };
}

pub(crate) use debug_assert_zero;
pub(crate) use forward_ibig_binop_to_repr;
pub(crate) use forward_ibig_ubig_binop_to_repr;
pub(crate) use forward_modular_binop_to_assign;
pub(crate) use forward_modular_binop_to_ref_ref;
pub(crate) use forward_ubig_binop_to_repr;
pub(crate) use forward_ubig_ibig_binop_to_repr;
pub(crate) use impl_binop_assign_by_taking;
pub(crate) use impl_binop_assign_with_primitive;
pub(crate) use impl_binop_with_primitive;
pub(crate) use impl_commutative_binop_with_primitive;
pub(crate) use impl_modular_binop_ref_ref_by_clone;
pub(crate) use impl_modular_commutative_op_for_ref;
