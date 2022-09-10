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
/// # Examples
/// 
/// TODO
/// 
/// # Generic Parameters
/// 
/// The const generic parameters will be abbreviated as `BASE` -> `B`, `RoundingMode` -> `R`.
/// THe `BASE` must be in range \[2, isize::MAX\], and the `RoundingMode` can be chosen from
/// the [mode] module.
/// 
/// # Parsing and printing
/// 
/// Conversion from and to [str] is limited to native radix (i.e. base). To print or parse
/// with different radix, please use [to_binary()][FBig::to_binary], [to_decimal()][FBig::to_decimal]
/// or [with_base()][FBig::with_base] to convert.
/// 
/// For detailed requirements of parsing, refer to the [from_str_native()][FBig::from_str_native] method.
/// 
/// # Convert from/to `f32`/`f64`
/// 
/// The conversion between [FBig] and [f32]/[f64] is only defined for base 2 [FBig]. To convert
/// from/to other bases, please first convert to base 2, and then change the base using [with_base()][FBig::with_base].
/// 
/// Converting from [f32]/[f64] (using [TryFrom][core::convert::TryFrom]) is lossless, except for
/// that `NAN` values will result in [Err]. Converting to [f32]/[f64] (using [to_f32()][FBig::to_f32]
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
    /// # Panic
    /// 
    /// Panics if the [Repr] has more digits than the precision limit specified in the context.
    /// Note that this condition is not checked in release build.
    #[inline]
    pub fn from_repr(repr: Repr<B>, context: Context<R>) -> Self {
        debug_assert!(!context.limited() || repr.digits() < context.precision);
        Self { repr, context }
    }

    const fn zero() -> Self {
        Self::new(Repr::zero(), Context::new(0))
    }
    /// [FBig] with value 0 and unlimited precision
    pub const ZERO: Self = Self::zero();

    const fn one() -> Self {
        Self::new(Repr::one(), Context::new(0))
    }
    /// [FBig] with value 1 and unlimited precision
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
    pub const INFINITY: Self = Self::inf();

    const fn neg_inf() -> Self {
        Self::new(Repr::neg_infinity(), Context::new(0))
    }
    /// [FBig] instance representing the negative infinity (-∞)
    pub const NEG_INFINITY: Self = Self::neg_inf();

    /// Determine if the float number is zero
    #[inline]
    pub const fn is_zero(&self) -> bool {
        self.repr.is_zero()
    }
    /// Determine if the float number is one
    #[inline]
    pub const fn is_one(&self) -> bool {
        self.repr.is_one()
    }
    /// Determine if the float number is (±)infinity
    #[inline]
    pub const fn is_infinite(&self) -> bool {
        self.repr.is_infinite()
    }
    /// Determine if the float number is finite
    #[inline]
    pub const fn is_finite(&self) -> bool {
        self.repr.is_finite()
    }

    /// Get the maximum precision set for the float number.
    #[inline]
    pub const fn precision(&self) -> usize {
        self.context.precision
    }

    /// Get the actual precision needed for the float number.
    ///
    /// This is also the actual precision needed for the float number. Shrink to this value using [Self::with_precision] will not cause loss of float precision.
    #[inline]
    pub fn digits(&self) -> usize {
        self.repr.digits()
    }

    /// Get the context associated with the float number
    #[inline]
    pub const fn context(&self) -> Context<R> {
        self.context
    }
    /// Get the reference to the numeric representation
    #[inline]
    pub const fn repr(&self) -> &Repr<B> {
        &self.repr
    }
    /// Get the underlying numeric representation
    #[inline]
    pub fn into_repr(self) -> Repr<B> {
        self.repr
    }

    /// Convert raw parts into a float number, the precision will be inferred from significand
    /// (the lowest k such that `significand <= base^k`)
    #[inline]
    pub fn from_parts(significand: IBig, exponent: isize) -> Self {
        let repr = Repr::new(significand, exponent);
        let precision = repr.digits().max(1); // set precision to 1 if signficand is zero
        let context = Context::new(precision);
        Self::new(repr, context)
    }

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

    /// Convert the float number into raw (signficand, exponent) parts
    #[inline]
    pub fn into_parts(self) -> (IBig, isize) {
        (self.repr.significand, self.repr.exponent)
    }

    /// Return the value of the least significant digit of the float number x,
    /// such that x + ulp is the first float bigger than x (given the precision from the context).
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
            exponent: self.repr.exponent + self.repr.digits() as isize - self.context.precision as isize
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
            context: self.context.clone(),
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
