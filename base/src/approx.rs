//! Trait definitions for approximated values

/// Represent an calculation result with a possible error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

    /// Get a reference to the calculation result
    #[inline]
    pub const fn value_ref(&self) -> &T {
        match self {
            Self::Exact(v) => v,
            Self::Inexact(v, _) => v,
        }
    }

    /// Return the value if the result is exact, panic otherwise.
    #[inline]
    pub fn unwrap(self) -> T {
        match self {
            Self::Exact(val) => val,
            Self::Inexact(_, _) => panic!("called `Approximation::unwrap()` on a `Inexact` value"),
        }
    }

    /// Return the error if the result is inexact, [`None`] if it is exact.
    #[inline]
    pub fn error(self) -> Option<E> {
        match self {
            Self::Exact(_) => None,
            Self::Inexact(_, e) => Some(e),
        }
    }

    /// Borrow the error if the result is inexact, [`None`] if it is exact.
    #[inline]
    pub const fn error_ref(&self) -> Option<&E> {
        match self {
            Self::Exact(_) => None,
            Self::Inexact(_, e) => Some(e),
        }
    }

    /// Map the result value to a new type, preserving the error (if any).
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

    /// Chain a fallible mapping that itself returns an [`Approximation`], combining the
    /// errors: an inexact input or an inexact result both yield an inexact result.
    #[inline]
    pub fn and_then<U, F>(self, f: F) -> Approximation<U, E>
    where
        F: FnOnce(T) -> Approximation<U, E>,
    {
        match self {
            Self::Exact(v) => match f(v) {
                Approximation::Exact(v2) => Approximation::Exact(v2),
                Approximation::Inexact(v2, e) => Approximation::Inexact(v2, e),
            },
            Self::Inexact(v, e) => match f(v) {
                Approximation::Exact(v2) => Approximation::Inexact(v2, e),
                Approximation::Inexact(v2, e2) => Approximation::Inexact(v2, e2),
            },
        }
    }
}
