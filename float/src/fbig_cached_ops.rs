//! Operators for [`CachedFBig`] — all binary/unary operators with cache preservation.

use core::ops::{
    Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Rem, RemAssign, Sub, SubAssign,
};

use dashu_base::Abs;

use crate::fbig::FBig;
use crate::fbig_cached::CachedFBig;
use crate::repr::{Context, Word};
use crate::round::Round;

// ---------------------------------------------------------------------------
// CachedFBig op CachedFBig (preserves LHS cache)
// ---------------------------------------------------------------------------

macro_rules! impl_cached_binop {
    ($Op:ident, $op:ident) => {
        impl<R: Round, const B: Word> $Op<CachedFBig<R, B>> for CachedFBig<R, B> {
            type Output = CachedFBig<R, B>;
            #[inline]
            fn $op(self, rhs: CachedFBig<R, B>) -> Self::Output {
                CachedFBig::from_fbig($Op::$op(self.fbig, rhs.fbig), &self.cache)
            }
        }
    };
}
impl_cached_binop!(Add, add);
impl_cached_binop!(Sub, sub);
impl_cached_binop!(Mul, mul);
impl_cached_binop!(Div, div);
impl_cached_binop!(Rem, rem);

macro_rules! impl_cached_binop_assign {
    ($OpAssign:ident, $op_assign:ident, $Op:ident, $op:ident) => {
        impl<R: Round, const B: Word> $OpAssign<CachedFBig<R, B>> for CachedFBig<R, B> {
            #[inline]
            fn $op_assign(&mut self, rhs: CachedFBig<R, B>) {
                let res = $Op::$op(self.fbig.clone(), rhs.fbig);
                self.fbig = res;
            }
        }
    };
}
impl_cached_binop_assign!(AddAssign, add_assign, Add, add);
impl_cached_binop_assign!(SubAssign, sub_assign, Sub, sub);
impl_cached_binop_assign!(MulAssign, mul_assign, Mul, mul);
impl_cached_binop_assign!(DivAssign, div_assign, Div, div);
impl_cached_binop_assign!(RemAssign, rem_assign, Rem, rem);

// ---------------------------------------------------------------------------
// CachedFBig op FBig and FBig op CachedFBig (cache preserved from CachedFBig side)
// ---------------------------------------------------------------------------

macro_rules! impl_cached_binop_one_way_with_fbig {
    ($Op:ident, $op:ident) => {
        impl<R: Round, const B: Word> $Op<FBig<R, B>> for CachedFBig<R, B> {
            type Output = CachedFBig<R, B>;
            #[inline]
            fn $op(self, rhs: FBig<R, B>) -> Self::Output {
                CachedFBig::from_fbig($Op::$op(self.fbig, rhs), &self.cache)
            }
        }
        impl<'l, R: Round, const B: Word> $Op<FBig<R, B>> for &'l CachedFBig<R, B> {
            type Output = CachedFBig<R, B>;
            #[inline]
            fn $op(self, rhs: FBig<R, B>) -> Self::Output {
                CachedFBig::from_fbig($Op::$op(self.fbig.clone(), rhs), &self.cache)
            }
        }
        impl<'r, R: Round, const B: Word> $Op<&'r FBig<R, B>> for CachedFBig<R, B> {
            type Output = CachedFBig<R, B>;
            #[inline]
            fn $op(self, rhs: &FBig<R, B>) -> Self::Output {
                CachedFBig::from_fbig($Op::$op(self.fbig, rhs.clone()), &self.cache)
            }
        }
        impl<'l, 'r, R: Round, const B: Word> $Op<&'r FBig<R, B>> for &'l CachedFBig<R, B> {
            type Output = CachedFBig<R, B>;
            #[inline]
            fn $op(self, rhs: &FBig<R, B>) -> Self::Output {
                CachedFBig::from_fbig($Op::$op(self.fbig.clone(), rhs.clone()), &self.cache)
            }
        }
    };
}

