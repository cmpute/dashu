use crate::{
    error::{assert_finite_operands, assert_limited_precision, FpError, FpResult},
    fbig::FBig,
    helper_macros::{self, impl_binop_assign_by_taking},
    repr::{Context, Repr, Word},
    round::{Round, Rounded, Rounding},
    utils::{digit_len, digits_bounds_from_bits, shl_digits, shl_digits_in_place, split_digits},
};
use core::cmp::Ordering;
use core::ops::{Div, DivAssign, Rem, RemAssign};
use dashu_base::{
    AbsOrd, Approximation, BitTest, DivEuclid, DivRem, DivRemEuclid, Inverse, RemEuclid, Sign,
    Signed,
};
use dashu_int::{fast_div::ConstDivisor, modular::IntoRing, IBig, UBig};

/// Attach the dividend/divisor XOR sign to a zero quotient: the raw quotient significand is
/// `+0`, so the sign of a zero result (`0/finite`, or a finite/finite that rounds to zero) is
/// `sign(lhs) XOR sign(rhs)`.
fn make_div_repr<const B: Word>(
    sign_negative: bool,
    significand: IBig,
    exponent: isize,
) -> Repr<B> {
    if significand.is_zero() {
        if sign_negative {
            Repr::neg_zero()
        } else {
            Repr::zero()
        }
    } else {
        Repr::new(significand, exponent)
    }
}

macro_rules! impl_div_for_fbig {
    (impl $op:ident, $method:ident, $repr_method:ident) => {
        impl<R: Round, const B: Word> $op<FBig<R, B>> for FBig<R, B> {
            type Output = FBig<R, B>;
            fn $method(self, rhs: FBig<R, B>) -> Self::Output {
                let context = Context::max(self.context, rhs.context);
                // Route through `unwrap_fp` (not the Repr-level unwrap) so an exponent
                // overflow/underflow saturates to the directed endpoint, mode-aware.
                let result = context
                    .$repr_method(self.repr, rhs.repr)
                    .map(|r| r.map(|repr| FBig::new(repr, context)));
                context.unwrap_fp(result)
            }
        }

        impl<'l, R: Round, const B: Word> $op<FBig<R, B>> for &'l FBig<R, B> {
            type Output = FBig<R, B>;
            fn $method(self, rhs: FBig<R, B>) -> Self::Output {
                let context = Context::max(self.context, rhs.context);
                let result = context
                    .$repr_method(self.repr.clone(), rhs.repr)
                    .map(|r| r.map(|repr| FBig::new(repr, context)));
                context.unwrap_fp(result)
            }
        }

        impl<'r, R: Round, const B: Word> $op<&'r FBig<R, B>> for FBig<R, B> {
            type Output = FBig<R, B>;
            fn $method(self, rhs: &FBig<R, B>) -> Self::Output {
                let context = Context::max(self.context, rhs.context);
                let result = context
                    .$repr_method(self.repr, rhs.repr.clone())
                    .map(|r| r.map(|repr| FBig::new(repr, context)));
                context.unwrap_fp(result)
            }
        }

        impl<'l, 'r, R: Round, const B: Word> $op<&'r FBig<R, B>> for &'l FBig<R, B> {
            type Output = FBig<R, B>;
            fn $method(self, rhs: &FBig<R, B>) -> Self::Output {
                let context = Context::max(self.context, rhs.context);
                let result = context
                    .$repr_method(self.repr.clone(), rhs.repr.clone())
                    .map(|r| r.map(|repr| FBig::new(repr, context)));
                context.unwrap_fp(result)
            }
        }
    };
}

macro_rules! impl_rem_for_fbig {
    (impl $op:ident, $method:ident, $repr_method:ident) => {
        impl<R: Round, const B: Word> $op<FBig<R, B>> for FBig<R, B> {
            type Output = FBig<R, B>;
            fn $method(self, rhs: FBig<R, B>) -> Self::Output {
                let context = Context::max(self.context, rhs.context);
                FBig::new(context.$repr_method(self.repr, rhs.repr).value(), context)
            }
        }

        impl<'l, R: Round, const B: Word> $op<FBig<R, B>> for &'l FBig<R, B> {
            type Output = FBig<R, B>;
            fn $method(self, rhs: FBig<R, B>) -> Self::Output {
                let context = Context::max(self.context, rhs.context);
                FBig::new(context.$repr_method(self.repr.clone(), rhs.repr).value(), context)
            }
        }

        impl<'r, R: Round, const B: Word> $op<&'r FBig<R, B>> for FBig<R, B> {
            type Output = FBig<R, B>;
            fn $method(self, rhs: &FBig<R, B>) -> Self::Output {
                let context = Context::max(self.context, rhs.context);
                FBig::new(context.$repr_method(self.repr, rhs.repr.clone()).value(), context)
            }
        }

        impl<'l, 'r, R: Round, const B: Word> $op<&'r FBig<R, B>> for &'l FBig<R, B> {
            type Output = FBig<R, B>;
            fn $method(self, rhs: &FBig<R, B>) -> Self::Output {
                let context = Context::max(self.context, rhs.context);
                FBig::new(
                    context
                        .$repr_method(self.repr.clone(), rhs.repr.clone())
                        .value(),
                    context,
                )
            }
        }
    };
}
impl_div_for_fbig!(impl Div, div, repr_div);
impl_rem_for_fbig!(impl Rem, rem, repr_rem);
impl_binop_assign_by_taking!(impl DivAssign<Self>, div_assign, div);
impl_binop_assign_by_taking!(impl RemAssign<Self>, rem_assign, rem);

