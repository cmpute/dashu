use core::{
    convert::{TryFrom, TryInto},
    num::FpCategory,
};

use dashu_base::{
    AbsOrd, Approximation::*, BitTest, ConversionError, DivExact, DivRemEuclid, EstimatedLog2,
    FloatEncoding, Sign, Signed,
};
use dashu_int::{IBig, UBig, Word};

use crate::{
    error::{assert_finite, panic_unlimited_precision, FpError, FpResult},
    fbig::FBig,
    math::cache::{reborrow_cache, ConstCache},
    repr::{Context, Repr},
    round::{
        mode::{HalfAway, HalfEven, Zero},
        Round, Rounded, Rounding,
        Rounding::*,
    },
    utils::{factor_base, ilog_exact, shl_digits, shl_digits_in_place, shr_digits},
};

impl<R: Round> Context<R> {
    /// Convert an [IBig] instance to a [FBig] instance with precision
    /// and rounding given by the context.
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
    /// assert_eq!(context.convert_int::<10>((-12).into()), Exact(DBig::from_str("-12")?));
    /// assert_eq!(
    ///     context.convert_int::<10>(5678.into()),
    ///     Inexact(DBig::from_str("5.7e3")?, AddOne)
    /// );
    /// # Ok::<(), ParseError>(())
    /// ```
    pub fn convert_int<const B: Word>(&self, n: IBig) -> Rounded<FBig<R, B>> {
        let repr = Repr::<B>::new(n, 0);
        self.repr_round(repr).map(|v| FBig::new(v, *self))
    }
}

macro_rules! impl_from_float_for_fbig {
    ($t:ty) => {
        impl TryFrom<$t> for Repr<2> {
            type Error = ConversionError;

            fn try_from(f: $t) -> Result<Self, Self::Error> {
                match f.decode() {
                    Ok((man, exp)) => Ok(if man == 0 && f.is_sign_negative() {
                        Self::neg_zero()
                    } else {
                        Repr::new(man.into(), exp as _)
                    }),
                    Err(FpCategory::Infinite) => match f.sign() {
                        Sign::Positive => Ok(Self::infinity()),
                        Sign::Negative => Ok(Self::neg_infinity()),
                    },
                    _ => Err(ConversionError::OutOfBounds), // NaN
                }
            }
        }

        impl<R: Round> TryFrom<$t> for FBig<R, 2> {
            type Error = ConversionError;

            fn try_from(f: $t) -> Result<Self, Self::Error> {
                match f.decode() {
                    Ok((man, exp)) => {
                        // preserve the sign of a signed zero (-0.0 -> Repr::neg_zero())
                        let repr = if man == 0 && f.is_sign_negative() {
                            Repr::neg_zero()
                        } else {
                            Repr::new(man.into(), exp as _)
                        };

                        // The precision is inferenced from the mantissa, because the mantissa of
                        // normal float is always normalized. This will produce correct precision
                        // for subnormal floats
                        let bits = man.unsigned_abs().bit_len();
                        let context = Context::new(bits);
                        Ok(Self::new(repr, context))
                    }
                    Err(FpCategory::Infinite) => match f.sign() {
                        Sign::Positive => Ok(Self::INFINITY),
                        Sign::Negative => Ok(Self::NEG_INFINITY),
                    },
                    _ => Err(ConversionError::OutOfBounds), // NaN
                }
            }
        }
    };
}

impl_from_float_for_fbig!(f32);
impl_from_float_for_fbig!(f64);

impl<R: Round, const B: Word> FBig<R, B> {
    /// Convert the float number to base 10 (with decimal exponents) rounding to even
    /// and tying away from zero.
    ///
    /// It's equivalent to `self.with_rounding::<HalfAway>().with_base::<10>()`.
    /// The output is directly of type [DBig][crate::DBig].
    ///
    /// See [with_base()][Self::with_base] for the precision behavior.
    ///
    /// # Examples
    ///
    /// ```
    /// # use core::str::FromStr;
    /// # use dashu_base::ParseError;
    /// # use dashu_float::{FBig, DBig};
    /// use dashu_base::Approximation::*;
    /// use dashu_float::round::Rounding::*;
    ///
    /// type Real = FBig;
    ///
    /// assert_eq!(
    ///     Real::from_str("0x1234")?.to_decimal(),
    ///     Exact(DBig::from_str("4660")?)
    /// );
    /// assert_eq!(
    ///     Real::from_str("0x12.34")?.to_decimal(),
    ///     Inexact(DBig::from_str("18.20")?, NoOp)
    /// );
    /// assert_eq!(
    ///     Real::from_str("0x1.234p-4")?.to_decimal(),
    ///     Inexact(DBig::from_str("0.07111")?, AddOne)
    /// );
    /// # Ok::<(), ParseError>(())
    /// ```
    ///
    /// # Panics
    ///
    /// Panics if the associated context has unlimited precision and the conversion
    /// cannot be performed losslessly.
    #[inline]
    pub fn to_decimal(&self) -> Rounded<FBig<HalfAway, 10>> {
        self.clone().with_rounding().with_base::<10>()
    }

    /// Convert the float number to base 2 (with binary exponents) rounding towards zero.
    ///
    /// It's equivalent to `self.with_rounding::<Zero>().with_base::<2>()`.
    ///
    /// See [with_base()][Self::with_base] for the precision and rounding behavior.
    ///
    /// # Examples
    ///
    /// ```
    /// # use core::str::FromStr;
    /// # use dashu_base::ParseError;
    /// # use dashu_float::{FBig, DBig};
    /// use dashu_base::Approximation::*;
    /// use dashu_float::round::{mode::HalfAway, Rounding::*};
    ///
    /// type Real = FBig;
    ///
    /// assert_eq!(
    ///     DBig::from_str("1234")?.to_binary(),
    ///     Exact(Real::from_str("0x4d2")?)
    /// );
    /// assert_eq!(
    ///     DBig::from_str("12.34")?.to_binary(),
    ///     Inexact(Real::from_str("0xc.57")?, NoOp)
    /// );
    /// assert_eq!(
    ///     DBig::from_str("1.234e-1")?.to_binary(),
    ///     Inexact(Real::from_str("0x1.f97p-4")?, NoOp)
    /// );
    /// # Ok::<(), ParseError>(())
    /// ```
    ///
    /// # Panics
    ///
    /// Panics if the associated context has unlimited precision and the conversion
    /// cannot be performed losslessly.
    #[inline]
    pub fn to_binary(&self) -> Rounded<FBig<Zero, 2>> {
        self.clone().with_rounding().with_base::<2>()
    }

    /// Explicitly change the precision of the float number.
    ///
    /// If the given precision is less than the current value in the context,
    /// it will be rounded with the rounding mode specified by the generic parameter.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use core::str::FromStr;
    /// # use dashu_base::ParseError;
    /// # use dashu_float::{FBig, DBig};
    /// use dashu_base::Approximation::*;
    /// use dashu_float::round::{mode::HalfAway, Rounding::*};
    ///
    /// let a = DBig::from_str("2.345")?;
    /// assert_eq!(a.precision(), 4);
    /// assert_eq!(
    ///     a.clone().with_precision(3),
    ///     Inexact(DBig::from_str("2.35")?, AddOne)
    /// );
    /// assert_eq!(
    ///     a.clone().with_precision(5),
    ///     Exact(DBig::from_str("2.345")?)
    /// );
    /// # Ok::<(), ParseError>(())
    /// ```
    #[inline]
    pub fn with_precision(self, precision: usize) -> Rounded<Self> {
        let new_context = Context::new(precision);

        // shrink if necessary
        let repr = if self.context.precision > precision {
            // it also handles unlimited precision
            new_context.repr_round(self.repr)
        } else {
            Exact(self.repr)
        };

        repr.map(|v| Self::new(v, new_context))
    }