macro_rules! impl_cached_binop_reverse_with_fbig {
    ($Op:ident, $op:ident) => {
        impl<R: Round, const B: Word> $Op<CachedFBig<R, B>> for FBig<R, B> {
            type Output = CachedFBig<R, B>;
            #[inline]
            fn $op(self, rhs: CachedFBig<R, B>) -> Self::Output {
                CachedFBig::from_fbig($Op::$op(self, rhs.fbig), &rhs.cache)
            }
        }
        impl<'l, R: Round, const B: Word> $Op<CachedFBig<R, B>> for &'l FBig<R, B> {
            type Output = CachedFBig<R, B>;
            #[inline]
            fn $op(self, rhs: CachedFBig<R, B>) -> Self::Output {
                CachedFBig::from_fbig($Op::$op(self.clone(), rhs.fbig), &rhs.cache)
            }
        }
        impl<'r, R: Round, const B: Word> $Op<&'r CachedFBig<R, B>> for FBig<R, B> {
            type Output = CachedFBig<R, B>;
            #[inline]
            fn $op(self, rhs: &CachedFBig<R, B>) -> Self::Output {
                CachedFBig::from_fbig($Op::$op(self, rhs.fbig.clone()), &rhs.cache)
            }
        }
        impl<'l, 'r, R: Round, const B: Word> $Op<&'r CachedFBig<R, B>> for &'l FBig<R, B> {
            type Output = CachedFBig<R, B>;
            #[inline]
            fn $op(self, rhs: &CachedFBig<R, B>) -> Self::Output {
                CachedFBig::from_fbig($Op::$op(self.clone(), rhs.fbig.clone()), &rhs.cache)
            }
        }
    };
}

impl_cached_binop_one_way_with_fbig!(Add, add);
impl_cached_binop_one_way_with_fbig!(Sub, sub);
impl_cached_binop_one_way_with_fbig!(Mul, mul);
impl_cached_binop_one_way_with_fbig!(Div, div);
impl_cached_binop_one_way_with_fbig!(Rem, rem);

impl_cached_binop_reverse_with_fbig!(Add, add);
impl_cached_binop_reverse_with_fbig!(Sub, sub);
impl_cached_binop_reverse_with_fbig!(Mul, mul);
impl_cached_binop_reverse_with_fbig!(Div, div);
impl_cached_binop_reverse_with_fbig!(Rem, rem);

// assign: CachedFBig op= FBig

macro_rules! impl_cached_binop_assign_with_fbig {
    ($OpAssign:ident, $op_assign:ident, $Op:ident, $op:ident) => {
        impl<R: Round, const B: Word> $OpAssign<FBig<R, B>> for CachedFBig<R, B> {
            #[inline]
            fn $op_assign(&mut self, rhs: FBig<R, B>) {
                self.fbig = $Op::$op(self.fbig.clone(), rhs);
            }
        }
        impl<R: Round, const B: Word> $OpAssign<&FBig<R, B>> for CachedFBig<R, B> {
            #[inline]
            fn $op_assign(&mut self, rhs: &FBig<R, B>) {
                self.fbig = $Op::$op(self.fbig.clone(), rhs.clone());
            }
        }
    };
}

impl_cached_binop_assign_with_fbig!(AddAssign, add_assign, Add, add);
impl_cached_binop_assign_with_fbig!(SubAssign, sub_assign, Sub, sub);
impl_cached_binop_assign_with_fbig!(MulAssign, mul_assign, Mul, mul);
impl_cached_binop_assign_with_fbig!(DivAssign, div_assign, Div, div);
impl_cached_binop_assign_with_fbig!(RemAssign, rem_assign, Rem, rem);

// ---------------------------------------------------------------------------
// CachedFBig op Primitive and Primitive op CachedFBig
// (delegate through the FBig-side impls above)
// ---------------------------------------------------------------------------

