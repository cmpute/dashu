use crate::{
    error::panic_unlimited_precision,
    repr::{Context, Repr, Word},
    round::{mode, Round},
    utils::digit_len,
};
use dashu_base::Sign;
use dashu_int::{DoubleWord, IBig};

/// An arbitrary precision floating point number with arbitrary base and rounding mode.
///
/// An `FBig` is a [`Repr`] (the value: significand × base<sup>exponent</sup>) paired with a
/// [`Context`] (the precision cap and rounding mode). Arithmetic follows the associated context;
/// use the [`Context`] methods directly when you need a different precision/rounding, or to receive
/// the rounding direction and errors instead of a panic.
///
/// The generic parameters are `BASE` (`B`, in `[2, isize::MAX]`) and `RoundingMode` (`R`, chosen from
/// the [`mode`] module). With the defaults the number is base 2 rounded towards zero (the most
/// efficient format); [`DBig`](crate::DBig) aliases base 10 rounded to nearest.
///
/// Binary operators require both operands to share the same base and rounding mode (no hidden
/// conversion is performed); comparison allows differing rounding modes but not differing bases.
///
/// See the [user guide](https://github.com/cmpute/dashu/blob/master/guide/src/types.md) for the
/// memory layout, and the
/// [construction](https://github.com/cmpute/dashu/blob/master/guide/src/construct.md),
/// [parsing & printing](https://github.com/cmpute/dashu/blob/master/guide/src/io/parse.md),
/// [IEEE 754 compliance](https://github.com/cmpute/dashu/blob/master/guide/src/compliance.md), and
/// [conversion](https://github.com/cmpute/dashu/blob/master/guide/src/convert.md) pages for those
/// topics. (Notably: `FBig` has no NaN, supports IEEE-754 signed zero, and treats infinities as
/// terminal values.) The accepted string format is documented on the [`core::str::FromStr`] impl.
///
/// # Examples
///
/// ```
/// # use dashu_base::ParseError;
/// # use dashu_float::DBig;
/// use core::str::FromStr;
///
/// // parsing
/// let a = DBig::from_parts(123456789.into(), -5);
/// let b = DBig::from_str("1234.56789")?;
/// let c = DBig::from_str("1.23456789e3")?;
/// assert_eq!(a, b);
/// assert_eq!(b, c);
///
/// // printing
/// assert_eq!(format!("{}", DBig::from_str("12.34")?), "12.34");
/// let x = DBig::from_str("10.01")?
///     .with_precision(0) // use unlimited precision
///     .value();
/// if dashu_int::Word::BITS == 64 {
///     // number of digits to display depends on the word size
///     assert_eq!(
///         format!("{:?}", x.powi(100.into())),
///         "1105115697720767968..1441386704950100001 * 10 ^ -200 (prec: 0)"
///     );
/// }
/// # Ok::<(), ParseError>(())
/// ```
pub struct FBig<RoundingMode: Round = mode::Zero, const BASE: Word = 2> {
    pub(crate) repr: Repr<BASE>,
    pub(crate) context: Context<RoundingMode>,
}

impl<R: Round, const B: Word> FBig<R, B> {
    /// Create a [FBig] instance from raw parts, internal use only
    #[inline]
    pub(crate) const fn new(repr: Repr<B>, context: Context<R>) -> Self {
        Self { repr, context }
    }

    /// Create a [FBig] instance from [Repr] and [Context].
    ///
    /// This method should not be used in most cases. It's designed to be used when
    /// you hold a [Repr] instance and want to create an [FBig] from that.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_float::DBig;
    /// use dashu_float::{Repr, Context};
    ///
    /// assert_eq!(DBig::from_repr(Repr::one(), Context::new(1)), DBig::ONE);
    /// assert_eq!(DBig::from_repr(Repr::infinity(), Context::new(1)), DBig::INFINITY);
    /// ```
    ///
    /// # Panics
    ///
    /// Panics if the [Repr] has more digits than `precision + 1` (the one allowed guard digit from
    /// an inexact add/sub — see [`Repr`]). Note that this condition is not checked in release builds.
    #[inline]
    pub fn from_repr(repr: Repr<B>, context: Context<R>) -> Self {
        debug_assert!(
            repr.is_infinite() || !context.is_limited() || repr.digits() <= context.precision + 1
        );
        Self { repr, context }
    }

