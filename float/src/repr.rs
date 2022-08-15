use crate::{
    round::{mode, Round},
    utils::{digit_len, base_as_ibig}, ibig_ext::remove_pow,
};
use core::marker::PhantomData;
use dashu_int::{IBig, Sign, Word, DoubleWord};

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
        Self { significand: IBig::ZERO, exponent: 0 }
    }
    pub const fn one() -> Self {
        Self { significand: IBig::ONE, exponent: 0 }
    }
    pub const fn neg_one() -> Self {
        Self { significand: IBig::NEG_ONE, exponent: 0 }
    }
    pub const fn infinity() -> Self {
        Self { significand: IBig::ZERO, exponent: 1 }
    }
    pub const fn neg_infinity() -> Self {
        Self { significand: IBig::ZERO, exponent: -1 }
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
        let Self {mut significand, mut exponent} = self;
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
        Self { significand, exponent }
    }

    /// Get the number of digits in the significand.
    pub fn digits(&self) -> usize {
        digit_len::<B>(&self.significand)
    }

    /// Create a [Repr] from significand and exponent. This
    /// constructor will normalize the representation.
    #[inline]
    pub fn new(significand: IBig, exponent: isize) -> Self {
        Self{significand, exponent}.normalize()
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
        Self { precision, _marker: PhantomData }
    }

    #[inline]
    pub const fn max(lhs: Self, rhs: Self) -> Self {
        Self {
            precision: if lhs.precision > rhs.precision {
                lhs.precision
            } else {
                rhs.precision
            },
            _marker: PhantomData
        }
    }
}
