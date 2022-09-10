use crate::ubig::UBig;
use alloc::vec;
use dashu_base::{DivRem, PowerOfTwo};

impl UBig {
    /// Divide out all multiples of the factor from the integer,
    /// returns the exponent of the removed factor.
    ///
    /// For self = 0 or factor = 0 or 1, this method returns None.
    pub fn remove(&mut self, factor: &UBig) -> Option<usize> {
        if self.is_zero() || factor.is_zero() || factor.is_one() {
            return None;
        }

        // shortcut for power of 2
        if factor.is_power_of_two() {
            let bits = factor.trailing_zeros().unwrap();
            let exp = self.trailing_zeros().unwrap() / bits;
            *self >>= exp * bits;
            return Some(exp);
        }

        let (mut q, r) = (&*self).div_rem(factor);
        if !r.is_zero() {
            return Some(0);
        }

        // first stage, division with exponentially growing factors
        let mut exp = 1;
        let mut pows = vec![factor.square()];
        loop {
            let last = pows.last().unwrap();
            let (new_q, r) = (&q).div_rem(last);
            if !r.is_zero() {
                break;
            }

            exp += 1 << pows.len();
            q = new_q;
            pows.push(last.square());
        }

        // second stage, division from highest power to the lowest
        while let Some(last) = pows.pop() {
            let (new_q, r) = (&q).div_rem(last);
            if r.is_zero() {
                exp += 1 << (pows.len() + 1);
                q = new_q;
            }
        }

        // last division
        let (new_q, r) = (&q).div_rem(factor);
        if r.is_zero() {
            exp += 1;
            q = new_q;
        }

        *self = q;
        Some(exp)
    }
}
