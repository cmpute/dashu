//! Exact oracles and assertions shared by high-part integration tests.
use dashu_base::{
    Approximation::{Exact, Inexact},
    DivRem, EstimatedLog2, UnsignedAbs,
};
use dashu_float::{
    round::{Round, Rounded},
    Context, FBig, Repr, Word,
};
use dashu_int::{IBig, UBig};

pub fn significand(words: &[Word]) -> IBig {
    IBig::from(UBig::from_words(words))
}

/// Scale the exact rational to p significant digits, then classify its exact
/// remainder. No floating-point division or high-precision self-oracle is used.
pub fn quotient_oracle<R: Round, const B: Word>(
    p: usize,
    a: &Repr<B>,
    b: &Repr<B>,
) -> Rounded<Repr<B>> {
    let sign = a.significand().sign() * b.significand().sign();
    let mut numer = a.significand().clone().unsigned_abs();
    let mut denom = b.significand().clone().unsigned_abs();
    assert!(!numer.is_zero() && !denom.is_zero());
    let base = UBig::from(B);
    let mut order = numer.ilog(&base) as isize - denom.ilog(&base) as isize;
    let below = if order >= 0 {
        numer < &denom * base.pow(order as usize)
    } else {
        &numer * base.pow((-order) as usize) < denom
    };
    if below {
        order -= 1;
    }
    let scale = p as isize - 1 - order;
    if scale >= 0 {
        numer *= base.pow(scale as usize);
    } else {
        denom *= base.pow((-scale) as usize);
    }
    let (q, rem) = numer.div_rem(&denom);
    let q = sign * IBig::from(q);
    let exp = a.exponent() - b.exponent() - scale;
    if rem.is_zero() {
        return Exact(Repr::new(q, exp));
    }
    let adjust = R::round_low_part(&q, sign, || (&rem << 1).cmp(&denom));
    Inexact(Repr::new(q + adjust, exp), adjust)
}

/// Words a p-digit significand occupies in base B (one spare word for the
/// guard digit a previous rounding may have left).
pub fn words_for_precision(p: usize, base: Word) -> usize {
    let (_, lb) = base.log2_bounds();
    let bits = p as f32 * lb + Word::BITS as f32;
    (bits as usize) / (Word::BITS as usize) + 2
}

/// Scale a word count tuned for 64-bit words to preserve the operand's bit
/// length on every target.
pub fn scale(n: usize) -> usize {
    n * (64 / Word::BITS as usize).max(1)
}

pub fn check_mul<R: Round, const B: Word>(p: usize, a: &Repr<B>, b: &Repr<B>) {
    let expected = Context::<R>::new(0)
        .mul(a, b)
        .unwrap()
        .value()
        .with_precision(p);
    assert_eq!(
        Context::<R>::new(p).mul(a, b).unwrap().map(FBig::into_repr),
        expected.map(FBig::into_repr)
    );
}

pub fn check_sqr<R: Round, const B: Word>(p: usize, a: &Repr<B>) {
    let expected = Context::<R>::new(0)
        .sqr(a)
        .unwrap()
        .value()
        .with_precision(p);
    assert_eq!(
        Context::<R>::new(p).sqr(a).unwrap().map(FBig::into_repr),
        expected.map(FBig::into_repr)
    );
}

pub fn check_cubic<R: Round, const B: Word>(p: usize, a: &Repr<B>) {
    let expected = Context::<R>::new(0)
        .cubic(a)
        .unwrap()
        .value()
        .with_precision(p);
    assert_eq!(
        Context::<R>::new(p).cubic(a).unwrap().map(FBig::into_repr),
        expected.map(FBig::into_repr)
    );
}

pub fn check_products<R: Round, const B: Word>(p: usize, a: &Repr<B>, b: &Repr<B>, f: &Repr<B>) {
    check_mul::<R, B>(p, a, b);
    check_sqr::<R, B>(p, f);
    check_cubic::<R, B>(p, f);
}