    /// Create a [FBig] instance from [Repr]. Due to the limitation of const operations,
    /// the precision of the float is set to unlimited.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_float::DBig;
    /// use dashu_float::{Repr, Context};
    ///
    /// assert_eq!(DBig::from_repr_const(Repr::one()), DBig::ONE);
    /// assert_eq!(DBig::from_repr_const(Repr::infinity()), DBig::INFINITY);
    /// ```
    #[inline]
    pub const fn from_repr_const(repr: Repr<B>) -> Self {
        Self {
            repr,
            context: Context::new(0),
        }
    }

    /// [FBig] with value 0 and unlimited precision
    ///
    /// To test if the float number is zero, use `self.repr().is_zero()`.
    pub const ZERO: Self = Self::new(Repr::zero(), Context::new(0));

    /// [FBig] with value 1 and unlimited precision
    ///
    /// To test if the float number is one, use `self.repr().is_one()`.
    pub const ONE: Self = Self::new(Repr::one(), Context::new(0));

    /// [FBig] with value -1 and unlimited precision
    pub const NEG_ONE: Self = Self::new(Repr::neg_one(), Context::new(0));

    /// [FBig] instance representing the positive infinity (+∞)
    ///
    /// To test if the float number is infinite, use `self.repr().infinite()`.
    pub const INFINITY: Self = Self::new(Repr::infinity(), Context::new(0));

    /// [FBig] instance representing the negative infinity (-∞)
    ///
    /// To test if the float number is infinite, use `self.repr().infinite()`.
    pub const NEG_INFINITY: Self = Self::new(Repr::neg_infinity(), Context::new(0));

    /// Get the maximum precision set for the float number.
    ///
    /// It's equivalent to `self.context().precision()`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use core::str::FromStr;
    /// # use dashu_base::ParseError;
    /// # use dashu_float::DBig;
    /// # use dashu_int::IBig;
    /// use dashu_float::Repr;
    ///
    /// let a = DBig::from_str("1.234")?;
    /// assert!(a.repr().significand() <= &IBig::from(10).pow(a.precision()));
    /// # Ok::<(), ParseError>(())
    /// ```
    #[inline]
    pub const fn precision(&self) -> usize {
        self.context.precision
    }

    /// Get the number of the significant digits in the float number
    ///
    /// It's equivalent to `self.repr().digits()`.
    ///
    /// This value is also the actual precision needed for the float number. Shrink to this
    /// value using [with_precision()][FBig::with_precision] will not cause loss of float precision.
    ///
    /// # Examples
    ///
    /// ```
    /// # use core::str::FromStr;
    /// # use dashu_base::ParseError;
    /// # use dashu_float::DBig;
    /// use dashu_base::Approximation::*;
    ///
    /// let a = DBig::from_str("-1.234e-3")?;
    /// assert_eq!(a.digits(), 4);
    /// assert!(matches!(a.clone().with_precision(4), Exact(_)));
    /// assert!(matches!(a.clone().with_precision(3), Inexact(_, _)));
    /// # Ok::<(), ParseError>(())
    /// ```
    #[inline]
    pub fn digits(&self) -> usize {
        self.repr.digits()
    }

    /// Get the context associated with the float number
    #[inline]
    pub const fn context(&self) -> Context<R> {
        self.context
    }
    /// Get a reference to the underlying numeric representation
    #[inline]
    pub const fn repr(&self) -> &Repr<B> {
        &self.repr
    }
    /// Get the underlying numeric representation
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_float::DBig;
    /// use dashu_float::Repr;
    ///
    /// let a = DBig::ONE;
    /// assert_eq!(a.into_repr(), Repr::<10>::one());
    /// ```
    #[inline]
    pub fn into_repr(self) -> Repr<B> {
        self.repr
    }

    /// Convert raw parts (significand, exponent) into a float number.
    ///
    /// The precision will be inferred from significand (the lowest k such that `significand <= base^k`)
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_base::ParseError;
    /// # use dashu_float::DBig;
    /// use core::str::FromStr;
    /// let a = DBig::from_parts((-1234).into(), -2);
    /// assert_eq!(a, DBig::from_str("-12.34")?);
    /// assert_eq!(a.precision(), 4); // 1234 has 4 (decimal) digits
    /// # Ok::<(), ParseError>(())
    /// ```
    #[inline]
    pub fn from_parts(significand: IBig, exponent: isize) -> Self {
        let precision = digit_len::<B>(&significand).max(1); // set precision to 1 if signficand is zero
        let repr = Repr::new(significand, exponent);
        let context = Context::new(precision);
        Self::new(repr, context)
    }

