/// Represent an calculation result with possible error.
pub enum Approximation<T, E> {
    /// The result is exact, contains the result value
    Exact(T),

    /// The result is inexact, contains the result value and error
    Inexact(T, E),
}

impl<T, E> Approximation<T, E> {
    /// Get the value of the calculation regardless of error
    #[inline]
    pub fn value(self) -> T {
        match self {
            Self::Exact(v) => v,
            Self::Inexact(v, _) => v,
        }
    }

    /// Get the reference to the calculation result
    #[inline]
    pub fn value_ref(&self) -> &T {
        match self {
            Self::Exact(v) => v,
            Self::Inexact(v, _) => v,
        }
    }

    #[inline]
    pub fn error(&self) -> Option<&E> {
        match self {
            Self::Exact(_) => None,
            Self::Inexact(_, e) => Some(e),
        }
    }

    #[inline]
    pub fn map<U, F>(self, f: F) -> Approximation<U, E>
    where
        F: FnOnce(T) -> U,
    {
        match self {
            Self::Exact(v) => Approximation::Exact(f(v)),
            Self::Inexact(v, e) => Approximation::Inexact(f(v), e),
        }
    }

    #[inline]
    pub fn and_then<U, F>(self, f: F) -> Approximation<U, E>
    where
        F: FnOnce(T) -> Approximation<U, E>,
    {
        match self {
            Self::Exact(v) => match f(v) {
                Approximation::Exact(v2) => Approximation::Exact(v2),
                Approximation::Inexact(v2, e) => Approximation::Inexact(v2, e)
            },
            Self::Inexact(v, e) => match f(v) {
                Approximation::Exact(v2) => Approximation::Inexact(v2, e),
                Approximation::Inexact(v2, e2) => Approximation::Inexact(v2, e2)
            },
        }
    } 
}