    /// Explicitly change the rounding mode of the number.
    ///
    /// This operation doesn't modify the underlying representation, it only changes
    /// the rounding mode in the context.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use core::str::FromStr;
    /// # use dashu_base::ParseError;
    /// # use dashu_float::{FBig, DBig};
    /// use dashu_base::Approximation::*;
    /// use dashu_float::round::{mode::{HalfAway, Zero}, Rounding::*};
    ///
    /// type DBigHalfAway = DBig;
    /// type DBigZero = FBig::<Zero, 10>;
    ///
    /// let a = DBigHalfAway::from_str("2.345")?;
    /// let b = DBigZero::from_str("2.345")?;
    /// assert_eq!(a.with_rounding::<Zero>(), b);
    /// # Ok::<(), ParseError>(())
    /// ```
    #[inline]
    pub fn with_rounding<NewR: Round>(self) -> FBig<NewR, B> {
        FBig {
            repr: self.repr,
            context: Context::new(self.context.precision),
        }
    }

    /// Explicitly change the base of the float number.
    ///
    /// This function internally calls [with_base_and_precision][Self::with_base_and_precision].
    /// The precision of the result number will be calculated in such a way that the new
    /// limit of the significand is less than or equal to before. That is, the new precision
    /// will be the max integer such that
    ///
    /// `NewB ^ new_precision <= B ^ old_precision`
    ///
    /// If any rounding happens during the conversion, it follows the rounding mode specified
    /// by the generic parameter.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use core::str::FromStr;
    /// # use dashu_base::ParseError;
    /// # use dashu_float::{FBig, DBig};
    /// use dashu_base::Approximation::*;
    /// use dashu_float::round::{mode::Zero, Rounding::*};
    ///
    /// type FBin = FBig;
    /// type FDec = FBig<Zero, 10>;
    /// type FHex = FBig<Zero, 16>;
    ///
    /// let a = FBin::from_str("0x1.234")?; // 0x1234 * 2^-12
    /// assert_eq!(
    ///     a.clone().with_base::<10>(),
    ///     // 1.1376953125 rounded towards zero
    ///     Inexact(FDec::from_str("1.137")?, NoOp)
    /// );
    /// assert_eq!(
    ///     a.clone().with_base::<16>(),
    ///     // conversion is exact when the new base is a power of the old base
    ///     Exact(FHex::from_str("1.234")?)
    /// );
    /// # Ok::<(), ParseError>(())
    /// ```
    ///
    /// # Panics
    ///
    /// Panics if the associated context has unlimited precision and the conversion
    /// cannot be performed losslessly.
    #[inline]
    #[allow(non_upper_case_globals)]
    pub fn with_base<const NewB: Word>(self) -> Rounded<FBig<R, NewB>> {
        // if self.context.precision is zero, then precision is also zero
        let precision =
            Repr::<B>::BASE.pow(self.context.precision).log2_bounds().0 / NewB.log2_bounds().1;
        self.with_base_and_precision(precision as usize)
    }

    /// Explicitly change the base of the float number with given precision (under the new base).
    ///
    /// Infinities are mapped to infinities inexactly, the error will be [Rounding::NoOp].
    ///
    /// Conversion for float numbers with unlimited precision is only allowed in following cases:
    /// - The number is infinite
    /// - The new base NewB is a power of B
    /// - B is a power of the new base NewB
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use core::str::FromStr;
    /// # use dashu_base::ParseError;
    /// # use dashu_float::{FBig, DBig};
    /// use dashu_base::Approximation::*;
    /// use dashu_float::round::{mode::Zero, Rounding::*};
    ///
    /// type FBin = FBig;
    /// type FDec = FBig<Zero, 10>;
    /// type FHex = FBig<Zero, 16>;
    ///
    /// let a = FBin::from_str("0x1.234")?; // 0x1234 * 2^-12
    /// assert_eq!(
    ///     a.clone().with_base_and_precision::<10>(8),
    ///     // 1.1376953125 rounded towards zero
    ///     Inexact(FDec::from_str("1.1376953")?, NoOp)
    /// );
    /// assert_eq!(
    ///     a.clone().with_base_and_precision::<16>(8),
    ///     // conversion can be exact when the new base is a power of the old base
    ///     Exact(FHex::from_str("1.234")?)
    /// );
    /// assert_eq!(
    ///     a.clone().with_base_and_precision::<16>(2),
    ///     // but the conversion is still inexact if the target precision is smaller
    ///     Inexact(FHex::from_str("1.2")?, NoOp)
    /// );
    /// # Ok::<(), ParseError>(())
    /// ```
    ///
    /// # Panics
    ///
    /// Panics if the associated context has unlimited precision and the conversion
    /// cannot be performed losslessly.
    #[allow(non_upper_case_globals)]
    #[inline]
    pub fn with_base_and_precision<const NewB: Word>(
        self,
        precision: usize,
    ) -> Rounded<FBig<R, NewB>> {
        let context = Context::<R>::new(precision);
        context
            .convert_base(self.repr, None)
            .map(|repr| FBig::new(repr, context))
    }

    /// Convert the float number to integer with the given rounding mode.
    ///
    /// # Warning
    ///
    /// If the float number has a very large exponent, it will be evaluated and result
    /// in allocating an huge integer and it might eat up all your memory.
    ///
    /// To get a rough idea of how big the number is, it's recommended to use [EstimatedLog2].
    ///
    /// # Examples
    ///
    /// ```
    /// # use core::str::FromStr;
    /// # use dashu_base::ParseError;
    /// # use dashu_float::{FBig, DBig};
    /// use dashu_base::Approximation::*;
    /// use dashu_float::round::Rounding::*;
    ///
    /// assert_eq!(
    ///     DBig::from_str("1234")?.to_int(),
    ///     Exact(1234.into())
    /// );
    /// assert_eq!(
    ///     DBig::from_str("1.234e6")?.to_int(),
    ///     Exact(1234000.into())
    /// );
    /// assert_eq!(
    ///     DBig::from_str("1.234")?.to_int(),
    ///     Inexact(1.into(), NoOp)
    /// );
    /// # Ok::<(), ParseError>(())
    /// ```
    ///
    /// # Panics
    ///
    /// Panics if the number is infinte
    pub fn to_int(&self) -> Rounded<IBig> {
        assert_finite(&self.repr);

        // shortcut when the number is already an integer
        if self.repr.exponent >= 0 {
            return Exact(shl_digits::<B>(&self.repr.significand, self.repr.exponent as usize));
        }

        let (hi, lo, precision) = self.split_at_point_internal();
        let adjust = R::round_fract::<B>(&hi, lo, precision);
        Inexact(hi + adjust, adjust)
    }

    /// Convert the float number to [f32] with the rounding mode associated with the type.
    ///
    /// Note that the conversion is inexact even if the number is infinite.
    ///
    /// # Examples
    ///
    /// ```
    /// # use core::str::FromStr;
    /// # use dashu_base::ParseError;
    /// # use dashu_float::DBig;
    /// assert_eq!(DBig::from_str("1.234")?.to_f32().value(), 1.234);
    /// assert_eq!(DBig::INFINITY.to_f32().value(), f32::INFINITY);
    /// # Ok::<(), ParseError>(())
    /// ```
    #[inline]
    pub fn to_f32(&self) -> Rounded<f32> {
        match Context::<R>::convert_to_f32(self.repr.clone()) {
            Ok(rounded) => rounded,
            Err(err) => f32_directed_endpoint::<R>(err),
        }
    }

