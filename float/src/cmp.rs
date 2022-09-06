use core::cmp::Ordering;

use crate::{
    fbig::FBig,
    repr::Word,
    round::Round,
    utils::shl_digits,
};

impl<R: Round, const B: Word> PartialEq for FBig<R, B> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        match (self.repr.is_infinite(), other.repr.is_infinite()) {
            // +inf = +inf, -inf = -inf
            (true, true) => !((self.repr.exponent >= 0) ^ (other.repr.exponent >= 0)),

            // the representation is normalized so direct comparing is okay,
            // and the context doesn't count in comparison
            (false, false) => self.repr == other.repr,

            // inf != any exact numbers
            (_, _) => false
        }
    }
}
impl<R: Round, const B: Word> Eq for FBig<R, B> {}

impl<R: Round, const B: Word> PartialOrd for FBig<R, B> {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<R: Round, const B: Word> Ord for FBig<R, B> {
    fn cmp(&self, other: &Self) -> Ordering {
        // case 1: compare with inf
        match (self.repr.is_infinite(), other.repr.is_infinite()) {
            (true, true) => return self.repr.exponent.cmp(&other.repr.exponent),
            (false, true) => return match other.repr.exponent >= 0 {
                true => Ordering::Less,
                false => Ordering::Greater
            },
            (true, false) => return match self.repr.exponent >= 0 {
                true => Ordering::Greater,
                false => Ordering::Less
            },
            _ => {}
        };

        // case 2: compare sign
        match self.repr.significand.signum().cmp(&other.repr.significand.signum()) {
            Ordering::Greater => return Ordering::Greater,
            Ordering::Less => return Ordering::Less,
            _ => {}
        };
        let sign = self.repr.significand.sign();

        // case 3: compare exponent and precision
        let (lhs_exp, rhs_exp) = (self.repr.exponent, other.repr.exponent);
        let (lhs_prec, rhs_prec) = (self.context.precision, other.context.precision);
        if lhs_prec != 0 && rhs_prec != 0 {
            // only compare when both number are not having arbitrary precision
            if lhs_exp > rhs_exp + rhs_prec as isize{
                return sign * Ordering::Greater;
            }
            if rhs_exp > lhs_exp + lhs_prec as isize {
                return sign * Ordering::Less;
            }
        }

        // case 4: compare exponent and digits
        let (lhs_digits, rhs_digits) = (self.repr.digits_ub(), other.repr.digits_ub());
        if lhs_exp > rhs_exp + rhs_digits as isize {
            return sign * Ordering::Greater;
        }
        if rhs_exp > lhs_exp + lhs_digits as isize {
            return sign * Ordering::Less;
        }

        // case 5: compare exact values by shifting
        match lhs_exp.cmp(&rhs_exp) {
            Ordering::Equal => self.repr.significand.cmp(&other.repr.significand),
            Ordering::Greater => shl_digits::<B>(&self.repr.significand, (lhs_exp - rhs_exp) as usize).cmp(&other.repr.significand),
            Ordering::Less => self.repr.significand.cmp(&shl_digits::<B>(&other.repr.significand, (rhs_exp - lhs_exp) as usize)),
        }
    }
}

// TODO: implement comparison with IBig