impl<R: Round, const B: Word> DivEuclid<FBig<R, B>> for FBig<R, B> {
    type Output = IBig;
    #[inline]
    fn div_euclid(self, rhs: FBig<R, B>) -> Self::Output {
        let (num, den) = align_as_int(self, rhs);
        num.div_euclid(den)
    }
}

impl<R: Round, const B: Word> DivEuclid<FBig<R, B>> for &FBig<R, B> {
    type Output = IBig;
    #[inline]
    fn div_euclid(self, rhs: FBig<R, B>) -> Self::Output {
        self.clone().div_euclid(rhs)
    }
}

impl<R: Round, const B: Word> DivEuclid<&FBig<R, B>> for FBig<R, B> {
    type Output = IBig;
    #[inline]
    fn div_euclid(self, rhs: &FBig<R, B>) -> Self::Output {
        self.div_euclid(rhs.clone())
    }
}

impl<R: Round, const B: Word> DivEuclid<&FBig<R, B>> for &FBig<R, B> {
    type Output = IBig;
    #[inline]
    fn div_euclid(self, rhs: &FBig<R, B>) -> Self::Output {
        self.clone().div_euclid(rhs.clone())
    }
}

impl<R: Round, const B: Word> RemEuclid<FBig<R, B>> for FBig<R, B> {
    type Output = FBig<R, B>;
    #[inline]
    fn rem_euclid(self, rhs: FBig<R, B>) -> Self::Output {
        let r_exponent = self.repr.exponent.min(rhs.repr.exponent);
        let context = Context::max(self.context, rhs.context);

        let (num, den) = align_as_int(self, rhs);
        let r = num.rem_euclid(den);
        let mut r = context.convert_int(r.into()).value();
        if !r.repr.significand.is_zero() {
            r.repr.exponent += r_exponent;
        }
        r
    }
}

impl<R: Round, const B: Word> RemEuclid<FBig<R, B>> for &FBig<R, B> {
    type Output = FBig<R, B>;
    #[inline]
    fn rem_euclid(self, rhs: FBig<R, B>) -> Self::Output {
        self.clone().rem_euclid(rhs)
    }
}

impl<R: Round, const B: Word> RemEuclid<&FBig<R, B>> for FBig<R, B> {
    type Output = FBig<R, B>;
    #[inline]
    fn rem_euclid(self, rhs: &FBig<R, B>) -> Self::Output {
        self.rem_euclid(rhs.clone())
    }
}

impl<R: Round, const B: Word> RemEuclid<&FBig<R, B>> for &FBig<R, B> {
    type Output = FBig<R, B>;
    #[inline]
    fn rem_euclid(self, rhs: &FBig<R, B>) -> Self::Output {
        self.clone().rem_euclid(rhs.clone())
    }
}

impl<R: Round, const B: Word> DivRemEuclid<FBig<R, B>> for FBig<R, B> {
    type OutputDiv = IBig;
    type OutputRem = FBig<R, B>;
    #[inline]
    fn div_rem_euclid(self, rhs: FBig<R, B>) -> (IBig, FBig<R, B>) {
        let r_exponent = self.repr.exponent.min(rhs.repr.exponent);
        let context = Context::max(self.context, rhs.context);

        let (num, den) = align_as_int(self, rhs);
        let (q, r) = num.div_rem_euclid(den);
        let mut r = context.convert_int(r.into()).value();
        if !r.repr.significand.is_zero() {
            r.repr.exponent += r_exponent;
        }
        (q, r)
    }
}

impl<R: Round, const B: Word> DivRemEuclid<FBig<R, B>> for &FBig<R, B> {
    type OutputDiv = IBig;
    type OutputRem = FBig<R, B>;
    #[inline]
    fn div_rem_euclid(self, rhs: FBig<R, B>) -> (IBig, FBig<R, B>) {
        self.clone().div_rem_euclid(rhs)
    }
}

impl<R: Round, const B: Word> DivRemEuclid<&FBig<R, B>> for FBig<R, B> {
    type OutputDiv = IBig;
    type OutputRem = FBig<R, B>;
    #[inline]
    fn div_rem_euclid(self, rhs: &FBig<R, B>) -> (IBig, FBig<R, B>) {
        self.div_rem_euclid(rhs.clone())
    }
}

impl<R: Round, const B: Word> DivRemEuclid<&FBig<R, B>> for &FBig<R, B> {
    type OutputDiv = IBig;
    type OutputRem = FBig<R, B>;
    #[inline]
    fn div_rem_euclid(self, rhs: &FBig<R, B>) -> (IBig, FBig<R, B>) {
        self.clone().div_rem_euclid(rhs.clone())
    }
}