    /// Convert the float number to [f64] with the rounding mode associated with the type.
    ///
    /// Note that the conversion is inexact even if the number is infinite.
    ///
    /// # Examples
    ///
    /// ```
    /// # use core::str::FromStr;
    /// # use dashu_base::ParseError;
    /// # use dashu_float::DBig;
    /// assert_eq!(DBig::from_str("1.234")?.to_f64().value(), 1.234);
    /// assert_eq!(DBig::INFINITY.to_f64().value(), f64::INFINITY);
    /// # Ok::<(), ParseError>(())
    /// ```
    #[inline]
    pub fn to_f64(&self) -> Rounded<f64> {
        match Context::<R>::convert_to_f64(self.repr.clone()) {
            Ok(rounded) => rounded,
            Err(err) => f64_directed_endpoint::<R>(err),
        }
    }
}

/// `isize` exponent arithmetic overflowed during base conversion: the value's magnitude
/// falls outside the representable exponent range, so the result is ±infinity (`large`) or
/// ±0 (`!large`). Mirrors the convention `convert_base` already uses in its division path,
/// keeping the conversion overflow-safe (no panic) at the value level.
#[allow(non_upper_case_globals)]
fn converted_overflow_repr<const NewB: Word>(large: bool, sign: Sign) -> Rounded<Repr<NewB>> {
    Inexact(
        if large {
            Repr::<NewB>::infinity_with_sign(sign)
        } else {
            Repr::<NewB>::zero_with_sign(sign)
        },
        Rounding::NoOp,
    )
}

/// Number of significant bits a binary float format keeps for a value whose most-significant bit
/// sits at position `msb`: `max_bits` across the normal range, but fewer for subnormals, whose
/// spacing is fixed at `2^subnormal_exp` (e.g. `2^-1074` for f64, `2^-149` for f32). Rounding the
/// source straight to this width lets the bit-encoding step avoid a second rounding, which would
/// otherwise double-round subnormals.
fn significand_bits(v: &Repr<2>, max_bits: usize, subnormal_exp: isize) -> usize {
    if v.significand.is_zero() {
        return max_bits;
    }
    let msb = v.exponent + v.digits() as isize - 1;
    (msb - subnormal_exp + 1).clamp(1, max_bits as isize) as usize
}

/// Convert `repr` to base 2 and reduce to a `width`-bit round-to-odd value: the top `width` bits
/// with the lowest kept bit forced to 1 whenever the conversion was inexact. Rounding this to any
/// width up to `width - 2` reproduces the correctly-rounded value for every rounding mode, so the
/// two-step "convert, then round to the final width" cannot double-round.
///
/// The conversion is first evaluated at `width + GUARD` bits (work precision `2·(width + GUARD)`)
/// and only then round-to-odd'd down to `width`. The extra guard is required for wide source
/// significands (e.g. a decimal `FBig` with hundreds of bits): the base-conversion logarithm is
/// *near-correct* — its ln/exp series carry a few-ulp error at the work precision — so converting
/// straight at `width` can land a value whose true result sits within ~`2^{-2·width}` of a
/// `width`-bit midpoint on the wrong side of that midpoint, and the subsequent round picks the
/// wrong neighbor (a decimal→f32 subnormal off by 1 ULP). The guard pushes that residual error
/// well below one `width`-bit ulp.
#[allow(non_upper_case_globals)]
fn convert_base_odd<const B: Word>(repr: Repr<B>, width: usize) -> Repr<2> {
    const GUARD: usize = 24;
    match Context::<Zero>::new(width + GUARD).convert_base::<B, 2>(repr, None) {
        Exact(v) => v,
        Inexact(v, _) if v.significand.is_zero() => v,
        Inexact(v, _) => {
            let digits = v.digits();
            let (sign, mut mag) = v.significand.into_parts();
            // collapse onto exactly `width` significant bits (drop or pad), then force the lowest
            // kept bit to 1 to mark the inexact conversion (round-to-odd).
            let exp = if digits >= width {
                let shift = digits - width;
                if shift > 0 {
                    mag >>= shift;
                }
                v.exponent + shift as isize
            } else {
                let shift = width - digits;
                mag <<= shift;
                v.exponent - shift as isize
            };
            mag.set_bit(0);
            Repr::new(IBig::from_parts(sign, mag), exp)
        }
    }
}

impl<R: Round> Context<R> {
    // Convert `repr` (base B) to the nearest f64 under this context's rounding mode. A generous
    // round-to-odd base conversion is rounded once to the target's precision at its own magnitude
    // (fewer than 53 bits for subnormals), so `into_f64_internal` re-rounds nothing — which would
    // otherwise double-round subnormals. Handles a source significand of any size.
    //
    // Returns `Err(FpError::Overflow/Underflow)` when the value is outside the finite f64 range;
    // the caller decides whether to saturate that to the directed endpoint (`to_f64`) or report it
    // as a conversion error (`TryFrom`). This makes `into_f64_internal` the single source of truth
    // for "is this value in range", shared by both APIs.
    fn convert_to_f64<const B: Word>(repr: Repr<B>) -> FpResult<f64> {
        if repr.is_infinite() {
            return Ok(Inexact(repr.sign() * f64::INFINITY, Rounding::NoOp));
        }
        // Underflow short-circuit on the *source* value (before base conversion). For a value far
        // below half the smallest subnormal (`|x| < 2^-1075`), the result is `±0` under nearest and
        // the smallest subnormal under outward modes — independent of base conversion. Checking the
        // source matters because converting a catastrophically tiny value (e.g. a wide-significand
        // decimal at a hugely negative exponent) drives the conversion's internal `exp` to underflow,
        // yielding an `odd` with a wildly wrong (too large) magnitude that `encode` then fails to
        // flag. `log2_bounds` on the source is exact (derived from the significand bit length), so
        // it sees the true magnitude; `ub < -1075` certifies `|x| < 2^-1075 = ½·MIN_SUBNORMAL`.
        // (Zero is excluded — it is exactly `0`, not an underflow.)
        if !repr.significand().is_zero() && repr.log2_bounds().1 < -1075.0 {
            return Err(FpError::Underflow(repr.sign()));
        }
        let odd = convert_base_odd::<B>(repr, 60);
        // Unified range check (shared by `to_f64` and `TryFrom`): a value beyond f64::MAX is out
        // of range regardless of rounding mode (the mode only picks the saturation endpoint).
        // Checked on the base-2 `odd`, BEFORE the significand rounding that could collapse a
        // beyond-MAX value onto MAX (which `encode` would then miss, breaking mode-independence).
        // `log2_bounds` fast-rejects the common case; the exact `abs_cmp` runs only near MAX.
        let (lb, ub) = odd.log2_bounds();
        if lb > 1024.0
            || (ub >= 1024.0
                && odd
                    .abs_cmp(&(UBig::from(0x1FFFFFFFFFFFFFu64) << 971))
                    .is_gt())
        {
            return Err(FpError::Overflow(odd.sign()));
        }
        let bits = significand_bits(&odd, 53, -1074);
        // The base conversion's rounding flag must propagate: `1e20 → f64` is inexact at the
        // round-to-odd step even though the already-rounded significand then `encode`s exactly.
        // This mirrors `Approximation::and_then` — an inexact input lifts an exact `encode` to
        // inexact with the input's flag, while an inexact `encode` keeps its own flag.
        let rounded = Context::<R>::new(bits).repr_round(odd);
        match rounded {
            Exact(v) => v.into_f64_internal(),
            Inexact(v, e) => match v.into_f64_internal()? {
                Exact(f) => Ok(Inexact(f, e)),
                Inexact(f, e2) => Ok(Inexact(f, e2)),
            },
        }
    }

