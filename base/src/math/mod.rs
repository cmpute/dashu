//! Trait definitions for math operations

/// Estimate the bounds of the binary logarithm.
///
/// The result is `(lower bound, upper bound)` such that lower bound ≤ log2(self) ≤ upper bound.
/// The precision of the bounds is at least 8 bits (relative error < 2^-8).
///
/// With `std` disabled, the precision is about 13 bits. With `std` enabled, the precision
/// will be full 23 bits.
/// 
/// For negative values, the logarithm is calculated based on its absolute value.
///
/// # Panics
///
/// Panics if the number is 0
///
/// # Example
///
/// ```
/// use dashu_base::Log2Bounds;
/// 
/// let lb3 = 1.584962500721156f32;
/// let (lb3_lb, lb3_ub) = 3u8.log2_bounds();
/// assert!(lb3_lb <= lb3 && lb3 <= lb3_ub);
/// assert!((lb3 - lb3_lb) / lb3 < 1. / 256.);
/// assert!((lb3_ub - lb3) / lb3 <= 1. / 256.);
/// ```
pub trait Log2Bounds {
    fn log2_bounds(&self) -> (f32, f32);
}

mod log;
