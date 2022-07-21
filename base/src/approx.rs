/// Represent an calculation result with possible error.
pub enum Approximation<T, E> {
    /// The result is exact, contains the result value
    Exact(T),

    /// The result is inexact, contains the result value and error
    InExact(T, E)
}

impl<T, E> Approximation<T, E> {
    /// Get the value of the calculation regardless of error
    #[inline]
    pub fn value(self) -> T {
        match self {
            Self::Exact(v) => v,
            Self::InExact(v, _) => v
        }
    }
}

impl<T, E: Default> Approximation<T, E> {
    /// Get the error of the calculation. Default is returned if the result is exact.
    #[inline]
    pub fn error(self) -> E {
        match self {
            Self::Exact(_) => E::default(),
            Self::InExact(_, e) => e
        }
    }
}