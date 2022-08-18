use crate::{
    ibig_ext::remove_pow,
    round::{mode, Round, Rounding},
    utils::{base_as_ibig, digit_len, shr_rem_radix_in_place},
};
use core::marker::PhantomData;
use dashu_base::Approximation;
use dashu_int::{DoubleWord, IBig, Sign, Word};

#[derive(PartialEq, Eq)]
pub struct Repr<const BASE: Word> {
    pub(crate) significand: IBig,
    pub(crate) exponent: isize,
}

#[derive(Clone, Copy)]
pub struct Context<RoundingMode: Round> {
    // TODO: let precision = 0 implies no precision bound, but when no-precision number operates with another has-precision number, the precision will be set as the other one's. This will requires us to make sure 0 value also has non-zero precision (1 will be ideal)
    pub(crate) precision: usize,
    pub(crate) _marker: PhantomData<RoundingMode>,
}

impl<const B: Word> Repr<B> {
    pub const BASE: IBig = base_as_ibig::<B>();

    pub const fn zero() -> Self {
        Self {
            significand: IBig::ZERO,
            exponent: 0,
        }
    }
    pub const fn one() -> Self {
        Self {
            significand: IBig::ONE,
            exponent: 0,
        }
    }
    pub const fn neg_one() -> Self {
        Self {
            significand: IBig::NEG_ONE,
            exponent: 0,
        }
    }
    pub const fn infinity() -> Self {
        Self {
            significand: IBig::ZERO,
            exponent: 1,
        }
    }
    pub const fn neg_infinity() -> Self {
        Self {
            significand: IBig::ZERO,
            exponent: -1,
        }
    }
    pub const fn is_zero(&self) -> bool {
        self.significand.is_zero() && self.exponent == 0
    }
    pub const fn is_one(&self) -> bool {
        self.significand.is_one() && self.exponent == 0
    }
    pub const fn is_infinite(&self) -> bool {
        self.significand.is_zero() && self.exponent != 0
    }
    pub const fn is_finite(&self) -> bool {
        !self.is_infinite()
    }

    pub fn normalize(self) -> Self {
        use core::convert::TryInto;
        let Self {
            mut significand,
            mut exponent,
        } = self;
        if significand.is_zero() {
            return Self::zero();
        }
        if B == 2 {
            if let Some(shift) = significand.trailing_zeros() {
                significand >>= shift;
                exponent += shift as isize;
            };
        } else {
            let shift: isize = remove_pow(&mut significand, &B.into()).try_into().unwrap();
            exponent += shift;
        }
        Self {
            significand,
            exponent,
        }
    }

    /// Get the number of digits in the significand.
    #[inline]
    pub fn digits(&self) -> usize {
        digit_len::<B>(&self.significand)
    }

    /// Fast over estimation of [digits][Self::digits]
    #[inline]
    pub fn digits_ub(&self) -> usize {
        (self.significand.log2_bounds().1 / Self::BASE.log2_bounds().0) as usize + 1
    }

    /// Create a [Repr] from significand and exponent. This
    /// constructor will normalize the representation.
    #[inline]
    pub fn new(significand: IBig, exponent: isize) -> Self {
        Self {
            significand,
            exponent,
        }
        .normalize()
    }
}

// This custom implementation is necessary due to https://github.com/rust-lang/rust/issues/98374
impl<const B: Word> Clone for Repr<B> {
    #[inline]
    fn clone(&self) -> Self {
        Self {
            significand: self.significand.clone(),
            exponent: self.exponent,
        }
    }

    #[inline]
    fn clone_from(&mut self, source: &Self) {
        self.significand.clone_from(&source.significand);
        self.exponent = source.exponent;
    }
}

impl<R: Round> Context<R> {
    #[inline]
    pub const fn new(precision: usize) -> Self {
        Self {
            precision,
            _marker: PhantomData,
        }
    }

    #[inline]
    pub const fn max(lhs: Self, rhs: Self) -> Self {
        Self {
            precision: if lhs.precision > rhs.precision {
                lhs.precision
            } else {
                rhs.precision
            },
            _marker: PhantomData,
        }
    }

    /// Round the repr to the desired precision
    pub(crate) fn repr_round<const B: Word>(&self, repr: Repr<B>) -> Approximation<Repr<B>, Rounding> {
        assert!(repr.is_finite());
        // XXX: estimated digit length can be used here to prevent costly call to the digits()
        let digits = repr.digits();
        if digits > self.precision {
            let Repr {
                mut significand,
                exponent,
            } = repr;
            let shift = digits - self.precision;
            let r = shr_rem_radix_in_place::<B>(&mut significand, shift);
            let adjust = R::round_fract::<B>(&significand, r, shift);
            Approximation::Inexact(
                Repr::new(significand + adjust, exponent + shift as isize),
                adjust,
            )
        } else {
            Approximation::Exact(repr)
        }
    }
}