    // [convert_to_f64] for f32.
    fn convert_to_f32<const B: Word>(repr: Repr<B>) -> FpResult<f32> {
        if repr.is_infinite() {
            return Ok(Inexact(repr.sign() * f32::INFINITY, Rounding::NoOp));
        }
        // See `convert_to_f64`: underflow short-circuit on the source value. f32's smallest
        // subnormal is `2^-149`, so `|x| < 2^-150 = ½·MIN_SUBNORMAL` rounds to `±0` (nearest) or
        // the smallest subnormal (outward). The source `log2_bounds` avoids the base-conversion
        // magnitude corruption that would otherwise hide a catastrophic underflow from `encode`.
        // (Zero is excluded — it is exactly `0`, not an underflow.)
        if !repr.significand().is_zero() && repr.log2_bounds().1 < -150.0 {
            return Err(FpError::Underflow(repr.sign()));
        }
        let odd = convert_base_odd::<B>(repr, 32);
        // See `convert_to_f64`: unified range check on the base-2 `odd`, before the significand
        // rounding. f32::MAX = (2^24 − 1) × 2^104 ≈ 2^128.
        let (lb, ub) = odd.log2_bounds();
        if lb > 128.0 || (ub >= 128.0 && odd.abs_cmp(&(UBig::from(0xFFFFFFu64) << 104)).is_gt()) {
            return Err(FpError::Overflow(odd.sign()));
        }
        let bits = significand_bits(&odd, 24, -149);
        let rounded = Context::<R>::new(bits).repr_round(odd);
        match rounded {
            Exact(v) => v.into_f32_internal(),
            Inexact(v, e) => match v.into_f32_internal()? {
                Exact(f) => Ok(Inexact(f, e)),
                Inexact(f, e2) => Ok(Inexact(f, e2)),
            },
        }
    }

    // Convert the [Repr] from base B to base NewB, with the precision under the target base from this context.
    #[allow(non_upper_case_globals)]
    fn convert_base<const B: Word, const NewB: Word>(
        &self,
        repr: Repr<B>,
        mut cache: Option<&mut ConstCache>,
    ) -> Rounded<Repr<NewB>> {
        // shortcut if NewB is the same as B
        if NewB == B {
            return Exact(Repr {
                significand: repr.significand,
                exponent: repr.exponent,
            });
        }

        // shortcut for infinities, no rounding happens but the result is inexact
        if repr.is_infinite() {
            return Inexact(
                Repr {
                    significand: repr.significand,
                    exponent: repr.exponent,
                },
                Rounding::NoOp,
            );
        }

        if NewB > B {
            // shortcut if NewB is a power of B
            let n = ilog_exact(NewB, B);
            if n > 1 {
                let (exp, rem) = repr.exponent.div_rem_euclid(n as isize);
                let signif = repr.significand * B.pow(rem as u32);
                let repr = Repr::new(signif, exp);
                return self.repr_round(repr);
            }
        } else {
            // shortcut if B is a power of NewB
            let n = ilog_exact(B, NewB);
            if n > 1 {
                let exp = match repr.exponent.checked_mul(n as isize) {
                    Some(e) => e,
                    None => return converted_overflow_repr::<NewB>(repr.exponent > 0, repr.sign()),
                };
                return Exact(Repr::new(repr.significand, exp));
            }
        }

        // Shortcut: when B and NewB share common factors, factor out the common part.
        // B = NewB^a * r where gcd(r, NewB) = 1, so B^exp = NewB^(a*exp) * r^exp.
        // For positive exponents the result is always exact (integer multiplication).
        // For negative exponents, exact only when r^|exp| divides the significand.
        let (a, r) = factor_base(B, NewB);
        if a > 0 && r > 1 {
            if repr.exponent >= 0 {
                let sign = repr.sign();
                let r_exp = UBig::from_word(r).pow(repr.exponent as usize);
                let significand = repr.significand * r_exp;
                let exp = match (a as isize).checked_mul(repr.exponent) {
                    Some(e) => e,
                    None => return converted_overflow_repr::<NewB>(true, sign),
                };
                let new_repr = Repr::<NewB>::new(significand, exp);
                return self.repr_round(new_repr);
            } else {
                let r_exp: IBig = UBig::from_word(r).pow((-repr.exponent) as usize).into();
                if let Some(quot) = (&repr.significand).div_exact(r_exp, &()) {
                    // the quotient's sign is the dividend's (r_exp > 0)
                    let exp = match (a as isize).checked_mul(repr.exponent) {
                        Some(e) => e,
                        None => return converted_overflow_repr::<NewB>(false, quot.sign()),
                    };
                    let new_repr = Repr::<NewB>::new(quot, exp);
                    return self.repr_round(new_repr);
                }
            }
        }

        // When NewB is a multiple of B: compute significand * B^exp directly
        // as an integer, then express in base NewB.
        if NewB % B == 0 && repr.exponent >= 0 {
            let signif = repr.significand * Repr::<B>::BASE.pow(repr.exponent as usize);
            let new_repr = Repr::<NewB>::new(signif, 0);
            return self.repr_round(new_repr);
        }

        // if the base cannot be converted losslessly, the precision must be set
        if self.precision == 0 {
            panic_unlimited_precision();
        }

        // choose a exponent threshold such that number with exponent smaller than this value
        // will be converted by directly evaluating the power. The threshold here is chosen such
        // that the power under base 10 will fit in a double word.
        const THRESHOLD_SMALL_EXP: isize = (Word::BITS as f32 * 0.60206) as isize; // word bits * 2 / log2(10)
        if repr.exponent.abs() <= THRESHOLD_SMALL_EXP {
            // if the exponent is small enough, directly evaluate the exponent
            if repr.exponent >= 0 {
                let signif = repr.significand * Repr::<B>::BASE.pow(repr.exponent as usize);
                Exact(Repr::new(signif, 0))
            } else {
                let den: Repr<NewB> =
                    Repr::new(Repr::<B>::BASE.pow(-repr.exponent as usize).into(), 0);
                // repr_div requires the dividend to be no wider than `precision + divisor`, so
                // pre-shrink the significand the same way Context::div does — the caller, not
                // the kernel, is responsible for bounding the dividend. Rounding it to
                // `den.digits() + precision` preserves enough information for the division to
                // be correctly rounded at `precision`.
                let num: Repr<NewB> = Repr::new(repr.significand, 0);
                let num =
                    if !num.is_pos_zero() && num.digits_ub() > den.digits_lb() + self.precision {
                        Self::new(den.digits() + self.precision)
                            .repr_round_ref(&num)
                            .value()
                    } else {
                        num
                    };
                match self.repr_div(num, den) {
                    Ok(v) => v.map(|r: Repr<NewB>| Repr {
                        significand: r.significand,
                        exponent: r.exponent,
                    }),
                    Err(FpError::Overflow(sign)) => {
                        Inexact(Repr::<NewB>::infinity_with_sign(sign), Rounding::NoOp)
                    }
                    Err(FpError::Underflow(sign)) => {
                        Inexact(Repr::<NewB>::zero_with_sign(sign), Rounding::NoOp)
                    }
                    Err(_) => unreachable!(),
                }
            }
        } else {
            // if the exponent is large, then we first estimate the result exponent as floor(exponent * log(B) / log(NewB)),
            // then the fractional part is multiplied with the original significand
            let work_context = Context::<R>::new(2 * self.precision); // double the precision to get the precise logarithm
                                                                      // ln(old base) and ln(new base) — near-correct is sufficient for the exponent estimate,
                                                                      // and using the near-correct `ln_compute`/`ln_base` (R: Round) keeps base conversion
                                                                      // off the `ErrorBounds` bound. Both are computed in base NewB so the euclidean division
                                                                      // has matching bases.
            let new_exp = repr.exponent
                * work_context
                    .ln_compute::<NewB>(
                        &Repr::new(Repr::<B>::BASE.into(), 0),
                        work_context.precision,
                        false,
                        reborrow_cache(&mut cache),
                    )
                    .to_value_radius::<R>()
                    .0;
            let (exponent, rem) =
                new_exp.div_rem_euclid(work_context.ln_base::<NewB>(reborrow_cache(&mut cache)));
            let exponent_sign = exponent.sign();
            let exponent: isize = match exponent.try_into() {
                Ok(v) => v,
                Err(_) => {
                    return converted_overflow_repr::<NewB>(
                        exponent_sign == Sign::Positive,
                        repr.sign(),
                    );
                }
            };
            // exp(fractional exponent) — near-correct is sufficient (it scales the significand),
            // so use `exp_compute` (R: Round) and stay off the `ErrorBounds` bound.
            let n = 1usize << (work_context.precision.bit_len() / 2);
            let exp_rem = work_context
                .exp_compute::<NewB>(
                    &rem.repr,
                    work_context.precision,
                    false,
                    n,
                    reborrow_cache(&mut cache),
                )
                .expect("exp(reduced rem) cannot overflow (|rem| < B^-n)")
                .mid;
            let significand = repr.significand * exp_rem.repr.significand;
            let repr = Repr::new(significand, exponent + exp_rem.repr.exponent);
            self.repr_round(repr)
        }
    }
}

