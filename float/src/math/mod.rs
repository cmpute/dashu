//! Advanced mathematical functions

use crate::{
    error::{panic_infinite, panic_nan, panic_overflow, panic_underflow},
    fbig::FBig,
    repr::{Context, Repr, Word},
    round::{Round, Rounded},
};

pub mod consts;
pub mod trig;

/// The result of an advanced mathematical operation.
///
/// This enum is used to handle non-finite results (NaN, Infinite) and
/// boundary conditions (Overflow, Underflow) without panicking,
/// as the core [`FBig`] type only represents finite numbers.
///
/// Finite results are wrapped in a [Rounded] to preserve rounding information.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FpResult<const B: Word> {
    Normal(Rounded<Repr<B>>),
    Overflow,
    Underflow,
    NaN,
    /// An exact infinite result is obtained from finite inputs, such as
    /// divide by zero or logarithm of zero.
    Infinite,
}

impl<const B: Word> FpResult<B> {
    /// Convert the result into an [`FBig`] with the given context.
    ///
    /// # Panics
    /// Panics if the result is not `Normal`.
    #[inline]
    #[must_use]
    pub fn value<R: Round>(self, context: &Context<R>) -> FBig<R, B> {
        match self {
            Self::Normal(rounded) => FBig::new(rounded.value(), *context),
            Self::NaN => panic_nan(),
            Self::Infinite => panic_infinite(),
            Self::Overflow => panic_overflow(),
            Self::Underflow => panic_underflow(),
        }
    }

    /// Convert the result into an optional [`FBig`] with the given context.
    /// Returns `None` if the result is not `Normal`.
    #[inline]
    #[must_use]
    pub fn ok<R: Round>(self, context: &Context<R>) -> Option<Rounded<FBig<R, B>>> {
        match self {
            Self::Normal(rounded) => Some(rounded.map(|repr| FBig::new(repr, *context))),
            _ => None,
        }
    }

    /// Returns `true` if the result is `NaN`.
    #[inline]
    #[must_use]
    pub const fn is_nan(&self) -> bool {
        matches!(self, Self::NaN)
    }

    /// Returns `true` if the result is `Infinite`.
    #[inline]
    #[must_use]
    pub const fn is_infinite(&self) -> bool {
        matches!(self, Self::Infinite)
    }

    /// Returns `true` if the result is a normal finite value.
    #[inline]
    #[must_use]
    pub const fn is_normal(&self) -> bool {
        matches!(self, Self::Normal(_))
    }

    /// Returns `true` if the result is a finite value (Normal, Overflow, or Underflow).
    #[inline]
    #[must_use]
    pub const fn is_finite(&self) -> bool {
        matches!(self, Self::Normal(_) | Self::Overflow | Self::Underflow)
    }
}