macro_rules! impl_div_primitive_with_fbig {
    ($($t:ty)*) => {$(
        helper_macros::impl_binop_with_primitive!(impl Div<$t>, div);
        helper_macros::impl_binop_assign_with_primitive!(impl DivAssign<$t>, div_assign);
    )*};
}
impl_div_primitive_with_fbig!(u8 u16 u32 u64 u128 usize UBig i8 i16 i32 i64 i128 isize IBig);
// TODO: we should specialize FBig / UBig or FBig / IBig for better efficiency

impl<R: Round, const B: Word> Inverse for FBig<R, B> {
    type Output = FBig<R, B>;

    #[inline]
    fn inv(self) -> Self::Output {
        self.context.unwrap_fp(self.context.inv(&self.repr))
    }
}

impl<R: Round, const B: Word> Inverse for &FBig<R, B> {
    type Output = FBig<R, B>;

    #[inline]
    fn inv(self) -> Self::Output {
        self.context.unwrap_fp(self.context.inv(&self.repr))
    }
}

impl<R: Round, const B: Word> FBig<R, B> {
    /// Calculate the multiplicative inverse (`1 / self`) of the floating point number.
    ///
    /// # Panics
    ///
    /// Panics if the precision is unlimited.
    #[inline]
    pub fn inv(&self) -> Self {
        self.context.unwrap_fp(self.context.inv(&self.repr))
    }
}

// Align two float by exponent such that they are both turned into integers
fn align_as_int<R: Round, const B: Word>(lhs: FBig<R, B>, rhs: FBig<R, B>) -> (IBig, IBig) {
    let ediff = lhs.repr.exponent - rhs.repr.exponent;
    let (mut num, mut den) = (lhs.repr.significand, rhs.repr.significand);
    if ediff >= 0 {
        shl_digits_in_place::<B>(&mut num, ediff as _);
    } else {
        shl_digits_in_place::<B>(&mut den, (-ediff) as _);
    }
    (num, den)
}

impl<R: Round> Context<R> {
    /// Division kernel for an already-bounded dividend: `lhs` must carry at most
    /// `rhs.digits() + precision` digits (the `FBig` operators' operands always do — a
    /// significand is at most `precision + 1` digits and the divisor at least one). For an
    /// arbitrary [`Repr`] dividend use [`Context::div`], which bounds it exactly (see
    /// [`Self::repr_div_split`]).
    pub(crate) fn repr_div<const B: Word>(&self, lhs: Repr<B>, rhs: Repr<B>) -> FpResult<Repr<B>> {
        self.repr_div_split(lhs, rhs, IBig::ZERO, 0)
    }

    /// [`Self::repr_div`] for an arbitrary-width dividend: the excess low digits below the
    /// `divisor digits + precision` bound are split off as `lo` and carried through the
    /// division as sticky rounding information, so no information is ever lost. (Rounding the
    /// dividend instead — as an earlier version of `Context::div` did — perturbs ties,
    /// direction and the exactness flag: the quotient of a *rounded* dividend can divide
    /// exactly where the true one doesn't, land on a false midpoint, or round the wrong way
    /// under directed modes.)
    ///
    /// The exact digit counts are only computed when the dividend is *possibly* wider than the
    /// bound: `digit_len` is an `ilog`, which for a non-power-of-two base computes a full power
    /// of the base, so the check first runs on integer-only bit-length bounds (the f32-based
    /// `Repr::digits_ub`/`digits_lb` call into libm's `log2` and are not free either).
    pub(crate) fn repr_div_any_width<const B: Word>(
        &self,
        lhs: Repr<B>,
        rhs: Repr<B>,
    ) -> FpResult<Repr<B>> {
        let maybe_wide = !lhs.is_pos_zero() && {
            // digit_len::<2> is the plain bit length (no libm)
            let (num_ub, _) = digits_bounds_from_bits::<B>(digit_len::<2>(&lhs.significand));
            let (_, den_lb) = digits_bounds_from_bits::<B>(digit_len::<2>(&rhs.significand));
            num_ub > den_lb + self.precision
        };
        if !maybe_wide {
            return self.repr_div(lhs, rhs);
        }

        // exact split (the sign of the dividend is preserved on both parts)
        let ddigits = digit_len::<B>(&rhs.significand);
        let k = digit_len::<B>(&lhs.significand).saturating_sub(ddigits + self.precision);
        if k == 0 {
            return self.repr_div(lhs, rhs);
        }
        let (hi, lo) = split_digits::<B>(lhs.significand, k);
        self.repr_div_split(
            Repr {
                significand: hi,
                exponent: lhs.exponent,
            },
            rhs,
            lo,
            k,
        )
    }