impl<const B: Word> Repr<B> {
    // this method requires that the representation is already rounded to 24 binary bits
    fn into_f32_internal(self) -> FpResult<f32> {
        assert!(B == 2);
        debug_assert!(self.is_finite());
        debug_assert!(self.significand.bit_len() <= 24);

        let sign = self.sign();
        if self.is_neg_zero() {
            // encode() would drop the sign of -0; preserve it exactly
            return Ok(Exact(sign * 0f32));
        }
        let man24: i32 = self.significand.try_into().unwrap();
        match f32::encode(man24, self.exponent as i16) {
            Exact(v) => Ok(Exact(v)),
            Inexact(v, _) if v.is_infinite() => Err(FpError::Overflow(sign)),
            Inexact(0.0, _) => Err(FpError::Underflow(sign)),
            Inexact(v, _) => Ok(Inexact(v, Rounding::NoOp)),
        }
    }

    /// Convert the float number representation to a [f32] with the default IEEE 754 rounding mode.
    ///
    /// The default IEEE 754 rounding mode is [HalfEven] (rounding to nearest, ties to even). To convert
    /// the float number with a specific rounding mode, please use [FBig::to_f32].
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_base::Approximation::*;
    /// # use dashu_float::{Repr, round::Rounding::*};
    /// assert_eq!(Repr::<2>::one().to_f32(), Exact(1.0));
    /// assert_eq!(Repr::<10>::infinity().to_f32(), Inexact(f32::INFINITY, NoOp));
    /// ```
    #[inline]
    pub fn to_f32(&self) -> Rounded<f32> {
        match Context::<HalfEven>::convert_to_f32(self.clone()) {
            Ok(rounded) => rounded,
            Err(err) => f32_directed_endpoint::<HalfEven>(err),
        }
    }

    // this method requires that the representation is already rounded to 53 binary bits
    fn into_f64_internal(self) -> FpResult<f64> {
        assert!(B == 2);
        debug_assert!(self.is_finite());
        debug_assert!(self.significand.bit_len() <= 53);

        let sign = self.sign();
        if self.is_neg_zero() {
            // encode() would drop the sign of -0; preserve it exactly
            return Ok(Exact(sign * 0f64));
        }
        let man53: i64 = self.significand.try_into().unwrap();
        match f64::encode(man53, self.exponent as i16) {
            Exact(v) => Ok(Exact(v)),
            Inexact(v, _) if v.is_infinite() => Err(FpError::Overflow(sign)),
            Inexact(0.0, _) => Err(FpError::Underflow(sign)),
            Inexact(v, _) => Ok(Inexact(v, Rounding::NoOp)),
        }
    }

    /// Convert the float number representation to a [f64] with the default IEEE 754 rounding mode.
    ///
    /// The default IEEE 754 rounding mode is [HalfEven] (rounding to nearest, ties to even). To convert
    /// the float number with a specific rounding mode, please use [FBig::to_f64].
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_base::Approximation::*;
    /// # use dashu_float::{Repr, round::Rounding::*};
    /// assert_eq!(Repr::<2>::one().to_f64(), Exact(1.0));
    /// assert_eq!(Repr::<10>::infinity().to_f64(), Inexact(f64::INFINITY, NoOp));
    /// ```
    #[inline]
    pub fn to_f64(&self) -> Rounded<f64> {
        match Context::<HalfEven>::convert_to_f64(self.clone()) {
            Ok(rounded) => rounded,
            Err(err) => f64_directed_endpoint::<HalfEven>(err),
        }
    }

    /// Convert the float number representation to a [IBig].
    ///
    /// The fractional part is always rounded to zero. To convert with other rounding modes,
    /// please use [FBig::to_int()].
    ///
    /// # Warning
    ///
    /// If the float number has a very large exponent, it will be evaluated and result
    /// in allocating an huge integer and it might eat up all your memory.
    ///
    /// To get a rough idea of how big the number is, it's recommended to use [EstimatedLog2].
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_base::Approximation::*;
    /// # use dashu_int::IBig;
    /// # use dashu_float::{Repr, round::Rounding::*};
    /// assert_eq!(Repr::<2>::neg_one().to_int(), Exact(IBig::NEG_ONE));
    /// ```
    ///
    /// # Panics
    ///
    /// Panics if the number is infinte.
    pub fn to_int(&self) -> Rounded<IBig> {
        assert_finite(self);

        if self.exponent >= 0 {
            // the number is already an integer
            Exact(shl_digits::<B>(&self.significand, self.exponent as usize))
        } else if self.smaller_than_one() {
            // the number is definitely smaller than
            Inexact(IBig::ZERO, Rounding::NoOp)
        } else {
            let int = shr_digits::<B>(&self.significand, (-self.exponent) as usize);
            Inexact(int, Rounding::NoOp)
        }
    }
}

impl<const B: Word> From<UBig> for Repr<B> {
    #[inline]
    fn from(n: UBig) -> Self {
        Self::new(n.into(), 0)
    }
}
impl<R: Round, const B: Word> From<UBig> for FBig<R, B> {
    #[inline]
    fn from(n: UBig) -> Self {
        Self::from_parts(n.into(), 0)
    }
}

impl<const B: Word> From<IBig> for Repr<B> {
    #[inline]
    fn from(n: IBig) -> Self {
        Self::new(n, 0)
    }
}
impl<R: Round, const B: Word> From<IBig> for FBig<R, B> {
    #[inline]
    fn from(n: IBig) -> Self {
        Self::from_parts(n, 0)
    }
}