macro_rules! impl_cached_binop_one_way_with_primitive {
    ($Op:ident, $op:ident, $target:ty) => {
        impl<R: Round, const B: Word> $Op<$target> for CachedFBig<R, B> {
            type Output = CachedFBig<R, B>;
            #[inline]
            fn $op(self, rhs: $target) -> Self::Output {
                self.$op(FBig::<R, B>::from(rhs))
            }
        }
        impl<'l, R: Round, const B: Word> $Op<$target> for &'l CachedFBig<R, B> {
            type Output = CachedFBig<R, B>;
            #[inline]
            fn $op(self, rhs: $target) -> Self::Output {
                self.$op(FBig::<R, B>::from(rhs))
            }
        }
        impl<'r, R: Round, const B: Word> $Op<&'r $target> for CachedFBig<R, B> {
            type Output = CachedFBig<R, B>;
            #[inline]
            fn $op(self, rhs: &$target) -> Self::Output {
                self.$op(FBig::<R, B>::from(rhs.clone()))
            }
        }
        impl<'l, 'r, R: Round, const B: Word> $Op<&'r $target> for &'l CachedFBig<R, B> {
            type Output = CachedFBig<R, B>;
            #[inline]
            fn $op(self, rhs: &$target) -> Self::Output {
                self.$op(FBig::<R, B>::from(rhs.clone()))
            }
        }
    };
}

macro_rules! impl_cached_binop_reverse_with_primitive {
    ($Op:ident, $op:ident, $target:ty) => {
        impl<R: Round, const B: Word> $Op<CachedFBig<R, B>> for $target {
            type Output = CachedFBig<R, B>;
            #[inline]
            fn $op(self, rhs: CachedFBig<R, B>) -> Self::Output {
                FBig::<R, B>::from(self).$op(rhs)
            }
        }
        impl<'l, R: Round, const B: Word> $Op<CachedFBig<R, B>> for &'l $target {
            type Output = CachedFBig<R, B>;
            #[inline]
            fn $op(self, rhs: CachedFBig<R, B>) -> Self::Output {
                FBig::<R, B>::from(self.clone()).$op(rhs)
            }
        }
        impl<'r, R: Round, const B: Word> $Op<&'r CachedFBig<R, B>> for $target {
            type Output = CachedFBig<R, B>;
            #[inline]
            fn $op(self, rhs: &CachedFBig<R, B>) -> Self::Output {
                FBig::<R, B>::from(self).$op(rhs)
            }
        }
        impl<'l, 'r, R: Round, const B: Word> $Op<&'r CachedFBig<R, B>> for &'l $target {
            type Output = CachedFBig<R, B>;
            #[inline]
            fn $op(self, rhs: &CachedFBig<R, B>) -> Self::Output {
                FBig::<R, B>::from(self.clone()).$op(rhs)
            }
        }
    };
}

macro_rules! impl_cached_binop_assign_with_primitive {
    ($OpAssign:ident, $op_assign:ident, $Op:ident, $op:ident, $target:ty) => {
        impl<R: Round, const B: Word> $OpAssign<$target> for CachedFBig<R, B> {
            #[inline]
            fn $op_assign(&mut self, rhs: $target) {
                self.$op_assign(FBig::<R, B>::from(rhs));
            }
        }
        impl<R: Round, const B: Word> $OpAssign<&$target> for CachedFBig<R, B> {
            #[inline]
            fn $op_assign(&mut self, rhs: &$target) {
                self.$op_assign(FBig::<R, B>::from(rhs.clone()));
            }
        }
    };
}

macro_rules! impl_cached_binop_with_primitives {
    ($Op:ident, $op:ident $(, $t:ty)*) => {
        $(
            impl_cached_binop_one_way_with_primitive!($Op, $op, $t);
            impl_cached_binop_reverse_with_primitive!($Op, $op, $t);
        )*
    };
}

macro_rules! impl_cached_binop_assign_with_primitives {
    ($OpAssign:ident, $op_assign:ident, $Op:ident, $op:ident $(, $t:ty)*) => {
        $(
            impl_cached_binop_assign_with_primitive!($OpAssign, $op_assign, $Op, $op, $t);
        )*
    };
}