    /// The division kernel proper: computes `lhs / rhs` correctly rounded to this context's
    /// precision. `(lo, k)` describes the part of the dividend below its least significant
    /// kept digit: the true dividend is `(lhs.significand · B^k + lo) · B^lhs.exponent`, and
    /// the pair participates in the final rounding as a sticky fraction.
    fn repr_div_split<const B: Word>(
        &self,
        lhs: Repr<B>,
        rhs: Repr<B>,
        lo: IBig,
        k: usize,
    ) -> FpResult<Repr<B>> {
        assert_finite_operands(&lhs, &rhs);
        assert_limited_precision(self.precision);

        let sign_negative = lhs.sign() != rhs.sign();
        let sign = if sign_negative {
            Sign::Negative
        } else {
            Sign::Positive
        };

        if rhs.significand.is_zero() {
            if lhs.significand.is_zero() {
                // 0/0 is indeterminate; callers that can signal it (Context::div) check first,
                // otherwise fall through to div_rem which panics on division by zero.
            } else {
                // finite / 0 = ±inf (sign = XOR), returned as a value
                return Ok(Approximation::Exact(Repr::infinity_with_sign(sign)));
            }
        }

        // Work with a positive divisor so that the quotient and remainder from `div_rem`
        // (truncated division: the remainder carries the dividend's sign) keep the value's
        // sign in `q`, and `round_ratio` below sees a plain (integer + fraction) split.
        // Negation is O(1) (a sign flip on the shared buffer).
        let (num, den) = if rhs.significand.is_positive() {
            (lhs.significand, rhs.significand)
        } else {
            (-lhs.significand, -rhs.significand)
        };

        let (mut q, mut r) = num.div_rem(&den);
        let mut e = lhs
            .exponent
            .checked_add(k as isize)
            .and_then(|e| e.checked_sub(rhs.exponent))
            .ok_or({
                // lhs.exponent >= 0 whenever the addition overflows, and < 0 whenever the
                // subtraction underflows (see the digit bound on `k` above)
                if lhs.exponent >= 0 {
                    FpError::Overflow(sign)
                } else {
                    FpError::Underflow(sign)
                }
            })?;

        let mut qdigits = digit_len::<B>(&q);

        // Exact division with the quotient already within the precision: nothing to scale and
        // nothing to round (the `lo` sticky part is empty exactly when k == 0).
        if r.is_zero() && k == 0 && qdigits <= self.precision {
            return Ok(Approximation::Exact(
                make_div_repr(sign_negative, q, e).check_finite_exponent()?,
            ));
        }

        if q.is_zero() {
            // num < den: scale the remainder up so the quotient has ~precision digits
            let ddigits = digit_len::<B>(&den);
            let rdigits = digit_len::<B>(&r); // rdigits <= ddigits
            let shift = ddigits + self.precision - rdigits;
            shl_digits_in_place::<B>(&mut r, shift);
            e = e
                .checked_sub(shift as isize)
                .ok_or(FpError::Underflow(sign))?;
            let (q0, r0) = r.div_rem(&den);
            q = q0;
            r = r0;
            // the scaled dividend has exactly ddigits+precision digits, so the quotient has
            // `precision` or `precision + 1` digits; with rdigits == ddigits the remainder is
            // still strictly below the divisor, which rules out the carry to B^precision
            qdigits = if rdigits == ddigits {
                self.precision
            } else {
                digit_len::<B>(&q)
            };
        } else if qdigits < self.precision {
            // TODO: here the operations can be optimized: 1. prevent double power, 2. q += q0 can be |= if B is power of 2
            let shift = self.precision - qdigits;
            shl_digits_in_place::<B>(&mut q, shift);
            shl_digits_in_place::<B>(&mut r, shift);
            e = e
                .checked_sub(shift as isize)
                .ok_or(FpError::Underflow(sign))?;

            let (q0, r0) = r.div_rem(&den);
            q += q0;
            r = r0;
            // q·B^shift ≤ B^precision − B^shift and q0 < B^shift, so the sum stays below
            // B^precision: the scaled quotient has exactly `precision` digits
            qdigits = self.precision;
        }

        // At this point the quotient is `(q + num_f/den_f)·B^e` with `den_f = den·B^k` and
        // `num_f = r·B^k + lo` (the remainder plus the dividend's sticky low part), where
        // `|num_f| < den_f` and `q` has at most `precision + 1` digits.
        let den_f = if k > 0 { shl_digits::<B>(&den, k) } else { den };
        let num_f = if k > 0 {
            shl_digits::<B>(&r, k) + lo
        } else {
            r
        };

        // An over-wide quotient (an exact division such as 15/3 at precision 2, or the p+1
        // digit quotient the scaling above can produce) must be rounded to the precision —
        // but in a *single* step: the dropped digit joins the fraction, so no information is
        // double-rounded away. The dividend bound guarantees the quotient carries at most one
        // extra digit, so exactly one digit is dropped here.
        let repr = if qdigits > self.precision {
            debug_assert_eq!(qdigits, self.precision + 1);
            let (qh, ql) = split_digits::<B>(q, 1);
            let e2 = e.saturating_add(1);

            // The fraction is (ql + num_f/den_f)/B; ql (when nonzero) and num_f both carry
            // the dividend's sign, and |num_f| < den_f, so the fraction's sign is ql's, or
            // num_f's when ql = 0. Its comparison against the half is
            //   2·(ql·den_f + num_f)  vs  B·den_f   ⟺   2·num_f  vs  (B − 2·ql)·den_f,
            // computed below without materializing ql·den_f + num_f or B·den_f. The
            // comparison lives in the closure so the directed modes (which ignore it) never
            // pay for it.
            let ql_is_zero = ql.is_zero();
            let low_sign = if ql_is_zero { num_f.sign() } else { ql.sign() };

            if ql_is_zero && num_f.is_zero() {
                Approximation::Exact(make_div_repr(sign_negative, qh, e2))
            } else {
                let adjust = R::round_low_part(&qh, low_sign, || {
                    if B == 2 {
                        // The dropped digit is a single bit and the comparison collapses to
                        // one direct comparison: with ql = 0, the fraction num_f/(2·den_f) is
                        // at the half exactly when |num_f| = den_f (unreachable: |num_f| <
                        // den_f strictly); with ql = ±1, the fraction (ql + f)/2 is at the
                        // half exactly when f = 0.
                        if ql_is_zero {
                            num_f.abs_cmp(&den_f)
                        } else {
                            match ql.sign() {
                                Sign::Positive => num_f.cmp(&IBig::ZERO),
                                Sign::Negative => num_f.cmp(&IBig::ZERO).reverse(),
                            }
                        }
                    } else {
                        // The dividend's sign s multiplies both sides (flipping the
                        // comparison when negative): compare 2·|num_f| against s·(B − 2·ql)
                        // ·den_f — positive in this branch — halving both sides when
                        // B − 2·ql is even, which always holds for an even base.
                        let kk = IBig::from(B) - (ql << 1); // B − 2·ql, |kk| ≤ B
                        let pos = qh.sign() == Sign::Positive;
                        let skk = if pos { kk } else { -kk };
                        if skk.sign() != Sign::Positive {
                            // |2·num_f| ≥ 0 > s·(B − 2·ql)·den_f
                            Ordering::Greater
                        } else {
                            let mag = if pos { num_f } else { -num_f };
                            let ord = if !skk.bit(0) {
                                mag.cmp(&(&(skk >> 1) * &den_f))
                            } else {
                                (mag << 1).cmp(&(skk * &den_f))
                            };
                            if pos {
                                ord
                            } else {
                                ord.reverse()
                            }
                        }
                    }
                });
                Approximation::Inexact(make_div_repr(sign_negative, qh + adjust, e2), adjust)
            }
        } else if num_f.is_zero() {
            Approximation::Exact(make_div_repr(sign_negative, q, e))
        } else {
            let adjust = R::round_ratio(&q, num_f, &den_f);
            Approximation::Inexact(make_div_repr(sign_negative, q + adjust, e), adjust)
        };
        let repr = match repr {
            Approximation::Exact(v) => Approximation::Exact(v.check_finite_exponent()?),
            Approximation::Inexact(v, flag) => {
                Approximation::Inexact(v.check_finite_exponent()?, flag)
            }
        };
        Ok(repr)
    }