impl<R: Round, const B: Word> TryFrom<FBig<R, B>> for IBig {
    type Error = ConversionError;

    #[inline]
    fn try_from(value: FBig<R, B>) -> Result<Self, Self::Error> {
        if value.repr.is_infinite() {
            Err(ConversionError::OutOfBounds)
        } else if value.repr.significand.is_zero() {
            // A zero significand is integer zero regardless of exponent. This also
            // accepts IEEE-754 signed zero, whose sign is carried by a -1 exponent
            // sentinel (not the significand); it is treated as plain 0. The zero
            // must be handled here rather than in the `else` branch below, which
            // shifts by `exponent as usize` and would underflow on the -1 sentinel.
            Ok(value.repr.significand)
        } else if value.repr.exponent < 0 {
            Err(ConversionError::LossOfPrecision)
        } else {
            let mut int = value.repr.significand;
            shl_digits_in_place::<B>(&mut int, value.repr.exponent as usize);
            Ok(int)
        }
    }
}

impl<R: Round, const B: Word> TryFrom<FBig<R, B>> for UBig {
    type Error = ConversionError;

    #[inline]
    fn try_from(value: FBig<R, B>) -> Result<Self, Self::Error> {
        let int: IBig = value.try_into()?;
        int.try_into()
    }
}

macro_rules! fbig_unsigned_conversions {
    ($($t:ty)*) => {$(
        impl<const B: Word> From<$t> for Repr<B> {
            #[inline]
            fn from(value: $t) -> Repr<B> {
                UBig::from(value).into()
            }
        }
        impl<R: Round, const B: Word> From<$t> for FBig<R, B> {
            #[inline]
            fn from(value: $t) -> FBig<R, B> {
                UBig::from(value).into()
            }
        }

        impl<const B: Word> TryFrom<Repr<B>> for $t {
            type Error = ConversionError;

            fn try_from(value: Repr<B>) -> Result<Self, Self::Error> {
                if value.sign() == Sign::Negative || value.is_infinite() {
                    Err(ConversionError::OutOfBounds)
                } else {
                    let (log2_lb, _) = value.log2_bounds();
                    if log2_lb >= <$t>::BITS as f32 {
                        Err(ConversionError::OutOfBounds)
                    } else if value.exponent < 0 {
                        Err(ConversionError::LossOfPrecision)
                    } else {
                        shl_digits::<B>(&value.significand, value.exponent as usize).try_into()
                    }
                }
            }
        }
        impl<R: Round, const B: Word> TryFrom<FBig<R, B>> for $t {
            type Error = ConversionError;

            #[inline]
            fn try_from(value: FBig<R, B>) -> Result<Self, Self::Error> {
                value.repr.try_into()
            }
        }
    )*};
}
fbig_unsigned_conversions!(u8 u16 u32 u64 u128 usize);

macro_rules! fbig_signed_conversions {
    ($($t:ty)*) => {$(
        impl<R: Round, const B: Word> From<$t> for FBig<R, B> {
            #[inline]
            fn from(value: $t) -> FBig<R, B> {
                IBig::from(value).into()
            }
        }

        impl<R: Round, const B: Word> TryFrom<FBig<R, B>> for $t {
            type Error = ConversionError;

            fn try_from(value: FBig<R, B>) -> Result<Self, Self::Error> {
                if value.repr.is_infinite() {
                    Err(ConversionError::OutOfBounds)
                } else {
                    let (log2_lb, _) = value.repr.log2_bounds();
                    if log2_lb >= <$t>::BITS as f32 {
                        Err(ConversionError::OutOfBounds)
                    } else if value.repr.exponent < 0 {
                        Err(ConversionError::LossOfPrecision)
                    } else {
                        shl_digits::<B>(&value.repr.significand, value.repr.exponent as usize).try_into()
                    }
                }
            }
        }
    )*};
}
fbig_signed_conversions!(i8 i16 i32 i64 i128 isize);

// The directed saturation endpoint for an out-of-range f32/f64 result, chosen from the `FpError`
// returned by `into_f*_internal`. Overflow saturates to ±MAX or ±∞ per the mode (outward modes
// reach ±∞; toward-zero/opposite/nearest saturate to the largest finite); underflow saturates to
// ±0 or the smallest subnormal of that sign. `round_low_part`'s AddOne/SubOne verdict on a
// same-sign residual is exactly the outward-vs-inward decision; only its directional verdict is
// used. This is the single place that picks the endpoint, shared by `to_f32`/`to_f64` (Repr uses
// HalfEven, FBig uses its own mode).
macro_rules! impl_float_directed_endpoint {
    (
        $fn:ident, $t:ty, $max:expr, $min:expr, $inf:expr, $neg_inf:expr,
        $smallest_sub:expr, $neg_smallest_sub:expr
    ) => {
        fn $fn<R: Round>(err: FpError) -> Rounded<$t> {
            match err {
                FpError::Overflow(sign) => {
                    let adj = if sign == Sign::Positive {
                        R::round_low_part(&IBig::ONE, Sign::Positive, || {
                            core::cmp::Ordering::Greater
                        })
                    } else {
                        R::round_low_part(&IBig::NEG_ONE, Sign::Negative, || {
                            core::cmp::Ordering::Greater
                        })
                    };
                    Inexact(
                        match (sign, adj) {
                            (Sign::Positive, AddOne) => $inf,
                            (Sign::Positive, _) => $max,
                            (Sign::Negative, SubOne) => $neg_inf,
                            (Sign::Negative, _) => $min,
                        },
                        adj,
                    )
                }
                FpError::Underflow(sign) => {
                    let adj = if sign == Sign::Positive {
                        R::round_low_part(&IBig::ZERO, Sign::Positive, || core::cmp::Ordering::Less)
                    } else {
                        R::round_low_part(&IBig::ZERO, Sign::Negative, || core::cmp::Ordering::Less)
                    };
                    Inexact(
                        match (sign, adj) {
                            (Sign::Positive, AddOne) => $smallest_sub, // smallest positive subnormal
                            (Sign::Positive, _) => 0.0,
                            (Sign::Negative, SubOne) => $neg_smallest_sub,
                            (Sign::Negative, _) => -0.0,
                        },
                        adj,
                    )
                }
                // `into_f*_internal` only returns Overflow/Underflow; the infinite-input case is
                // handled by `convert_to_f*` (returning `Ok`) before this is reached.
                _ => unreachable!("convert_to_f* only returns Overflow/Underflow here"),
            }
        }
    };
}
impl_float_directed_endpoint!(
    f32_directed_endpoint,
    f32,
    f32::MAX,
    f32::MIN,
    f32::INFINITY,
    f32::NEG_INFINITY,
    f32::from_bits(1),
    f32::from_bits(0x8000_0001)
);
impl_float_directed_endpoint!(
    f64_directed_endpoint,
    f64,
    f64::MAX,
    f64::MIN,
    f64::INFINITY,
    f64::NEG_INFINITY,
    f64::from_bits(1),
    f64::from_bits(0x8000_0000_0000_0001)
);

