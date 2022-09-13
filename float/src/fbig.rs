use crate::{
    repr::{Context, Repr, Word},
    round::{mode, Round},
};
use dashu_base::Sign;
use dashu_int::{DoubleWord, IBig};

/// An arbitrary precision floating point number with arbitrary base and rounding mode.
///
/// The float number consists of a [Repr] and a [Context]. The [Repr] instance determines
/// the value of the number, and the [Context] contains runtime information (such as precision
/// limit, rounding mode, etc.)
///
/// For how the number is represented, see [Repr], for how the precision limit and rounding
/// mode is applied, see [Context].
///
/// The arithmetic operations on [FBig] follows the behavior of its associated context.
/// If a different precision limit and/or rounding mode is required, or the rounding
/// information has to be preserved, use the methods of the [Context] type.
///
/// # Generic Parameters
///
/// The const generic parameters will be abbreviated as `BASE` -> `B`, `RoundingMode` -> `R`.
/// THe `BASE` must be in range \[2, isize::MAX\], and the `RoundingMode` can be chosen from
/// the [mode] module.
/// 
/// With the default generic parameters, the floating number is of base 2 rounded towards zero.
/// This is the most efficient format for operations. To represent a decimal number, the alias
/// [DBig][crate::DBig] is provided, which is base 10 rounded to the nearest.
///
/// # Parsing and printing
///
/// To create a [FBig] instance, there are four ways:
/// 1. Use predifined constants (e.g. [FBig::ZERO], [FBig::ONE], [FBig::NEG_INFINITY]).
/// 1. Use the literal macro `fbig!` or `dbig!` defined in the [`dashu-macro`](https://docs.rs/dashu-macros/latest/dashu_macros/) crate.
/// 1. Construct from the significand and exponent using [from_parts()][FBig::from_parts] or [from_parts_const()][FBig::from_parts_const].
/// 1. Parse from a string.
/// 
/// Conversion from and to [str] is limited to native radix (i.e. base). To print or parse
/// with different radix, please use [to_binary()][FBig::to_binary], [to_decimal()][FBig::to_decimal]
/// or [with_base()][FBig::with_base], [with_base_and_precision()][FBig::with_base_and_precision] to convert.
///
/// For printing, currently only the [Display][core::fmt::Display] and [Debug][core::fmt::Debug] are supported.
/// Other formatting traits will be supported in future.
/// 
/// ```
/// # use dashu_int::error::ParseError;
/// # use dashu_float::DBig;
/// // parsing
/// let a = DBig::from_parts(123456789.into(), -5);
/// let b = DBig::from_str_native("1234.56789")?;
/// let c = DBig::from_str_native("1.23456789e3")?;
/// assert_eq!(a, b);
/// assert_eq!(b, c);
/// 
/// // printing
/// assert_eq!(format!("{}", DBig::from_str_native("12.34")?), "12.34");
/// let x = DBig::from_str_native("10.01")?
///     .with_precision(0) // use unlimited precision
///     .value(); 
/// if dashu_int::Word::BITS == 64 {
///     // number of digits to display depends on the word size
///     assert_eq!(
///         format!("{:?}", x.powi(100.into())),
///         "1105115697720767968..1441386704950100001 * 10 ^ -200 (prec: 0, rnd: HalfAway)"
///     );
/// }
/// # Ok::<(), ParseError>(())
/// ```
/// 
/// For detailed information of parsing, refer to the [from_str_native()][FBig::from_str_native] method.
///
/// # Binary operations
/// 
/// Binary operations on [FBig] instances are restricted to the same base and same rounding mode. This is
/// designed to make sure that no hidden conversion is performed during the operations. However, for equality
/// test and comparsion, two [FBig] instances can have different rounding modes (but not different bases),
/// because rounding will never happends during comparison.
/// 
/// # Convert from/to `f32`/`f64`
///
/// The conversion between [FBig] and [f32]/[f64] is only defined for base 2 [FBig]. To convert
/// from/to other bases, please first convert to base 2, and then change the base using [with_base()][FBig::with_base]
/// or [with_base_and_precision()][FBig::with_base_and_precision].
///
/// Converting from [f32]/[f64] (using [TryFrom][core::convert::TryFrom]) is lossless, except for
/// that `NAN` values will result in an [Err]. Converting to [f32]/[f64] (using [to_f32()][FBig::to_f32]
/// and [to_f64()][FBig::to_f64]) is lossy, and the rounding direction is contained in the result of these
/// two methods.
///
/// The infinities are converted as it is, and the subnormals are converted using its actual values.
///
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
    /// Panics if the [Repr] has more digits than the precision limit specified in the context.
    /// Note that this condition is not checked in release build.
    #[inline]
    pub fn from_repr(repr: Repr<B>, context: Context<R>) -> Self {
        debug_assert!(repr.is_infinite() || !context.is_limited() || repr.digits() <= context.precision);
        Self { repr, context }
    }

    const fn zero() -> Self {
        Self::new(Repr::zero(), Context::new(0))
    }
    /// [FBig] with value 0 and unlimited precision
    /// 
    /// To test if the float number is zero, use `self.repr().is_zero()`.
    pub const ZERO: Self = Self::zero();

    const fn one() -> Self {
        Self::new(Repr::one(), Context::new(0))
    }
    /// [FBig] with value 1 and unlimited precision
    /// 
    /// To test if the float number is one, use `self.repr().one()`.
    pub const ONE: Self = Self::one();

    const fn neg_one() -> Self {
        Self::new(Repr::neg_one(), Context::new(0))
    }
    /// [FBig] with value -1 and unlimited precision
    pub const NEG_ONE: Self = Self::neg_one();

    const fn inf() -> Self {
        Self::new(Repr::infinity(), Context::new(0))
    }
    /// [FBig] instance representing the positive infinity (+∞)
    /// 
    /// To test if the float number is infinite, use `self.repr().infinite()`.
    pub const INFINITY: Self = Self::inf();

    const fn neg_inf() -> Self {
        Self::new(Repr::neg_infinity(), Context::new(0))
    }
    /// [FBig] instance representing the negative infinity (-∞)
    /// 
    /// To test if the float number is infinite, use `self.repr().infinite()`.
    pub const NEG_INFINITY: Self = Self::neg_inf();

    /// Get the maximum precision set for the float number.
    /// 
    /// It's equivalent to `self.context().precision()`.
    /// 
    /// # Examples
    /// 
    /// ```
    /// # use dashu_int::error::ParseError;
    /// # use dashu_float::DBig;
    /// use dashu_float::Repr;
    /// 
    /// let a = DBig::from_str_native("1.234")?;
    /// assert!(a.repr().significand() <= &Repr::<10>::BASE.pow(a.precision()));
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
    /// # use dashu_int::error::ParseError;
    /// # use dashu_float::DBig;
    /// use dashu_base::Approximation::*;
    /// 
    /// let a = DBig::from_str_native("-1.234e-3")?;
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
    /// # use dashu_int::error::ParseError;
    /// # use dashu_float::DBig;
    /// let a = DBig::from_parts((-1234).into(), -2);
    /// assert_eq!(a, DBig::from_str_native("-12.34")?);
    /// assert_eq!(a.precision(), 4); // 1234 has 4 (decimal) digits
    /// # Ok::<(), ParseError>(())
    /// ```
    #[inline]
    pub fn from_parts(significand: IBig, exponent: isize) -> Self {
        let repr = Repr::new(significand, exponent);
        let precision = repr.digits().max(1); // set precision to 1 if signficand is zero
        let context = Context::new(precision);
        Self::new(repr, context)
    }

    /// Convert raw parts (significand, exponent) into a float number in a `const` context.
    /// 
    /// It requires that the significand fits in a [DoubleWord].
    /// The precision will be inferred from significand (the lowest k such that `significand <= base^k`)
    /// 
    /// # Examples
    /// 
    /// ```
    /// # use dashu_int::error::ParseError;
    /// # use dashu_float::DBig;
    /// use dashu_base::Sign;
    /// 
    /// const A: DBig = DBig::from_parts_const(Sign::Negative, 1234, -2);
    /// assert_eq!(A, DBig::from_str_native("-12.34")?);
    /// assert_eq!(A.precision(), 4); // 1234 has 4 (decimal) digits
    /// # Ok::<(), ParseError>(())
    /// ```
    #[inline]
    pub const fn from_parts_const(
        sign: Sign,
        mut significand: DoubleWord,
        mut exponent: isize,
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
        Self::new(repr, Context::new(digits))
    }

    /// Return the value of the least significant digit of the float number x,
    /// such that `x + ulp` is the first float number greater than x (given the precision from the context).
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_int::error::ParseError;
    /// # use dashu_float::DBig;
    /// assert_eq!(DBig::from_str_native("1.23")?.ulp(), DBig::from_str_native("0.01")?);
    /// assert_eq!(DBig::from_str_native("01.23")?.ulp(), DBig::from_str_native("0.001")?);
    /// # Ok::<(), ParseError>(())
    /// ```
    #[inline]
    pub fn ulp(&self) -> Self {
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