// Unsigned
impl_cached_binop_with_primitives!(Add, add, u8, u16, u32, u64, u128, usize, dashu_int::UBig);
impl_cached_binop_with_primitives!(Sub, sub, u8, u16, u32, u64, u128, usize, dashu_int::UBig);
impl_cached_binop_with_primitives!(Mul, mul, u8, u16, u32, u64, u128, usize, dashu_int::UBig);
impl_cached_binop_with_primitives!(Div, div, u8, u16, u32, u64, u128, usize, dashu_int::UBig);
impl_cached_binop_with_primitives!(Rem, rem, u8, u16, u32, u64, u128, usize, dashu_int::UBig);

// Signed
impl_cached_binop_with_primitives!(Add, add, i8, i16, i32, i64, i128, isize, dashu_int::IBig);
impl_cached_binop_with_primitives!(Sub, sub, i8, i16, i32, i64, i128, isize, dashu_int::IBig);
impl_cached_binop_with_primitives!(Mul, mul, i8, i16, i32, i64, i128, isize, dashu_int::IBig);
impl_cached_binop_with_primitives!(Div, div, i8, i16, i32, i64, i128, isize, dashu_int::IBig);
impl_cached_binop_with_primitives!(Rem, rem, i8, i16, i32, i64, i128, isize, dashu_int::IBig);

// Assign variants
#[rustfmt::skip]
impl_cached_binop_assign_with_primitives!(AddAssign, add_assign, Add, add,
    u8, u16, u32, u64, u128, usize, dashu_int::UBig,
    i8, i16, i32, i64, i128, isize, dashu_int::IBig);
#[rustfmt::skip]
impl_cached_binop_assign_with_primitives!(SubAssign, sub_assign, Sub, sub,
    u8, u16, u32, u64, u128, usize, dashu_int::UBig,
    i8, i16, i32, i64, i128, isize, dashu_int::IBig);
#[rustfmt::skip]
impl_cached_binop_assign_with_primitives!(MulAssign, mul_assign, Mul, mul,
    u8, u16, u32, u64, u128, usize, dashu_int::UBig,
    i8, i16, i32, i64, i128, isize, dashu_int::IBig);
#[rustfmt::skip]
impl_cached_binop_assign_with_primitives!(DivAssign, div_assign, Div, div,
    u8, u16, u32, u64, u128, usize, dashu_int::UBig,
    i8, i16, i32, i64, i128, isize, dashu_int::IBig);
#[rustfmt::skip]
impl_cached_binop_assign_with_primitives!(RemAssign, rem_assign, Rem, rem,
    u8, u16, u32, u64, u128, usize, dashu_int::UBig,
    i8, i16, i32, i64, i128, isize, dashu_int::IBig);

// ---------------------------------------------------------------------------
// Unary operators
// ---------------------------------------------------------------------------

impl<R: Round, const B: Word> Neg for CachedFBig<R, B> {
    type Output = CachedFBig<R, B>;
    #[inline]
    fn neg(self) -> Self::Output {
        CachedFBig::from_fbig(-self.fbig, &self.cache)
    }
}

impl<R: Round, const B: Word> Abs for CachedFBig<R, B> {
    type Output = CachedFBig<R, B>;
    #[inline]
    fn abs(self) -> Self::Output {
        CachedFBig::from_fbig(Abs::abs(self.fbig), &self.cache)
    }
}

// ---------------------------------------------------------------------------
// Math functions (forward to Context / FBig, preserve cache handle)
// ---------------------------------------------------------------------------

