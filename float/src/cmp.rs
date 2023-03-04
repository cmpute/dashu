use core::cmp::Ordering;

use crate::{fbig::FBig, repr::Repr, repr::Word, round::Round, utils::shl_digits};

impl<R1: Round, R2: Round, const B: Word> PartialEq<FBig<R2, B>> for FBig<R1, B> {
    #[inline]
    fn eq(&self, other: &FBig<R2, B>) -> bool {
        match (self.repr.is_infinite(), other.repr.is_infinite()) {
            // +inf == +inf, -inf == -inf
            (true, true) => !((self.repr.exponent >= 0) ^ (other.repr.exponent >= 0)),

            // the representation is normalized so direct comparing is okay,
            // and the context doesn't count in comparison
            (false, false) => self.repr == other.repr,

            // inf != any exact numbers
            (_, _) => false,
        }
    }
}
impl<R: Round, const B: Word> Eq for FBig<R, B> {}

fn repr_cmp<const B: Word>(
    lhs: &Repr<B>,
    rhs: &Repr<B>,
    precision: Option<(usize, usize)>,
) -> Ordering {
    // case 1: compare with inf
    match (lhs.is_infinite(), rhs.is_infinite()) {
        (true, true) => return lhs.exponent.cmp(&rhs.exponent),
        (false, true) => {
            return match rhs.exponent >= 0 {
                true => Ordering::Less,
                false => Ordering::Greater,
            }
        }
        (true, false) => {
            return match lhs.exponent >= 0 {
                true => Ordering::Greater,
                false => Ordering::Less,
            }
        }
        _ => {}
    };

    // case 2: compare sign
    match lhs.significand.signum().cmp(&rhs.significand.signum()) {
        Ordering::Greater => return Ordering::Greater,
        Ordering::Less => return Ordering::Less,
        _ => {}
    };
    let sign = lhs.significand.sign();

    // case 3: compare exponent and precision
    let (lhs_exp, rhs_exp) = (lhs.exponent, rhs.exponent);
    if let Some((lhs_prec, rhs_prec)) = precision {
        if lhs_prec != 0 && rhs_prec != 0 {
            // only compare when both number are not having arbitrary precision
            if lhs_exp > rhs_exp + rhs_prec as isize {
                return sign * Ordering::Greater;
            }
            if rhs_exp > lhs_exp + lhs_prec as isize {
                return sign * Ordering::Less;
            }
        }
    }

    // case 4: compare exponent and digits
    let (lhs_digits, rhs_digits) = (lhs.digits_ub(), rhs.digits_ub());
    if lhs_exp > rhs_exp + rhs_digits as isize {
        return sign * Ordering::Greater;
    }
    if rhs_exp > lhs_exp + lhs_digits as isize {
        return sign * Ordering::Less;
    }

    // case 5: compare exact values by shifting
    let (lhs_signif, rhs_signif) = (&lhs.significand, &rhs.significand);
    match lhs_exp.cmp(&rhs_exp) {
        Ordering::Equal => lhs_signif.cmp(rhs_signif),
        Ordering::Greater => {
            shl_digits::<B>(lhs_signif, (lhs_exp - rhs_exp) as usize).cmp(rhs_signif)
        }
        Ordering::Less => {
            lhs_signif.cmp(&shl_digits::<B>(rhs_signif, (rhs_exp - lhs_exp) as usize))
        }
    }
}

impl<const B: Word> PartialOrd for Repr<B> {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<const B: Word> Ord for Repr<B> {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        repr_cmp(self, other, None)
    }
}

impl<R1: Round, R2: Round, const B: Word> PartialOrd<FBig<R2, B>> for FBig<R1, B> {
    #[inline]
    fn partial_cmp(&self, other: &FBig<R2, B>) -> Option<Ordering> {
        Some(repr_cmp(
            &self.repr,
            &other.repr,
            Some((self.context.precision, other.context.precision)),
        ))
    }
}

impl<R: Round, const B: Word> Ord for FBig<R, B> {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        repr_cmp(&self.repr, &other.repr, Some((self.context.precision, other.context.precision)))
    }
}