    pub(crate) fn repr_rem<const B: Word>(&self, lhs: Repr<B>, rhs: Repr<B>) -> Rounded<Repr<B>> {
        assert_finite_operands(&lhs, &rhs);

        let lhs_is_neg_zero = lhs.is_neg_zero();
        let (lhs_sign, lhs_signif) = lhs.significand.into_parts();
        let (_, rhs_signif) = rhs.significand.into_parts();

        use core::cmp::Ordering;
        let significand = match lhs.exponent.cmp(&rhs.exponent) {
            Ordering::Equal => {
                let r1 = lhs_signif % &rhs_signif;
                let r2 = rhs_signif - &r1;
                if r1 < r2 {
                    IBig::from_parts(lhs_sign, r1)
                } else {
                    IBig::from_parts(-lhs_sign, r2)
                }
            }
            Ordering::Greater => {
                // if the least significant digit of lhs is higher than rhs, then we can
                // align lhs to rhs and do simple modulo operations
                let modulo = ConstDivisor::new(rhs_signif);
                let shift = (lhs.exponent - rhs.exponent) as usize;
                let scaling = if B == 2 {
                    (UBig::ONE << shift).into_ring(&modulo)
                } else {
                    UBig::from_word(B).into_ring(&modulo).pow(&shift.into())
                };
                let r = lhs_signif.into_ring(&modulo) * scaling;
                let r1 = r.residue();
                let r2 = (-r).residue();
                if r1 < r2 {
                    IBig::from_parts(lhs_sign, r1)
                } else {
                    IBig::from_parts(-lhs_sign, r2)
                }
            }
            Ordering::Less => {
                // otherwise we have to split lhs into two parts
                let shift = (rhs.exponent - lhs.exponent) as usize;
                let (hi, lo) = split_digits::<B>(lhs_signif.into(), shift);

                let mut r1 = hi % &rhs_signif;
                let mut r2 = rhs_signif - &r1;

                shl_digits_in_place::<B>(&mut r1, shift);
                r1 += &lo;

                shl_digits_in_place::<B>(&mut r2, shift);
                r2 -= lo;

                if r1 < r2 {
                    lhs_sign * r1
                } else {
                    (-lhs_sign) * r2
                }
            }
        };

        let exponent = lhs.exponent.min(rhs.exponent);
        if significand.is_zero() {
            // the sign of a zero remainder follows the dividend (±0)
            Approximation::Exact(if lhs_is_neg_zero {
                Repr::neg_zero()
            } else {
                Repr::zero()
            })
        } else {
            match Repr::new(significand, exponent).check_finite_exponent() {
                Ok(repr) => self.repr_round(repr),
                Err(e) => match e {
                    FpError::Overflow(sign) => {
                        Approximation::Inexact(Repr::infinity_with_sign(sign), Rounding::NoOp)
                    }
                    FpError::Underflow(sign) => {
                        Approximation::Inexact(Repr::zero_with_sign(sign), Rounding::NoOp)
                    }
                    _ => unreachable!(),
                },
            }
        }
    }

