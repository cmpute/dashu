//! Trait definitions for operations on general numbers

/// Absolute value.
///
/// # Examples
/// ```
/// use dashu_base::Abs;
/// assert_eq!((-5).abs(), 5);
/// ```
pub trait Abs {
    type Output;

    fn abs(self) -> Self::Output;
}

/// Unsigned absolute value.
///
/// # Examples
/// ```
/// use dashu_base::UnsignedAbs;
/// assert_eq!((-5i8).unsigned_abs(), 5u8);
/// ```
pub trait UnsignedAbs {
    type Output;

    fn unsigned_abs(self) -> Self::Output;
}

/// Next power of two.
///
/// # Examples
/// ```
/// # use dashu_base::NextPowerOfTwo;
/// assert_eq!(5.next_power_of_two(), 8);
/// ```
pub trait PowerOfTwo {
    type Output;

    fn next_power_of_two(self) -> Self::Output;
    // TODO: is_power_of_two(self)
}