macro_rules! impl_from_fbig_for_float {
    ($t:ty, $convert:ident) => {
        impl TryFrom<Repr<2>> for $t {
            type Error = ConversionError;

            #[inline]
            fn try_from(value: Repr<2>) -> Result<Self, Self::Error> {
                if value.is_infinite() {
                    return Err(ConversionError::LossOfPrecision);
                }
                // Range detection is shared with `to_f32`/`to_f64` via `convert_to_f*`: it returns
                // `Err(Overflow)` for a value beyond the largest finite `$t`, so `OutOfBounds` is
                // reported the same way under every rounding mode (Repr uses HalfEven here).
                match Context::<HalfEven>::$convert(value) {
                    Ok(Exact(v)) => Ok(v),
                    Ok(Inexact(_, _)) => Err(ConversionError::LossOfPrecision),
                    Err(FpError::Overflow(_)) => Err(ConversionError::OutOfBounds),
                    Err(FpError::Underflow(_)) => Err(ConversionError::LossOfPrecision),
                    Err(_) => unreachable!(),
                }
            }
        }

        impl<R: Round> TryFrom<FBig<R, 2>> for $t {
            type Error = ConversionError;

            #[inline]
            fn try_from(value: FBig<R, 2>) -> Result<Self, Self::Error> {
                if value.repr.is_infinite() {
                    return Err(ConversionError::LossOfPrecision);
                }
                // A value beyond the largest finite `$t` is out of range whatever the rounding mode:
                // the mode only selects the saturation endpoint (±MAX vs ±∞), and `convert_to_f*`
                // reports that range condition as `Err(Overflow)` regardless of mode — so
                // `Err(OutOfBounds)` reliably means "beyond the finite range", unlike the old
                // result-infiniteness check (which flipped LossOfPrecision/OutOfBounds with the mode).
                match Context::<R>::$convert(value.repr) {
                    Ok(Exact(v)) => Ok(v),
                    Ok(Inexact(_, _)) => Err(ConversionError::LossOfPrecision),
                    Err(FpError::Overflow(_)) => Err(ConversionError::OutOfBounds),
                    Err(FpError::Underflow(_)) => Err(ConversionError::LossOfPrecision),
                    Err(_) => unreachable!(),
                }
            }
        }
    };
}
impl_from_fbig_for_float!(f32, convert_to_f32);
impl_from_fbig_for_float!(f64, convert_to_f64);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repr::Repr;

    // Directed overflow must reach the endpoint for a value whose *most*-significant bit straddles
    // f32::MAX (not only for values whose lsb exponent is ≥ 128). `3·2¹²⁷` ≈ 1.5·2¹²⁸ overflows
    // f32::MAX = (2 − 2⁻²³)·2¹²⁷, yet its lsb exponent is 127 — the old exponent-gated branch fell
    // through to `encode`, which saturates to ±∞ mode-blindly.
    #[test]
    fn f32_directed_overflow_at_msb_boundary() {
        use crate::round::mode::{Down, Up};
        // f32: 3·2^127 under Zero/Down -> f32::MAX, under Up/Away -> +∞.
        let zero = FBig::<Zero, 2>::from_parts(3.into(), 127);
        let down = FBig::<Down, 2>::from_parts(3.into(), 127);
        let up = FBig::<Up, 2>::from_parts(3.into(), 127);
        assert_eq!(zero.to_f32().value().to_bits(), 0x7f7fffff); // f32::MAX
        assert_eq!(down.to_f32().value().to_bits(), 0x7f7fffff);
        assert!(up.to_f32().value().is_infinite() && up.to_f32().value().is_sign_positive());
        // negative mirror
        let nzero = FBig::<Zero, 2>::from_parts((-3).into(), 127);
        assert_eq!(nzero.to_f32().value().to_bits(), 0xff7fffff); // f32::MIN

        // f64: 3·2^1023 overflows f64::MAX; lsb exponent 1023 < 1024.
        let z64 = FBig::<Zero, 2>::from_parts(3.into(), 1023);
        assert_eq!(z64.to_f64().value().to_bits(), 0x7fefffffffffffff); // f64::MAX
        let u64 = FBig::<Up, 2>::from_parts(3.into(), 1023);
        assert!(u64.to_f64().value().is_infinite() && u64.to_f64().value().is_sign_positive());
    }

    // Directed underflow must reach the endpoint through the `encode` path too (not only for the
    // extreme exponents caught by the old explicit branch). `2⁻¹⁶⁰` is below the smallest subnormal;
    // under Up a positive value must round up to `2⁻¹⁴⁹`, but `encode` returns ±0 mode-blindly.
    #[test]
    fn f32_directed_underflow_through_encode() {
        use crate::round::mode::{Away, Down, Up};
        // f32: 2^-160 under Up/Away -> smallest +subnormal, under Zero/Down -> +0.
        let up = FBig::<Up, 2>::from_parts(IBig::ONE, -160);
        let away = FBig::<Away, 2>::from_parts(IBig::ONE, -160);
        let zero = FBig::<Zero, 2>::from_parts(IBig::ONE, -160);
        let down = FBig::<Down, 2>::from_parts(IBig::ONE, -160);
        assert_eq!(up.to_f32().value().to_bits(), 0x00000001); // smallest +subnormal
        assert_eq!(away.to_f32().value().to_bits(), 0x00000001);
        assert_eq!(zero.to_f32().value().to_bits(), 0x0);
        assert_eq!(down.to_f32().value().to_bits(), 0x0);
        // negative: Down/Away -> smallest -subnormal, Zero/Up -> -0.
        let ndown = FBig::<Down, 2>::from_parts(-IBig::ONE, -160);
        assert_eq!(ndown.to_f32().value().to_bits(), 0x80000001);

        // f64: 2^-1100 under Up -> smallest +subnormal.
        let u64 = FBig::<Up, 2>::from_parts(IBig::ONE, -1100);
        assert_eq!(u64.to_f64().value().to_bits(), 0x0000_0000_0000_0001);
    }

    // A decimal FBig whose value exceeds f32::MAX must saturate to the largest *finite* f32 under
    // toward-zero (Zero), not +∞ — directed overflow picks the endpoint per mode.
    #[test]
    fn f32_overflow_saturates_to_max_under_toward_zero() {
        use crate::round::mode::Up;
        // value ≈ 1.8e67 ≫ f32::MAX
        let sig: IBig = "18113714167384154970503568309331051197800435559900844351020490293248000000000000000000000000000000000000000000000000000000000000000000000040081793412673420422647135518538018600544982168035325793404432580816367361955984101780462905766719198411690240".parse().unwrap();
        let down = FBig::<Zero, 10>::from_parts(sig.clone(), -180);
        assert_eq!(down.to_f32().value().to_bits(), 0x7f7fffff); // f32::MAX, not +∞
                                                                 // The same value toward +∞ does reach +∞.
        let up = FBig::<Up, 10>::from_parts(sig, -180);
        assert!(up.to_f32().value().is_infinite());
    }

    // A wide decimal significand near an f32 subnormal midpoint must round to the correct
    // neighbor. The near-correct base-conversion logarithm previously landed on the wrong side of a
    // 32-bit midpoint, producing a result 1 ULP off (0x00040008 instead of 0x00040007).
    #[test]
    fn f32_decimal_subnormal_rounds_correctly() {
        let v = FBig::<HalfEven, 10>::from_parts(
            "367352494370447282365772889742681992006459962834329374643515088008836389924495676300982092025765783512749250624297822169761305238359955010699895018824154119671"
                .parse()
                .unwrap(),
            -198,
        );
        assert_eq!(v.to_f32().value().to_bits(), 0x00040007);
    }

    // A positive value below the smallest subnormal must round *up* to that smallest subnormal
    // under Up/Away (and to ±0 under toward-zero/opposite/nearest) — the underflow path used to
    // return a mode-blind signed zero, which is not an upper bound for a positive value under Up.
    #[test]
    fn f64_directed_underflow_below_smallest_subnormal() {
        use crate::round::mode::Down;
        // tiny positive binary value, far below 2^-1074
        let up = FBig::<crate::round::mode::Up, 2>::from_parts(IBig::ONE, -20000);
        let away = FBig::<crate::round::mode::Away, 2>::from_parts(IBig::ONE, -20000);
        let zero = FBig::<Zero, 2>::from_parts(IBig::ONE, -20000);
        let down = FBig::<Down, 2>::from_parts(IBig::ONE, -20000);
        assert_eq!(up.to_f64().value().to_bits(), 0x0000_0000_0000_0001); // smallest +subnormal
        assert_eq!(away.to_f64().value().to_bits(), 0x0000_0000_0000_0001);
        assert_eq!(zero.to_f64().value().to_bits(), 0x0); // +0
        assert_eq!(down.to_f64().value().to_bits(), 0x0);

        // tiny negative: Down/Away -> smallest -subnormal, Zero/Up -> -0
        let ndown = FBig::<Down, 2>::from_parts(-IBig::ONE, -20000);
        let nup = FBig::<crate::round::mode::Up, 2>::from_parts(-IBig::ONE, -20000);
        assert_eq!(ndown.to_f64().value().to_bits(), 0x8000_0000_0000_0001); // smallest -subnormal
        assert_eq!(nup.to_f64().value().to_bits(), 0x8000_0000_0000_0000); // -0
    }

    // A wide-significand *decimal* value far below ½·MIN_SUBNORMAL (exponent −20000). The base
    // conversion's internal `exp` underflows for such a catastrophically tiny value, so without the
    // source-`log2_bounds` short-circuit the converted magnitude is wrong and `to_f64` returned a
    // spurious finite subnormal (≈2^-593) instead of the directed underflow endpoint. The binary
    // test above doesn't hit this — it skips base conversion entirely.
    #[test]
    fn f64_decimal_wide_significand_underflow() {
        use crate::round::mode::{Down, Up};
        let wide = "1234567890123456789012345678901234567890123456789012345678901234567890123456789012345678901234"
            .parse::<IBig>()
            .unwrap();
        for prec in [20usize, 50, 100, 500] {
            let he = FBig::<HalfEven, 10>::from_parts(wide.clone(), -20000)
                .with_precision(prec)
                .value();
            let up = FBig::<Up, 10>::from_parts(wide.clone(), -20000)
                .with_precision(prec)
                .value();
            let dn = FBig::<Down, 10>::from_parts(wide.clone(), -20000)
                .with_precision(prec)
                .value();
            // positive value: nearest/Down -> +0, Up -> smallest positive subnormal
            assert_eq!(he.to_f64().value().to_bits(), 0x0, "HalfEven @ prec {prec}");
            assert_eq!(dn.to_f64().value().to_bits(), 0x0, "Down @ prec {prec}");
            assert_eq!(up.to_f64().value().to_bits(), 0x0000_0000_0000_0001, "Up @ prec {prec}");
            // f32 mirror: |x| < 2^-150 = ½·MIN_SUBNORMAL -> +0 / smallest +subnormal
            assert_eq!(he.to_f32().value().to_bits(), 0x0u32, "f32 HalfEven @ prec {prec}");
            assert_eq!(up.to_f32().value().to_bits(), 0x0000_0001u32, "f32 Up @ prec {prec}");

            // negative value: nearest/Up -> -0, Down -> smallest negative subnormal
            let neg = FBig::<HalfEven, 10>::from_parts(-wide.clone(), -20000)
                .with_precision(prec)
                .value();
            let ndn = FBig::<Down, 10>::from_parts(-wide.clone(), -20000)
                .with_precision(prec)
                .value();
            assert_eq!(
                neg.to_f64().value().to_bits(),
                0x8000_0000_0000_0000,
                "neg HalfEven @ prec {prec}"
            );
            assert_eq!(
                ndn.to_f64().value().to_bits(),
                0x8000_0000_0000_0001,
                "neg Down @ prec {prec}"
            );
        }
    }

    #[test]
    fn ibig_try_from_accepts_signed_zero() {
        // IEEE-754 signed zero (sign encoded in a -1 exponent sentinel) is plain 0.
        let neg_zero = FBig::<HalfAway, 10>::new(Repr::neg_zero(), Context::new(8));
        assert_eq!(IBig::try_from(neg_zero), Ok(IBig::from(0)));

        // positive zero already worked, and still does
        let pos_zero = FBig::<HalfAway, 10>::new(Repr::zero(), Context::new(8));
        assert_eq!(IBig::try_from(pos_zero), Ok(IBig::from(0)));

        // UBig delegates to the IBig impl, so it accepts signed zero too
        let neg_zero = FBig::<HalfAway, 2>::new(Repr::neg_zero(), Context::new(8));
        assert_eq!(UBig::try_from(neg_zero), Ok(UBig::from(0u8)));

        // a genuine fractional value must still be rejected
        let frac = FBig::<HalfAway, 10>::new(Repr::new(IBig::from(1), -1), Context::new(8));
        assert_eq!(IBig::try_from(frac), Err(ConversionError::LossOfPrecision));

        // a normal integer round-trips exactly
        let int_val = FBig::<HalfAway, 10>::new(Repr::new(IBig::from(42), 0), Context::new(8));
        assert_eq!(IBig::try_from(int_val), Ok(IBig::from(42)));
    }

    #[test]
    fn with_base_high_precision_no_overflow() {
        // Regression for issue #95: converting a high-precision base-2 float to base
        // 10 panicked on 32-bit targets ("arithmetic operations with the infinity are
        // not allowed!"). The base conversion evaluates exp(r) as `sum^(B^n)` through
        // `powi` with a huge exponent (B^n) on a base (sum) very close to 1; `powi`'s
        // overflow guard estimated log2(base) with the catastrophically-canceling
        // `log2_est`, and the ~1e-4 of f32 noise scaled by the exponent crossed the
        // (much smaller on 32-bit) isize threshold, yielding a spurious ±inf that then
        // panicked when shifted. See `powi` in exp.rs for the fix.
        use crate::round::mode::Zero;
        use core::str::FromStr;

        // The reporter's input: -1.1111…0011 in binary (578 significant bits), written
        // in the hex form dashu accepts for base-2 floats (`0x1.<hex>…`). The value is
        // identical to the raw binary literal.
        let num = FBig::<Zero, 2>::from_str(
            "-0x1.fffdc8d645194a5a95df4be063472d4406dd096339dd7dc2a8527d208b3da7b9e5c36b4f49a7982cb2ad20a4e7e4c016f858fe8cddea011a6d01fe3823189c4ed4f57a7babc331498",
        )
        .unwrap();

        // at the original 578-bit precision the conversion succeeds …
        let a = num
            .clone()
            .with_precision(578)
            .value()
            .with_base::<10>()
            .value();
        assert!(a.repr().is_finite());
        // … and so does a slightly higher precision (586), which panicked on 32-bit
        // (wasm32 / i686). The result matches the value computed on 64-bit. Compared
        // by value (FBig equality ignores context) rather than via string formatting,
        // so this works under no_std too.
        let b = num.with_precision(586).value().with_base::<10>().value();
        assert!(b.repr().is_finite());
        let expected = FBig::<Zero, 10>::from_str(
            "-1.9999661944503703041843468850635057967553124154072485151176192294480158424234268438137612977886891381228704640656094986435381057574477216648567249609280392009533217665484389886",
        )
        .unwrap();
        assert_eq!(b, expected);
    }
}