    /// Divide two floating point numbers under this context.
    ///
    /// # Examples
    ///
    /// ```
    /// # use core::str::FromStr;
    /// # use dashu_base::ParseError;
    /// # use dashu_float::DBig;
    /// use dashu_base::Approximation::*;
    /// use dashu_float::{Context, round::{mode::HalfAway, Rounding::*}};
    ///
    /// let context = Context::<HalfAway>::new(2);
    /// let a = DBig::from_str("-1.234")?;
    /// let b = DBig::from_str("6.789")?;
    /// assert_eq!(context.div(&a.repr(), &b.repr()), Ok(Inexact(DBig::from_str("-0.18")?, NoOp)));
    /// # Ok::<(), ParseError>(())
    /// ```
    ///
    /// # Euclidean Division
    ///
    /// To do euclidean division on the float numbers (get an integer quotient and remainder, equivalent to C99's
    /// `fmod` and `remquo`), please use the methods provided by traits [DivEuclid], [RemEuclid] and [DivRemEuclid].
    ///
    pub fn div<const B: Word>(&self, lhs: &Repr<B>, rhs: &Repr<B>) -> FpResult<FBig<R, B>> {
        if lhs.is_infinite() || rhs.is_infinite() {
            return Err(FpError::InfiniteInput);
        }
        if lhs.significand.is_zero() && rhs.significand.is_zero() {
            return Err(FpError::Indeterminate); // 0/0
        }

        // No operand pre-shrinking here: `repr_div_any_width` bounds an over-wide dividend by
        // an exact split, keeping the dropped digits as rounding information. Rounding the
        // dividend *before* dividing — as this used to do — corrupts the result: the rounded
        // dividend can divide exactly where the true one doesn't (a false `Exact` flag),
        // land the quotient on a false midpoint, or invert the direction under directed
        // modes.
        Ok(self
            .repr_div_any_width(lhs.clone(), rhs.clone())?
            .map(|v| FBig::new(v, *self)))
    }

    /// Calculate the remainder of `⌈lhs / rhs⌋`.
    ///
    /// The remainder is calculated as `r = lhs - ⌈lhs / rhs⌋ * rhs`, the division rounds to the nearest and ties to away.
    /// So if `n = (lhs / rhs).round()`, then `lhs == n * rhs + r` (given enough precision).
    ///
    /// # Examples
    ///
    /// ```
    /// # use core::str::FromStr;
    /// # use dashu_base::ParseError;
    /// # use dashu_float::DBig;
    /// use dashu_base::Approximation::*;
    /// use dashu_float::{Context, round::{mode::HalfAway, Rounding::*}};
    ///
    /// let context = Context::<HalfAway>::new(3);
    /// let a = DBig::from_str("6.789")?;
    /// let b = DBig::from_str("-1.234")?;
    /// assert_eq!(context.rem(&a.repr(), &b.repr()), Ok(Exact(DBig::from_str("-0.615")?)));
    /// # Ok::<(), ParseError>(())
    /// ```
    pub fn rem<const B: Word>(&self, lhs: &Repr<B>, rhs: &Repr<B>) -> FpResult<FBig<R, B>> {
        if lhs.is_infinite() || rhs.is_infinite() {
            return Err(FpError::InfiniteInput);
        }
        Ok(self
            .repr_rem(lhs.clone(), rhs.clone())
            .map(|v| FBig::new(v, *self)))
    }

