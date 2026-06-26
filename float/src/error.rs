use dashu_base::Sign;
use dashu_int::Word;

use crate::fbig::FBig;
use crate::repr::{Context, Repr};
use crate::round::{Round, Rounded};
use core::fmt::{self, Display, Formatter};

/// Error returned by floating-point operations that cannot produce a usable result.
///
/// # Errors vs. special values
///
/// Infinite *outputs* (e.g. `1/0 → +inf`, `ln(0) → -inf`) are **not** errors — they are
/// legitimate [`Exact`] values produced by operations whose mathematical result is genuinely
/// infinite. Overflow and underflow are distinct: the mathematical result is finite, but its
/// magnitude exceeds the representable exponent range. These are reported as
/// [`Overflow`](FpError::Overflow) / [`Underflow`](FpError::Underflow), and converted to
/// signed infinity / signed zero at the convenience layer via `Context::unwrap_fp` (or the
/// `Repr`-level counterpart `Context::unwrap_fp_repr`). Because the true result was finite,
/// the converted value is always [`Inexact`] with `Rounding::NoOp`.
///
/// The remaining variants ([`InfiniteInput`](FpError::InfiniteInput),
/// [`OutOfDomain`](FpError::OutOfDomain), [`Indeterminate`](FpError::Indeterminate)) signal
/// that an operation could not proceed, and always panic at the convenience layer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FpError {
    /// An operand was infinite. Infinities are terminal values: they can be produced and
    /// compared, but not fed back into arithmetic.
    InfiniteInput,

    /// The mathematical result is not a real number (domain error), e.g. `sqrt(-x)` for `x > 0`,
    /// `ln(-x)`, `asin(|x| > 1)`, `pow(negative, non-integer)`, an even root of a negative value.
    OutOfDomain,

    /// An indeterminate form, e.g. `0 / 0`. Only a *zero* divided by zero is
    /// indeterminate — a non-zero value divided by zero yields ±infinity, which is a
    /// legitimate [`Exact`] value rather than an error.
    Indeterminate,

    /// The result magnitude is too large to represent as a finite number.
    ///
    /// At the `FBig` convenience layer this is converted to a signed infinity via
    /// `Context::unwrap_fp` (or to a signed [`Repr`] via `Context::unwrap_fp_repr`).
    /// The converted result is always [`Inexact`]: the true result was a very large
    /// finite number, and infinity is an approximation.
    Overflow(Sign),

    /// The result magnitude is too small to represent as a finite non-zero number.
    ///
    /// At the `FBig` convenience layer this is converted to a signed zero via
    /// `Context::unwrap_fp` (or to a signed [`Repr`] via `Context::unwrap_fp_repr`).
    /// The converted result is always [`Inexact`]: the true result was a very small
    /// non-zero number, and zero is an approximation.
    Underflow(Sign),
}

impl Display for FpError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            FpError::InfiniteInput => {
                f.write_str("arithmetic with an infinite input is not allowed")
            }
            FpError::OutOfDomain => f.write_str("the operation result is out of domain"),
            FpError::Indeterminate => f.write_str("the operation result is an indeterminate form"),
            FpError::Overflow(_) => f.write_str("overflow: the result is too large to represent"),
            FpError::Underflow(_) => f.write_str("underflow: the result is too small to represent"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for FpError {}

/// The result of a floating point operation: a correctly-rounded value (which may be an
/// infinity produced as a value), or an [`FpError`] when the operation cannot proceed.
pub type FpResult<T> = Result<Rounded<T>, FpError>;

#[inline]
pub const fn assert_finite<const B: Word>(repr: &Repr<B>) {
    if repr.is_infinite() {
        panic_operate_with_inf()
    }
}

#[inline]
pub const fn assert_finite_operands<const B: Word>(lhs: &Repr<B>, rhs: &Repr<B>) {
    if lhs.is_infinite() || rhs.is_infinite() {
        panic_operate_with_inf()
    }
}

/// Panics when operate with infinities
pub const fn panic_operate_with_inf() -> ! {
    panic!("arithmetic operations with the infinity are not allowed!")
}

/// Panics if precision is set to 0
pub const fn assert_limited_precision(precision: usize) {
    if precision == 0 {
        panic_unlimited_precision()
    }
}

/// Panics when operate on unlimited precision number
pub const fn panic_unlimited_precision() -> ! {
    panic!("precision cannot be 0 (unlimited) for this operation!")
}

/// Panics when taking the zeroth root of a number
pub fn panic_root_zeroth() -> ! {
    panic!("finding 0th root is not allowed!")
}

/// Panics when the result of an operation is NaN
pub fn panic_nan() -> ! {
    panic!("the result of the operation is NaN!")
}

/// Panics when an operation is out of domain (e.g. sqrt of a negative number)
pub fn panic_out_of_domain() -> ! {
    panic!("the operation result is out of domain!")
}

impl<R: Round> Context<R> {
    /// Unwrap an [`FpResult`], returning the value directly.
    ///
    /// Converts [`Overflow`](FpError::Overflow) to a signed infinity and
    /// [`Underflow`](FpError::Underflow) to a signed zero. All other error
    /// variants panic (infinite input, out-of-domain, indeterminate).
    #[inline]
    pub fn unwrap_fp<const B: Word>(&self, result: FpResult<FBig<R, B>>) -> FBig<R, B> {
        match result {
            Ok(value) => value.value(),
            Err(FpError::Overflow(sign)) => FBig::new(Repr::infinity_with_sign(sign), *self),
            Err(FpError::Underflow(sign)) => FBig::new(Repr::zero_with_sign(sign), *self),
            Err(FpError::InfiniteInput) => panic_operate_with_inf(),
            Err(FpError::OutOfDomain) => panic_out_of_domain(),
            Err(FpError::Indeterminate) => panic_nan(),
        }
    }

    /// Unwrap an [`FpResult`] at the [`Repr`] level, returning the [`Repr`] directly.
    ///
    /// Converts [`Overflow`](FpError::Overflow) / [`Underflow`](FpError::Underflow) to
    /// signed infinity / signed zero; panics on all other error variants.
    #[inline]
    pub(crate) fn unwrap_fp_repr<const B: Word>(&self, result: FpResult<Repr<B>>) -> Repr<B> {
        match result {
            Ok(value) => value.value(),
            Err(FpError::Overflow(sign)) => Repr::infinity_with_sign(sign),
            Err(FpError::Underflow(sign)) => Repr::zero_with_sign(sign),
            Err(FpError::InfiniteInput) => panic_operate_with_inf(),
            Err(FpError::OutOfDomain) => panic_out_of_domain(),
            Err(FpError::Indeterminate) => panic_nan(),
        }
    }
}
