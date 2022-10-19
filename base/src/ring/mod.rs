//! Trait definitions for operations related to rings (integer/polynomial/etc.)

/// Compute quotient and remainder at the same time.
///
/// # Examples
/// ```
/// use dashu_base::DivRem;
/// assert_eq!(23.div_rem(10), (2, 3));
/// ```
pub trait DivRem<Rhs = Self> {
    type OutputDiv;
    type OutputRem;

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
    type OutputRem;

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
    type Output;

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
    type Output;

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
    type OutputDiv;
    type OutputRem;

    fn div_rem_euclid(self, rhs: Rhs) -> (Self::OutputDiv, Self::OutputRem);
}

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
    type OutputGcd;
    type OutputCoeff;

    /// Calculate the greatest common divisor between the two operands, returns
    /// the common divisor `g` and the Bézout coefficients respectively.
    ///
    /// Panics if both operands are zeros
    fn gcd_ext(self, rhs: Rhs) -> (Self::OutputGcd, Self::OutputCoeff, Self::OutputCoeff);
}


// TODO: more docs
/// Compute the roots (square root, cubic root) of an integer.
pub trait Root {
    type Output;

    fn sqrt(&self) -> Self::Output;
    fn cbrt(&self) -> Self::Output;
}

/// Compute the roots (square root, cubic root) of an integer, and returns the remainder of the root as well.
pub trait RootRem {
    type Output; // TODO(v0.3): Separate root type and remainder type

    // TODO(v0.3): remove in the next version
    #[deprecated(note = "this function is never supported and will be removed in the next version")]
    fn nth_root_rem(self, n: usize) -> (Self::Output, Self::Output);

    // TODO(v0.3): taking reference instead
    fn sqrt_rem(self) -> (Self::Output, Self::Output);
    fn cbrt_rem(self) -> (Self::Output, Self::Output);
}

// TODO(v0.3): Create a Root trait, but only has function for sqrt and cbrt

mod div_rem;
mod gcd;
mod root;