    /// Compute the multiplicative inverse of an `FBig`
    ///
    /// ```
    /// # use core::str::FromStr;
    /// # use dashu_base::ParseError;
    /// # use dashu_float::DBig;
    /// use dashu_base::Approximation::*;
    /// use dashu_float::{Context, round::{mode::HalfAway, Rounding::*}};
    ///
    /// let context = Context::<HalfAway>::new(2);
    /// let a = DBig::from_str("-1.234")?;
    /// assert_eq!(context.inv(&a.repr()), Ok(Inexact(DBig::from_str("-0.81")?, NoOp)));
    /// # Ok::<(), ParseError>(())
    /// ```
    #[inline]
    pub fn inv<const B: Word>(&self, f: &Repr<B>) -> FpResult<FBig<R, B>> {
        if f.is_infinite() {
            return Err(FpError::InfiniteInput);
        }
        // inv(±0) = ±inf (produced as a value by repr_div)
        Ok(self
            .repr_div(Repr::one(), f.clone())?
            .map(|v| FBig::new(v, *self)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::round::mode;
    use dashu_base::Approximation::*;

    fn r2(sig: i32, exp: isize) -> Repr<2> {
        Repr::new(sig.into(), exp)
    }

    // Regression tests for the rounding bugs reported in issue #100 (and two same-class value
    // bugs found while investigating): `Context::div` used to round an over-wide dividend
    // *before* dividing (perturbing ties, direction and the exactness flag), and `repr_div`
    // returned exact quotients unreduced and rounded p+1-digit quotients on the wrong grid.
    #[test]
    fn test_div_quotient_reduced_and_flagged() {
        // 15/3 = 5 needs three digits at precision 2: the exact quotient must be rounded
        // (Zero -> 4), not returned unreduced with an Exact flag.
        let r = Context::<mode::Zero>::new(2)
            .div(&r2(15, 0), &r2(3, 0))
            .unwrap();
        assert!(matches!(r, Inexact(..)), "15/3 @ p2 must be inexact");
        assert_eq!(r.value().repr(), &r2(1, 2));

        // 5/1 at precision 1: same unreduced-quotient bug — 5 is not representable at p1.
        let r = Context::<mode::Zero>::new(1)
            .div(&r2(5, 0), &r2(1, 0))
            .unwrap();
        assert!(matches!(r, Inexact(..)), "5/1 @ p1 must be inexact");
        assert_eq!(r.value().repr(), &r2(1, 2));

        // 63/3 = 21 at precision 2 under Zero: the dividend is pre-split (not pre-rounded!),
        // and 21 truncates onto the 2-digit grid as 1·2^4 = 16.
        let v = Context::<mode::Zero>::new(2)
            .div(&r2(63, 0), &r2(3, 0))
            .unwrap()
            .value();
        assert_eq!(v.repr(), &r2(1, 4));
    }

    #[test]
    fn test_div_directed_modes_on_wide_dividend() {
        // 7/-1 = -7 at precision 1 under Up must round toward +inf (-4). The old pre-shrink
        // rounded the dividend 7 up to 8 first, and 8/-1 = -8 then rounded *down*.
        let v = Context::<mode::Up>::new(1)
            .div(&r2(7, 0), &r2(-1, 0))
            .unwrap()
            .value();
        assert_eq!(v.repr(), &r2(-1, 2));

        // 31/4 = 7.75 in base 3 at precision 1 under HalfEven: the old pre-shrink turned it
        // into 30/4 = 7.5, a false midpoint that tied down to 6; the true value is above the
        // 7.5 midpoint of 6 and 9, so it rounds to 9.
        let v = Context::<mode::HalfEven>::new(1)
            .div(&Repr::<3>::new(IBig::from(31), 0), &Repr::<3>::new(IBig::from(4), 0))
            .unwrap()
            .value();
        assert_eq!(v.repr(), &Repr::<3>::new(IBig::from(1), 2));
    }

    // Two same-class value bugs in the p+1-digit quotient paths (found while investigating
    // #100): the final rounding used the integer grid instead of the precision-digit grid.
    #[test]
    fn test_div_p1_digit_quotient_rounds_on_precision_grid() {
        // 3/5 = 0.6 at precision 2 under HalfEven: 0.6·2^3 = 4.8 used to be rounded on the
        // integer grid to 5 (= 0.625, not even representable at p2); the p2 neighbours are
        // 0.5 and 0.75, so the result is 0.5.
        let v = Context::<mode::HalfEven>::new(2)
            .div(&r2(3, 0), &r2(5, 0))
            .unwrap()
            .value();
        assert_eq!(v.repr(), &r2(1, -1));

        // 14/3 = 4.67 at precision 2 under HalfEven: the quotient 100₂ carries p+1 digits;
        // the p2 grid is 100/110 (4 and 6) with midpoint 5, so 4.67 rounds to 4 (the integer
        // grid would give 5).
        let v = Context::<mode::HalfEven>::new(2)
            .div(&r2(14, 0), &r2(3, 0))
            .unwrap()
            .value();
        assert_eq!(v.repr(), &r2(1, 2));

        // 7/2 = 3.5 at precision 2 under HalfEven: the divisor normalizes to 1·2^1, so the
        // exact quotient 111·2^-1 used to be returned unreduced. 3.5 is the exact midpoint of
        // 11 (3) and 100 (4); ties to even gives 4.
        let v = Context::<mode::HalfEven>::new(2)
            .div(&r2(7, 0), &r2(2, 0))
            .unwrap()
            .value();
        assert_eq!(v.repr(), &r2(1, 2));
    }

    // The dividend's sticky low digits (split off by the width bound) must participate in the
    // rounding: they resolve exact ties and push directed modes the right way.
    #[test]
    fn test_div_sticky_low_digits() {
        // 15·2^-1 / 5 = 1.5 at precision 1 under HalfEven: exact tie between 1 and 2 -> 2.
        let v = Context::<mode::HalfEven>::new(1)
            .div(&r2(15, -1), &r2(5, 0))
            .unwrap()
            .value();
        assert_eq!(v.repr(), &r2(1, 1));

        // Same input under Up: the tie pushes up to 2.
        let v = Context::<mode::Up>::new(1)
            .div(&r2(15, -1), &r2(5, 0))
            .unwrap()
            .value();
        assert_eq!(v.repr(), &r2(1, 1));

        // 11/1 at precision 2 under HalfEven: the dividend 1011₂ is split at 1 digit; without
        // the sticky low bit the rounding would see an exact tie (10|1 -> even -> 10), with it
        // the fraction is 3/4 and the result is 1100₂ = 12.
        let v = Context::<mode::HalfEven>::new(2)
            .div(&r2(11, 0), &r2(1, 0))
            .unwrap()
            .value();
        assert_eq!(v.repr(), &r2(3, 2));

        // 25·2^-1 / 5 = 2.5 at precision 1 under Up: the p1 neighbours are 2 and 4, and a
        // directed mode picks the one in its direction regardless of the (midpoint 3).
        let v = Context::<mode::Up>::new(1)
            .div(&r2(25, -1), &r2(5, 0))
            .unwrap()
            .value();
        assert_eq!(v.repr(), &r2(1, 2));
    }

    #[test]
    fn test_div_by_zero_is_infinity() {
        let ctx = Context::<mode::HalfEven>::new(53);
        // finite / 0 = ±inf (a value, not an error); sign = XOR
        let pos = ctx.div::<2>(&r2(1, 0), &Repr::<2>::zero()).unwrap().value();
        assert!(pos.repr().is_infinite());
        assert_eq!(pos.repr().sign(), Sign::Positive);

        let neg = ctx
            .div::<2>(&r2(-1, 0), &Repr::<2>::zero())
            .unwrap()
            .value();
        assert_eq!(neg.repr().sign(), Sign::Negative);

        // 1 / -0 = -inf
        let neg2 = ctx
            .div::<2>(&r2(1, 0), &Repr::<2>::neg_zero())
            .unwrap()
            .value();
        assert_eq!(neg2.repr().sign(), Sign::Negative);
    }

    #[test]
    fn test_zero_over_zero_is_indeterminate() {
        let ctx = Context::<mode::HalfEven>::new(53);
        assert_eq!(
            ctx.div::<2>(&Repr::<2>::zero(), &Repr::<2>::zero()),
            Err(FpError::Indeterminate)
        );
    }

    #[test]
    fn test_inv_zero_is_infinity() {
        let ctx = Context::<mode::HalfEven>::new(53);
        let r = ctx.inv::<2>(&Repr::<2>::zero()).unwrap().value();
        assert!(r.repr().is_infinite());
        assert_eq!(r.repr().sign(), Sign::Positive);
    }

    #[test]
    fn test_fbig_div_zero_produces_infinity() {
        // FBig convenience layer: 1 / 0 yields an infinity-valued FBig (no panic).
        let one = FBig::<mode::HalfEven>::try_from(1.0f64).unwrap();
        let zero = FBig::<mode::HalfEven>::try_from(0.0f64).unwrap();
        let inf = one / zero;
        assert!(inf.repr().is_infinite());
    }

    #[test]
    #[should_panic]
    fn test_fbig_zero_over_zero_panics() {
        // 0 / 0 is indeterminate; the FBig layer panics.
        let zero = FBig::<mode::HalfEven>::try_from(0.0f64).unwrap();
        let _ = zero.clone() / zero;
    }

    // The `FBig / FBig` operator routes through `unwrap_fp`, so an exponent underflow saturates to
    // the directed endpoint (not a mode-blind signed zero): 2^isize::MIN / 3 ≈ 2^(isize::MIN − 2)
    // underflows; Up → smallest positive, Down → +0.
    #[test]
    fn test_div_directed_underflow() {
        use dashu_int::IBig;
        let p = 53;
        let floor_up = FBig::<mode::Up, 2>::from_parts(IBig::ONE, isize::MIN)
            .with_precision(p)
            .value();
        let floor_down = FBig::<mode::Down, 2>::from_parts(IBig::ONE, isize::MIN)
            .with_precision(p)
            .value();
        let three_up = FBig::<mode::Up, 2>::from_parts(IBig::from(3), 0)
            .with_precision(p)
            .value();
        let three_down = FBig::<mode::Down, 2>::from_parts(IBig::from(3), 0)
            .with_precision(p)
            .value();
        let up = floor_up / &three_up;
        let down = floor_down / &three_down;
        assert_eq!(up.repr().significand(), &IBig::ONE);
        assert_eq!(up.repr().exponent(), isize::MIN);
        assert!(down.repr().is_pos_zero());
        assert!(up > down);
    }
}
