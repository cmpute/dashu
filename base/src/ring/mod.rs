//! Trait definitions for operations related to rings (integer/polynomial/etc.)

/// Compute quotient and remainder at the same time.
///
/// # Examples
/// ```
/// use dashu_base::DivRem;
/// assert_eq!(23.div_rem(10), (2, 3));
/// ```
pub trait DivRem<Rhs = Self> {
    /// The type of the quotient.
    type OutputDiv;
    /// The type of the remainder.
    type OutputRem;

    /// Compute the quotient and remainder at the same time.
    fn div_rem(self, rhs: Rhs) -> (Self::OutputDiv, Self::OutputRem);
}

/// Compute quotient inplace and return remainder at the same time.
///
/// # Examples
/// ```
/// use dashu_base::DivRemAssign;
/// let mut n = 23;
/// let r = n.div_rem_assign(10);
/// assert!(n == 2 && r == 3);
/// ```
pub trait DivRemAssign<Rhs = Self> {
    /// The type of the remainder.
    type OutputRem;

    /// Divide `self` by `rhs` in place, storing the quotient in `self` and returning the remainder.
    fn div_rem_assign(&mut self, rhs: Rhs) -> Self::OutputRem;
}

/// Compute Euclidean quotient.
///
/// # Examples
/// ```
/// use dashu_base::DivEuclid;
/// assert_eq!((-23).div_euclid(10), -3);
/// ```
pub trait DivEuclid<Rhs = Self> {
    /// The type of the quotient.
    type Output;

    /// Compute the Euclidean quotient of `self / rhs`.
    fn div_euclid(self, rhs: Rhs) -> Self::Output;
}

/// Compute Euclidean remainder.
///
/// # Examples
/// ```
/// use dashu_base::RemEuclid;
/// assert_eq!((-23).rem_euclid(10), 7);
/// ```
pub trait RemEuclid<Rhs = Self> {
    /// The type of the remainder.
    type Output;

    /// Compute the non-negative Euclidean remainder of `self % rhs`.
    fn rem_euclid(self, rhs: Rhs) -> Self::Output;
}

/// Compute Euclidean quotient and remainder at the same time.
///
/// # Examples
/// ```
/// use dashu_base::DivRemEuclid;
/// assert_eq!((-23).div_rem_euclid(10), (-3, 7));
/// ```
pub trait DivRemEuclid<Rhs = Self> {
    /// The type of the quotient.
    type OutputDiv;
    /// The type of the remainder.
    type OutputRem;

    /// Compute the Euclidean quotient and remainder at the same time.
    fn div_rem_euclid(self, rhs: Rhs) -> (Self::OutputDiv, Self::OutputRem);
}

/// Exact division, re-exported from [`num-modular`](https://docs.rs/num-modular).
///
/// `DivExact<Rhs, Precompute>::div_exact(self, rhs, pre)` returns `Some(self / rhs)` when `rhs`
/// divides `self` exactly and `None` otherwise. `dashu`'s implementations use the empty
/// precomputation `Precompute = ()` (pass `&()` at the call site). For arbitrary-precision types an
/// exact division avoids the general division's normalization and remainder computation when the
/// divisor is small (e.g. `dashu-int` uses Hensel 2-adic division).
pub use num_modular::{DivExact, DivExactAssign};

/// Compute the greatest common divisor.
///
/// For negative integers, the common divisor is still kept positive.
///
/// # Examples
/// ```
/// use dashu_base::Gcd;
/// assert_eq!(12u8.gcd(10u8), 2);
/// ```
///
/// # Panics
///
/// Panics if both operands are zeros
pub trait Gcd<Rhs = Self> {
    /// The type of the greatest common divisor.
    type Output;

    /// Compute the greatest common divisor between the two operands.
    ///
    /// Panics if both operands are zeros
    fn gcd(self, rhs: Rhs) -> Self::Output;
}

/// Compute the greatest common divisor between self and the other operand, and return
/// both the common divisor `g` and the Bézout coefficients respectively.
///
/// For negative integers, the common divisor is still kept positive.
///
/// # Examples
/// ```
/// use dashu_base::{Gcd, ExtendedGcd};
/// let (g, cx, cy) = 12u8.gcd_ext(10u8);
/// assert_eq!(g, 12u8.gcd(10u8));
/// assert_eq!(g as i8, 12 * cx + 10 * cy);
/// ```
///
/// # Panics
///
/// Panics if both operands are zeros
pub trait ExtendedGcd<Rhs = Self> {
    /// The type of the greatest common divisor.
    type OutputGcd;
    /// The type of the Bézout coefficients.
    type OutputCoeff;

    /// Calculate the greatest common divisor between the two operands, returns
    /// the common divisor `g` and the Bézout coefficients respectively.
    ///
    /// Panics if both operands are zeros
    fn gcd_ext(self, rhs: Rhs) -> (Self::OutputGcd, Self::OutputCoeff, Self::OutputCoeff);
}

/// Computer the floored square root of the number and return the remainder at the same time.
pub trait SquareRootRem {
    /// The type of the (floored) root and the remainder.
    type Output;

    /// Compute the floored square root together with the remainder, so that
    /// `root*root + rem == *self` and `0 <= rem <= 2*root`.
    fn sqrt_rem(&self) -> (Self::Output, Self);
}

/// Computer the floored cubic root of the number and return the remainder at the same time.
pub trait CubicRootRem {
    /// The type of the (floored) root and the remainder.
    type Output;

    /// Compute the floored cubic root together with the remainder, so that
    /// `root*root*root + rem == *self` and `0 <= rem < 3*root*root + 3*root`.
    fn cbrt_rem(&self) -> (Self::Output, Self);
}

mod div_rem;
mod gcd;
mod root;
pub(crate) use root::NormalizedRootRem;
