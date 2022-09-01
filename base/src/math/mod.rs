//! Trait definitions for math operations

/// Fast estimation of the binary logarithm of a number
pub trait EstimatedLog2 {
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
    /// use dashu_base::EstimatedLog2;
    ///
    /// let lb3 = 1.584962500721156f32;
    /// let (lb3_lb, lb3_ub) = 3u8.log2_bounds();
    /// assert!(lb3_lb <= lb3 && lb3 <= lb3_ub);
    /// assert!((lb3 - lb3_lb) / lb3 < 1. / 256.);
    /// assert!((lb3_ub - lb3) / lb3 <= 1. / 256.);
    /// ```
    fn log2_bounds(&self) -> (f32, f32);

    /// Estimate the value of the binary logarithm. It's calculated as the
    /// average of [log2_bounds][EstimatedLog2::log2_bounds] by default.
    #[inline]
    fn log2_est(&self) -> f32 {
        let (lb, ub) = self.log2_bounds();
        (lb + ub) / 2.
    }
}

mod log;
