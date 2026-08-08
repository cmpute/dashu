use crate::{
    div_exact::hensel_div_odd_in_place,
    math::inv_mod_pow2,
    primitive::{extend_word, shrink_dword, WORD_BITS},
    repr::TypedReprRef,
    ubig::UBig,
    Word,
};
use alloc::vec;
use dashu_base::{DivRem, PowerOfTwo};

impl UBig {
    /// Divide out all multiples of the factor from the integer,
    /// returns the exponent of the removed factor.
    ///
    /// For self = 0 or factor = 0 or 1, this method returns None.
    ///
    /// # Examples
    ///
    /// ```
    /// use dashu_int::UBig;
    ///
    /// let mut a = UBig::from(8u32) * 3u32;
    /// assert_eq!(a.remove(&UBig::from(2u32)), Some(3));
    /// assert_eq!(a, UBig::from(3u32));
    /// ```
    pub fn remove(&mut self, factor: &UBig) -> Option<usize> {
        // A factor that fits in a single word is handled by the faster `remove_word`.
        if let TypedReprRef::RefSmall(dword) = factor.repr() {
            if let Some(word) = shrink_dword(dword) {
                return self.remove_word(word);
            }
        }

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
        let mut pows = vec![factor.sqr()];
        loop {
            let last = pows.last().unwrap();
            let (new_q, r) = (&q).div_rem(last);
            if !r.is_zero() {
                break;
            }

            exp += 1 << pows.len();
            q = new_q;
            let next_sq = last.sqr();
            pows.push(next_sq);
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

    /// Divide out all multiples of a single-word factor from the integer,
    /// returns the exponent of the removed factor.
    ///
    /// The single-word specialization of [`remove`](Self::remove): the factor's power-of-two part is
    /// stripped by the 2-valuation, and the odd part is divided out by **Hensel (2-adic) exact
    /// division** — the modular inverse of the factor is precomputed by Newton iteration, and each
    /// quotient limb is computed as `(word − carry) · d^{-1} mod 2^WORD_BITS`, i.e. only multiplies
    /// and subtracts with no division or normalization. Powers `d², d⁴, …` (which exceed a single
    /// word) use the general division, mirroring the binary-splitting in [`remove`](Self::remove).
    ///
    /// For self = 0 or factor = 0 or 1, this method returns None.
    ///
    /// # Examples
    ///
    /// ```
    /// use dashu_int::UBig;
    ///
    /// let mut a = UBig::from(10u32).pow(8) * 7u32;
    /// assert_eq!(a.remove_word(10), Some(8));
    /// assert_eq!(a, UBig::from(7u32));
    /// ```
    pub fn remove_word(&mut self, factor: Word) -> Option<usize> {
        if self.is_zero() || factor == 0 || factor == 1 {
            return None;
        }

        // factor = 2^s · d_odd: the power-of-two part and the odd part are removed separately.
        let trailing = factor.trailing_zeros();
        let s = trailing as usize;
        let d_odd = factor >> trailing;

        if d_odd == 1 {
            // A pure power of two: the exponent is bounded by the 2-valuation of self.
            let exp = self.trailing_zeros().unwrap() / s;
            *self >>= exp * s;
            return Some(exp);
        }

        if s == 0 {
            // An odd factor: no power-of-two part to reconcile.
            return Some(remove_odd_powers(self, d_odd, usize::MAX));
        }

        // A mixed factor (e.g. 10 = 2·5): strip the trailing 2s once, divide out the odd part (each
        // full `factor`-power consumes `s` of them, so the odd-part count is capped by `tz/s`), then
        // reassemble the cofactor `self / factor^exp = (odd part) << (tz − s·exp)` (the shift amount
        // is non-negative because `exp ≤ cap = tz/s`).
        let tz = self.trailing_zeros().unwrap();
        let cap = tz / s;
        let mut odd = self.clone() >> tz;
        let exp = remove_odd_powers(&mut odd, d_odd, cap);
        *self = odd << (tz - s * exp);
        Some(exp)
    }
}

/// Remove powers of the odd single-word `d` from `n` (in place), at most `cap` of them, and return
/// the number removed. `n` is left unchanged when `d` does not divide it.
///
/// The first division by `d` uses the Hensel (2-adic) exact division
/// ([`crate::div_exact::hensel_div_odd_in_place`]), which computes the quotient with only multiplies
/// and subtracts; the powers `d², d⁴, …` that follow exceed a single word and use the general
/// division, mirroring the binary-splitting of [`UBig::remove`].
fn remove_odd_powers(n: &mut UBig, d: Word, cap: usize) -> usize {
    if cap == 0 {
        return 0;
    }
    let di = inv_mod_pow2(extend_word(d), WORD_BITS) as Word;

    // First division by d into a scratch quotient; `n` is left untouched on failure.
    let mut q: alloc::vec::Vec<Word> = n.as_words().to_vec();
    if !hensel_div_odd_in_place(&mut q, d, di) {
        return 0;
    }
    *n = UBig::from_words(&q);
    let mut exp = 1;

    // Grow the powers d², d⁴, … while each divides the current quotient exactly (and stays within
    // `cap`), then refine downward with the collected powers, and finally divide out one last single
    // `d` (the count may be odd, leaving a leftover below d²).
    let mut power = UBig::from_word(d).sqr();
    let mut power_exp = 2usize;
    let mut pows = vec![(power.clone(), power_exp)];
    while exp + power_exp <= cap && *n >= power {
        let (new_q, r) = (&*n).div_rem(&power);
        if r.is_zero() {
            *n = new_q;
            exp += power_exp;
            power = power.sqr();
            power_exp <<= 1;
            pows.push((power.clone(), power_exp));
        } else {
            break;
        }
    }
    while let Some((p, e)) = pows.pop() {
        if exp + e > cap {
            continue;
        }
        let (new_q, r) = (&*n).div_rem(&p);
        if r.is_zero() {
            *n = new_q;
            exp += e;
        }
    }
    if exp < cap {
        let (new_q, r) = (&*n).div_rem(&UBig::from_word(d));
        if r.is_zero() {
            *n = new_q;
            exp += 1;
        }
    }
    exp
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::UBig;

    /// The Hensel kernel must agree with the general division on a sweep of small values — both the
    /// exactness test (remainder == 0) and the quotient itself.
    #[test]
    fn test_hensel_div_matches_div_rem() {
        for d in [3u16, 5, 7, 9, 15, 21, 255, 1001] {
            let d = d as Word;
            let d = d as Word;
            for lo in [0u8, 1, 2, 5, 200, 255] {
                let lo = lo as Word;
                for hi in [0u8, 1, 2, 7, 199] {
                    let hi = hi as Word;
                    let u = UBig::from_words(&[lo, hi]);
                    if u.is_zero() {
                        continue;
                    }
                    let mut q: alloc::vec::Vec<Word> = u.as_words().to_vec();
                    let di = inv_mod_pow2(extend_word(d), WORD_BITS) as Word;
                    let exact = hensel_div_odd_in_place(&mut q, d, di);
                    let (qr, r) = (&u).div_rem(&UBig::from_word(d));
                    assert_eq!(exact, r.is_zero(), "d={d} u={u:?}");
                    if r.is_zero() {
                        assert_eq!(UBig::from_words(&q), qr, "quotient d={d} u={u:?}");
                    }
                }
            }
        }
    }

    /// The Newton inverse must satisfy `d · d^{-1} ≡ 1 (mod 2^WORD_BITS)` for odd `d`.
    #[test]
    fn test_inv_mod_pow2() {
        for d in [3u8, 5, 7, 9, 11, 15, 31, 127, 255] {
            let d = d as Word;
            let inv = inv_mod_pow2(extend_word(d), WORD_BITS) as Word;
            assert_eq!(d.wrapping_mul(inv), 1, "d·d^-1 ≡ 1 for d={d}",);
        }
    }

    /// `remove_word` must agree with `remove` for every single-word factor, on both odd and even
    /// factors and with extra 2s mixed in.
    #[test]
    fn test_remove_word_matches_remove() {
        for d in [2u8, 3, 5, 7, 10, 12, 16, 25] {
            let d = d as Word;
            for i in 0..10usize {
                for rest in [1u8, 5, 7, 11] {
                    let base = UBig::from(d).pow(i) * rest;
                    for extra_twos in 0..3usize {
                        let n = base.clone() << extra_twos;
                        let mut a = n.clone();
                        let exp_a = a.remove_word(d);
                        let mut b = n.clone();
                        let exp_b = b.remove(&UBig::from(d));
                        assert_eq!(exp_a, exp_b, "d={d} i={i} rest={rest} twos={extra_twos}");
                        assert_eq!(a, b, "d={d} i={i} rest={rest} twos={extra_twos}");
                    }
                }
            }
        }
    }

    /// A not-divisible factor must leave the value unchanged (`remove_word` computes the quotient in
    /// a scratch and only commits it on an exact division).
    #[test]
    fn test_remove_word_noop_when_not_divisible() {
        for n in [3u32, 7, 9, 11, 13, 25, 100, 1000] {
            for d in [2u8, 3, 5, 7, 10] {
                let d = d as Word;
                let original = UBig::from(n);
                let mut a = original.clone();
                if a.remove_word(d) == Some(0) {
                    assert_eq!(a, original, "not-divisible must leave n unchanged: d={d} n={n}");
                }
            }
        }
    }

    /// Multi-word operands, including a pure power of two, a mixed factor, and a large odd factor.
    #[test]
    fn test_remove_word_large() {
        let n = UBig::from(10u8).pow(200);
        let mut a = n.clone();
        assert_eq!(a.remove_word(10), Some(200));
        assert_eq!(a, UBig::ONE);

        // 5^200 · 2^100 · 7: the odd factor's 2-valuation is irrelevant to it.
        let mut b = UBig::from(5u8).pow(200) * (UBig::ONE << 100) * 7u8;
        assert_eq!(b.remove_word(5), Some(200));
        assert_eq!(b, (UBig::ONE << 100) * 7u8);

        // A pure power of two.
        let mut c = (UBig::ONE << 1000) * 3u8;
        assert_eq!(c.remove_word(4), Some(500));
        assert_eq!(c, UBig::from(3u8));
    }
}
