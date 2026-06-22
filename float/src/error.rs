use crate::repr::{Repr, Word};
use crate::round::Rounded;
use core::fmt::{self, Display, Formatter};

/// Error returned by floating-point operations that cannot produce a usable result.
///
/// Infinite *outputs* are **not** errors — they are returned as legitimate values inside the
/// `Ok` variant (e.g. `1 / 0 → +inf`, `ln(0) → -inf`, `tan(π/2) → +inf`). This type only
/// signals that an operation could not proceed: an infinite input was supplied to an operation
/// that does not consume infinities, or the mathematical result is not a real number.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FpError {
    /// An operand was infinite. Infinities are terminal values: they can be produced and
    /// compared, but not fed back into arithmetic.
    InfiniteInput,

    /// The mathematical result is not a real number (domain error), e.g. `sqrt(-x)` for `x > 0`,
    /// `ln(-x)`, `asin(|x| > 1)`, `pow(negative, non-integer)`, an even root of a negative value.
    OutOfDomain,

    /// An indeterminate form, e.g. `0 / 0`.
    Indeterminate,
}

impl Display for FpError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            FpError::InfiniteInput => {
                f.write_str("arithmetic with an infinite input is not allowed")
            }
            FpError::OutOfDomain => f.write_str("the operation result is out of domain"),
            FpError::Indeterminate => f.write_str("the operation result is an indeterminate form"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for FpError {}

/// The result of a floating point operation: a correctly-rounded value (which may be an
/// infinity produced as a value), or an [`FpError`] when the operation cannot proceed.
pub type FpResult<T> = Result<Rounded<T>, FpError>;

/// Unwrap an [`FpResult`], panicking with a granular message for each error variant. Used by
/// the `FBig`/`CachedFBig` convenience layer, which exposes a panic-on-error API.
#[inline]
pub(crate) fn unwrap_fp<T>(result: FpResult<T>) -> Rounded<T> {
    match result {
        Ok(value) => value,
        Err(FpError::InfiniteInput) => panic_operate_with_inf(),
        Err(FpError::OutOfDomain) => panic_out_of_domain(),
        Err(FpError::Indeterminate) => panic_nan(),
    }
}

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