    /// Convert raw parts (significand, exponent) into a float number in a `const` context.
    ///
    /// It requires that the significand fits in a [DoubleWord].
    ///
    /// The precision will be inferred from significand (the lowest k such that `significand <= base^k`).
    /// If the `min_precision` is provided, then the higher one from the given and inferred precision
    /// will be used as the final precision.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_base::ParseError;
    /// # use dashu_float::DBig;
    /// use core::str::FromStr;
    /// use dashu_base::Sign;
    ///
    /// const A: DBig = DBig::from_parts_const(Sign::Negative, 1234, -2, None);
    /// assert_eq!(A, DBig::from_str("-12.34")?);
    /// assert_eq!(A.precision(), 4); // 1234 has 4 (decimal) digits
    ///
    /// const B: DBig = DBig::from_parts_const(Sign::Negative, 1234, -2, Some(5));
    /// assert_eq!(B.precision(), 5); // overrided by the argument
    /// # Ok::<(), ParseError>(())
    /// ```
    #[inline]
    pub const fn from_parts_const(
        sign: Sign,
        mut significand: DoubleWord,
        mut exponent: isize,
        min_precision: Option<usize>,
    ) -> Self {
        if significand == 0 {
            return Self::ZERO;
        }

        let mut digits = 0;

        // normalize
        if B.is_power_of_two() {
            let base_bits = B.trailing_zeros();
            let shift = significand.trailing_zeros() / base_bits;
            significand >>= shift * base_bits;
            exponent += shift as isize;
            digits = ((DoubleWord::BITS - significand.leading_zeros() + base_bits - 1) / base_bits)
                as usize;
        } else {
            let mut pow: DoubleWord = 1;
            while significand % (B as DoubleWord) == 0 {
                significand /= B as DoubleWord;
                exponent += 1;
            }
            while let Some(next) = pow.checked_mul(B as DoubleWord) {
                digits += 1;
                if next > significand {
                    break;
                }
                pow = next;
            }
        }

        let repr = Repr {
            significand: IBig::from_parts_const(sign, significand),
            exponent,
        };
        let precision = match min_precision {
            Some(prec) => {
                if prec > digits {
                    prec
                } else {
                    digits
                }
            }
            None => digits,
        };
        Self::new(repr, Context::new(precision))
    }

    /// Return the value of the least significant digit of the float number x,
    /// such that `x + ulp` is the first float number greater than x (given the precision from the context).
    ///
    /// # Examples
    ///
    /// ```
    /// # use core::str::FromStr;
    /// # use dashu_base::ParseError;
    /// # use dashu_float::DBig;
    /// assert_eq!(DBig::from_str("1.23")?.ulp(), DBig::from_str("0.01")?);
    /// assert_eq!(DBig::from_str("01.23")?.ulp(), DBig::from_str("0.001")?);
    /// # Ok::<(), ParseError>(())
    /// ```
    ///
    /// # Panics
    /// Panics if the precision of the number is 0 (unlimited).
    ///
    #[inline]
    pub fn ulp(&self) -> Self {
        if self.context.precision == 0 {
            panic_unlimited_precision();
        }
        if self.repr.is_infinite() {
            return self.clone();
        }

        let repr = Repr {
            significand: IBig::ONE,
            exponent: self.repr.exponent + self.repr.digits() as isize
                - self.context.precision as isize,
        };
        Self::new(repr, self.context)
    }

    /// Similar to [FBig::ulp], but use approximated digits. It's guaranteed to be smaller than ulp(), for internal use only.
    #[inline]
    pub(crate) fn sub_ulp(&self) -> Self {
        debug_assert!(self.context.precision != 0);
        debug_assert!(self.repr.is_finite());

        let repr = Repr {
            significand: IBig::ONE,
            exponent: self.repr.exponent + self.repr.digits_lb() as isize
                - self.context.precision as isize
                - 1,
        };
        Self::new(repr, self.context)
    }
}

// This custom implementation is necessary due to https://github.com/rust-lang/rust/issues/98374
impl<R: Round, const B: Word> Clone for FBig<R, B> {
    #[inline]
    fn clone(&self) -> Self {
        Self {
            repr: self.repr.clone(),
            context: self.context,
        }
    }

    #[inline]
    fn clone_from(&mut self, source: &Self) {
        self.repr.clone_from(&source.repr);
        self.context = source.context;
    }
}

impl<R: Round, const B: Word> Default for FBig<R, B> {
    /// Default value: 0.
    #[inline]
    fn default() -> Self {
        Self::ZERO
    }
}