/// Forward a unary function to a [`Context`] method returning `FpResult<FBig>`, panicking on
/// error and re-attaching the cache handle. Returns a bare `CachedFBig`.
macro_rules! forward_to_context {
    ($name:ident) => {
        #[doc = concat!("See [`FBig::", stringify!($name), "`].")]
        #[inline]
        pub fn $name(&self) -> CachedFBig<R, B> {
            let mut c = self.cache.borrow_mut();
            let fbig = self
                .fbig
                .context
                .unwrap_fp(self.fbig.context.$name::<B>(&self.fbig.repr, Some(&mut *c)));
            CachedFBig::from_fbig(fbig, &self.cache)
        }
    };
}

/// Forward a unary function to a [`Context`] method returning `FpResult<FBig>`, panicking on
/// error and discarding the rounding info. Returns a bare `CachedFBig`.
macro_rules! forward_to_context_unwrap {
    ($name:ident) => {
        #[doc = concat!("See [`FBig::", stringify!($name), "`].")]
        #[inline]
        pub fn $name(&self) -> CachedFBig<R, B> {
            let mut c = self.cache.borrow_mut();
            let fbig = self
                .fbig
                .context
                .unwrap_fp(self.fbig.context.$name::<B>(&self.fbig.repr, Some(&mut *c)));
            CachedFBig::from_fbig(fbig, &self.cache)
        }
    };
}

/// Forward a unary function that delegates to the inner [`FBig`] (no cache needed).
macro_rules! forward_to_fbig {
    ($name:ident) => {
        #[doc = concat!("See [`FBig::", stringify!($name), "`].")]
        #[inline]
        pub fn $name(&self) -> CachedFBig<R, B> {
            CachedFBig::from_fbig(self.fbig.clone().$name(), &self.cache)
        }
    };
    ($name:ident($arg:ident: $arg_ty:ty)) => {
        #[doc = concat!("See [`FBig::", stringify!($name), "`].")]
        #[inline]
        pub fn $name(&self, $arg: $arg_ty) -> CachedFBig<R, B> {
            CachedFBig::from_fbig(self.fbig.clone().$name($arg), &self.cache)
        }
    };
}

impl<R: Round, const B: Word> CachedFBig<R, B> {
    forward_to_context!(ln);
    forward_to_context!(ln_1p);
    forward_to_context!(exp);
    forward_to_context!(exp_m1);

    forward_to_fbig!(sqrt);
    forward_to_fbig!(inv);

    forward_to_context_unwrap!(sin);
    forward_to_context_unwrap!(cos);
    forward_to_context_unwrap!(tan);
    forward_to_context_unwrap!(asin);
    forward_to_context_unwrap!(acos);
    forward_to_context_unwrap!(atan);

    forward_to_fbig!(powi(exp: dashu_int::IBig));
    forward_to_fbig!(sqr);
    forward_to_fbig!(cubic);

    /// `self^exp` (see [`FBig::powf`]).
    pub fn powf(&self, exp: &Self) -> Self {
        let context = Context::max(self.fbig.context, exp.fbig.context);
        let mut c = self.cache.borrow_mut();
        let fbig =
            context.unwrap_fp(context.powf::<B>(&self.fbig.repr, &exp.fbig.repr, Some(&mut *c)));
        Self::from_fbig(fbig, &self.cache)
    }

    /// Sine and cosine together (see [`FBig::sin_cos`]).
    pub fn sin_cos(&self) -> (Self, Self) {
        let mut guard = self.cache.borrow_mut();
        let cache = Some(&mut *guard);
        let (s, c) = self.fbig.context.sin_cos::<B>(&self.fbig.repr, cache);
        (
            Self::from_fbig(self.fbig.context.unwrap_fp(s), &self.cache),
            Self::from_fbig(self.fbig.context.unwrap_fp(c), &self.cache),
        )
    }

    /// `atan2(y, x)` (see [`FBig::atan2`]).
    pub fn atan2(&self, x: &Self) -> Self {
        let mut c = self.cache.borrow_mut();
        let fbig = self.fbig.context.unwrap_fp(self.fbig.context.atan2::<B>(
            &self.fbig.repr,
            &x.fbig.repr,
            Some(&mut *c),
        ));
        Self::from_fbig(fbig, &self.cache)
    }
}
