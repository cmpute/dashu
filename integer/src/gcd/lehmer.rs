use core::{cmp::Ordering, mem};
use alloc::alloc::Layout;
use dashu_base::Gcd;

use crate::{
    arch::word::{Word, DoubleWord, SignedWord, SignedDoubleWord},
    cmp::cmp_in_place,
    div, shift, mul,
    memory::Memory,
    primitive::{highest_dword, extend_word, split_dword, signed_extend_word, split_signed_dword, WORD_BITS},
    bits::trim_leading_zeros,
    sign::Sign,
};

/// Minimum length of words for using double word guessing on lehmer step
pub const MIN_DWORD_GUESS_LEN: usize = 300;

/// Estimate the bezout coefficients using the highest word,
/// return the coefficients (a, b, c, d) such that gcd(x, y) = gcd(ax - by, dy - cx)
/// 
/// If the guess has completely failed, then (b and c will be zero.)
fn lehmer_guess(mut xbar: Word, mut ybar: Word) -> (Word, Word, Word, Word) {
    debug_assert!(xbar >= ybar);
    const COEFF_LIMIT: Word = SignedWord::MAX as Word;

    let (mut a, mut b, mut c, mut d) = (1, 0, 0, 1);
    while ybar != 0 {
        let q = xbar / ybar;
        if q > COEFF_LIMIT {
            break;
        }

        let r = a + q * c;
        let s = b + q * d;
        let t = xbar - q * ybar;

        if r > COEFF_LIMIT || s > COEFF_LIMIT {
            break;
        }
        if t < s || t + r > ybar - c {
            break;
        }

        a = r; b = s;
        xbar = t;

        if xbar == b {
            break;
        }

        let q = ybar / xbar;
        if q > COEFF_LIMIT {
            break;
        }

        let r = d + q * b;
        let s = c + q * a;
        let t = ybar - q * xbar;

        if r > COEFF_LIMIT || s > COEFF_LIMIT {
            break;
        }
        if t < s || t + r > xbar - c {
            break;
        }

        d = r; c = s;
        ybar = t;

        if ybar == c {
            break;
        }
    }

    (a, b, c, d)
}

/// Get the (aligned) highest bits of x and y with the width of a Word.
/// If y < x, then y will be padded with leading zeros.
#[inline]
fn highest_word_normalized(x: &[Word], y: &[Word]) -> (Word, Word) {
    let x_hi2 = highest_dword(x);
    let y_hi2 = match x.len() - y.len() {
        0 => highest_dword(y),
        1 => extend_word(*y.last().unwrap()),
        _ => 0
    };
    let shift = x_hi2.leading_zeros();
    let (_, x_hi) = split_dword(x_hi2 << shift);
    let (_, y_hi) = split_dword(y_hi2 << shift);
    (x_hi, y_hi)
}

fn lehmer_guess_dword(mut xbar: DoubleWord, mut ybar: DoubleWord) -> (Word, Word, Word, Word) {
    debug_assert!(xbar >= ybar);
    const COEFF_LIMIT: DoubleWord = SignedWord::MAX as DoubleWord;

    let (mut a, mut b, mut c, mut d) = (1, 0, 0, 1);
    while ybar != 0 {
        let q = xbar / ybar;

        if q > COEFF_LIMIT {
            break;
        }

        let r = a + q * c;
        let s = b + q * d;
        let t = xbar - q * ybar;

        if r > COEFF_LIMIT || s > COEFF_LIMIT {
            break;
        }
        if t < s || t + r > ybar - c {
            break;
        }

        a = r; b = s;
        xbar = t;

        if xbar == b {
            break;
        }

        let q = ybar / xbar;
        if q > COEFF_LIMIT {
            break;
        }

        let r = d + q * b;
        let s = c + q * a;
        let t = ybar - q * xbar;

        if r > COEFF_LIMIT || s > COEFF_LIMIT {
            break;
        }
        if t < s || t + r > xbar - c {
            break;
        }

        d = r; c = s;
        ybar = t;

        if ybar == c {
            break;
        }
    }

    // by now, abcd are all smaller than COEFF_LIMIT, so they fits in Words
    (a as Word, b as Word, c as Word, d as Word)
}

/// Get the (aligned) highest bits of x and y with the width of a DoubleWord.
/// If y < x, then y will be padded with leading zeros.
#[inline]
fn highest_dword_normalized(x: &[Word], y: &[Word]) -> (DoubleWord, DoubleWord) {
    debug_assert!(x.len() >= 3);
    let (x0, x_lo) = x.split_last().unwrap();
    let x12 = highest_dword(x_lo);
    let (y0, y12) = match x.len() - y.len() {
        0 => {
            let (y0, y_lo) = y.split_last().unwrap();
            (*y0, highest_dword(y_lo))
        },
        1 => (0, highest_dword(y)),
        2 => (0, extend_word(*y.last().unwrap())),
        _ => (0, 0)
    };
    let shift = x0.leading_zeros();
    let x_hi = extend_word(*x0) << (shift + WORD_BITS) | x12 >> (WORD_BITS - shift);
    let y_hi = extend_word(y0) << (shift + WORD_BITS) | y12 >> (WORD_BITS - shift);
    (x_hi, y_hi)
}

/// Calculate (x, y) = (a*x - b*y, d*y - c*x) in a single run.
/// 
/// Assuming x > y and (a, b, c, d) are calculated by lehmer_guess. Since the
/// coefficients are estimated by lehmer_guess, it will reduce the x by almost 1.
pub(crate) fn lehmer_step(x: &mut [Word], y: &mut [Word], a: Word, b: Word, c: Word, d: Word) {
    debug_assert!(x.len() >= y.len());
    debug_assert!(a <= SignedWord::MAX as Word && b <= SignedWord::MAX as Word);
    debug_assert!(c <= SignedWord::MAX as Word && d <= SignedWord::MAX as Word);
    let (a, b) = (signed_extend_word(a), signed_extend_word(b));
    let (c, d) = (signed_extend_word(c), signed_extend_word(d));

    let (mut x_carry, mut y_carry) = (0, 0);
    for (x_i, y_i) in x.iter_mut().zip(y.iter_mut()) {
        let (sx_i, sy_i) = (signed_extend_word(*x_i), signed_extend_word(*y_i));
        let (x_new, cx) = split_signed_dword(a * sx_i - b * sy_i + x_carry as SignedDoubleWord);
        let (y_new, cy) = split_signed_dword(d * sy_i - c * sx_i + y_carry as SignedDoubleWord);
        x_carry = cx;
        y_carry = cy;
        *x_i = x_new;
        *y_i = y_new;
    }
    // by now, if x_carry or y_carry is not zero, it will be cancelled out by the next iteration,
    // so we can just discard the high words of x
}

/// Temporary memory required for gcd.
#[inline]
pub fn memory_requirement_up_to(lhs_len: usize, rhs_len: usize) -> Layout {
    // Required memory:
    // - Possible memory required for the division in the euclidean step
    div::memory_requirement_exact(lhs_len, rhs_len)
}

pub(crate) fn gcd_in_place(lhs: &mut [Word], rhs: &mut [Word], memory: &mut Memory) -> (usize, bool) {
    // keep x >= y though the algorithm, and track the source of x and y
    let (mut x, mut y, mut swapped) = match cmp_in_place(lhs, rhs) {
        Ordering::Equal => return (lhs.len(), false),
        Ordering::Greater => (lhs, rhs, false),
        Ordering::Less => (rhs, lhs, true)
    };

    while y.len() > 2 {
        // Guess the coefficients based on the highest words
        let (a, b, c, d) = if x.len() < MIN_DWORD_GUESS_LEN {
            let (x_hi, y_hi) = highest_word_normalized(x, y);
            lehmer_guess(x_hi, y_hi)
        } else {
            let (x_hi, y_hi) = highest_dword_normalized(x, y);
            lehmer_guess_dword(x_hi, y_hi)
        };

        if b == 0 {
            // The guess has failed, do a euclidean step (x, y) = (y, x % y)
            let (shift, _) = div::div_rem_unnormalized_in_place(x, y, memory);
            let mut r = &mut x[..y.len()];
            let y_low_bits = shift::shr_in_place(y, shift);
            let r_low_bits = shift::shr_in_place(r, shift);
            debug_assert!(y_low_bits | r_low_bits == 0);

            // Trim leading zero and swap
            r = trim_leading_zeros(r);
            x = mem::replace(&mut y, r);
            swapped = !swapped;
        } else {
            // this step could be optimized with a specialized routine
            lehmer_step(x, y, a, b, c, d);
            x = trim_leading_zeros(x);
            y = trim_leading_zeros(y);
            if cmp_in_place(x, y).is_le() {
                mem::swap(&mut x, &mut y);
                swapped = !swapped;
            }
        }
    }

    if y.len() == 0 {
        // the gcd result is in x
        (x.len(), swapped)
    } else if y.get(1).unwrap_or(&0) == &0 {
        // forward to single word gcd, store result in x
        let y_word = *y.first().unwrap();
        let x_word = div::rem_by_word(x, y_word);
        x[0] = x_word.gcd(y_word);
        (1, swapped)
    } else {
        // forward to double word gcd, store result in x
        let y_dword = highest_dword(y);
        let x_dword = div::rem_by_dword(x, y_dword);
        let (g_lo, g_hi) = split_dword(x_dword.gcd(y_dword));

        x[0] = g_lo;
        if g_hi != 0 {
            x[1] = g_hi;
            (2, swapped)
        } else {
            (1, swapped)
        }
    }
}

/// Calculate (s, t) = (a*s - b*t, d*t - c*s) in a single run. Unlike [lehmer_step], both
/// s and t are signed, and the output is not guaranteed to be smaller. This input must have
/// the same size, if not, the smaller one should be padded with zeros.
fn lehmer_ext_step(s: (Sign, &mut [Word]), t: (Sign, &mut [Word]), a: Word, b: Word, c: Word, d: Word) -> (SignedWord, SignedWord) {
    debug_assert!(a <= SignedWord::MAX as Word && b <= SignedWord::MAX as Word);
    debug_assert!(c <= SignedWord::MAX as Word && d <= SignedWord::MAX as Word);

    let (s_sign, s_words) = s;
    let (t_sign, t_words) = t;
    let (a, c) = match s_sign {
        Sign::Positive => (signed_extend_word(a), signed_extend_word(c)),
        Sign::Negative => (-signed_extend_word(a), -signed_extend_word(c))
    };
    let (b, d) = match t_sign {
        Sign::Positive => (signed_extend_word(b), signed_extend_word(d)),
        Sign::Negative => (-signed_extend_word(b), -signed_extend_word(d)),
    };

    let (mut s_carry, mut t_carry) = (0, 0);
    for (s_i, t_i) in s_words.iter_mut().zip(t_words.iter_mut()) {
        let (ss_i, st_i) = (signed_extend_word(*s_i), signed_extend_word(*t_i));
        let (s_new, cs) = split_signed_dword(a * ss_i - b * st_i + s_carry as SignedDoubleWord);
        let (t_new, ct) = split_signed_dword(d * st_i - c * ss_i + t_carry as SignedDoubleWord);
        s_carry = cs;
        t_carry = ct;
        *s_i = s_new;
        *t_i = t_new;
    }
    (s_carry, t_carry)
}

/// Extended binary GCD for two multi-digits numbers
pub fn gcd_ext_in_place(
    lhs: &mut [Word],
    rhs: &mut [Word],
    g: &mut [Word],
    bonly: bool,
    memory: &mut Memory,
) -> (Sign, Sign) {
    let (lhs_len, rhs_len) = (lhs.len(), rhs.len());

    // keep x >= y though the algorithm, and track the source of x and y
    let (mut x, mut y, mut swapped) = match cmp_in_place(lhs, rhs) {
        Ordering::Equal => {
            // TODO: remove fill by returning a length as well
            lhs[1..].fill(0);
            rhs[1..].fill(0);
            *lhs.first_mut().unwrap() = 1;
            *rhs.first_mut().unwrap() = 0;
            return (Sign::Positive, Sign::Positive)
        },
        Ordering::Greater => (lhs, rhs, false),
        Ordering::Less => (rhs, lhs, true)
    };

    // keep x = s0*lhs - t0*rhs, y = t1*rhs - s1*lhs, gcd(x, y) = gcd(lhs, rhs)
    let (mut s0, mut memory) = memory.allocate_slice_fill::<Word>(rhs_len, 0);
    let (mut s1, mut memory) = memory.allocate_slice_fill::<Word>(rhs_len, 0);
    let (mut t0, mut memory) = memory.allocate_slice_fill::<Word>(lhs_len, 0);
    let (mut t1, mut memory) = memory.allocate_slice_fill::<Word>(lhs_len, 0);
    let (mut s0_end, mut s1_end) = (1, 1);
    let (mut t0_end, mut t1_end) = (1, 1);

    if swapped {
        *s1.first_mut().unwrap() = 1;
        *t0.first_mut().unwrap() = 1;
    } else {
        *s0.first_mut().unwrap() = 1;
        *t1.first_mut().unwrap() = 1;
    }

    
    while y.len() >= 2 {
        // Guess the coefficients based on the highest words
        let (a, b, c, d) = if x.len() < MIN_DWORD_GUESS_LEN {
            let (x_hi, y_hi) = highest_word_normalized(x, y);
            lehmer_guess(x_hi, y_hi)
        } else {
            let (x_hi, y_hi) = highest_dword_normalized(x, y);
            lehmer_guess_dword(x_hi, y_hi)
        };
        dbg!(a, b, c, d);

        if b == 0 {
            // The guess has failed, do a euclidean step (x, y) = (y, x % y)
            let (shift, q_top) = div::div_rem_unnormalized_in_place(x, y, &mut memory);
            let (mut r, q_lo) = x.split_at_mut(y.len());
            let y_low_bits = shift::shr_in_place(y, shift);
            let r_low_bits = shift::shr_in_place(r, shift);
            debug_assert!(y_low_bits | r_low_bits == 0);

            // s0 += q*s1, t1 += q*t0
            s0_end = q_lo.len() + s1_end;
            let s_carry = mul::add_signed_mul(&mut s0[..s0_end], Sign::Positive, q_lo, &s1[..s1_end], &mut memory);
            t0_end = q_lo.len() + t1_end;
            let t_carry = mul::add_signed_mul(t1, Sign::Positive, q_lo, &mut t0[..s1_end], &mut memory);
            debug_assert!(s_carry | t_carry == 0);
            if q_top > 0 {
                let s_carry = mul::add_mul_word_in_place(&mut s0[q_lo.len()..q_lo.len() + s0_end], q_top, &s1[..s1_end]);
                if s_carry > 0 {
                    s0[s0_end] = s_carry;
                    s0_end += 1;
                }
                let t_carry = mul::add_mul_word_in_place(&mut t1[q_lo.len()..q_lo.len() + t1_end], q_top, &t0[..t0_end]);
                if t_carry > 0 {
                    t1[t1_end] = t_carry;
                    t1_end += 1;
                }
            }

            // Trim leading zero and swap
            r = trim_leading_zeros(r);
            x = mem::replace(&mut y, r);
            mem::swap(&mut s0, &mut s1);
            mem::swap(&mut t0, &mut t1);
            mem::swap(&mut s0_end, &mut s1_end);
            mem::swap(&mut t0_end, &mut t1_end);
            swapped = !swapped;
        } else {
            // this step could be optimized with a specialized routine
            lehmer_step(x, y, a, b, c, d);
            x = trim_leading_zeros(x);
            y = trim_leading_zeros(y);

            // TODO: call lehmer_ext_step on s, t
            if cmp_in_place(x, y).is_le() {
                mem::swap(&mut x, &mut y);
                swapped = !swapped;
            }
        }
    }

    unimplemented!();
}

/*

fn lxgcd(mut x: UBig, mut y: UBig) -> (UBig, IBig, IBig) {
    use crate::ops::DivRem;
    use crate::ibig;

    if x < y {
        let (g, cy, cx) = lxgcd(y, x);
        return (g, cx, cy)
    }

    let (mut last_s, mut s) = (ibig!(1), ibig!(0));
    let (mut last_t, mut t) = (ibig!(0), ibig!(1));

    while let Large(v) = y.repr() {
        let (xbar, ybar) = if x.len() > v.len() {
            (*x.as_words().last().unwrap(), 0)
        } else {
            (*x.as_words().last().unwrap(), *v.last().unwrap())
        };

        let (a, b, c, d) = lehmer_guess(xbar, ybar);

        if b == 0 {
            let (q, r) = x.div_rem(&y);
            x = y; y = r;
            let new_s = last_s - IBig::from(q.clone())*&s;
            last_s = s; s = new_s;
            let new_t = last_t - IBig::from(q)*&t;
            last_t = t; t = new_t;
            
        } else {
            let new_x = a*&x - b*&y;
            let new_y = d*&y - c*&x;
            let new_sx = a*&last_s - b*&s; let new_sy = d*s - c*last_s;
            let new_tx = a*&last_t - b*&t; let new_ty = d*t - c*last_t;
            
            if new_x >= new_y {
                x = new_x; y = new_y;
                last_s = new_sx; s = new_sy;
                last_t = new_tx; t = new_ty;
            } else {
                y = new_x; x = new_y;
                s = new_sx; last_s = new_sy;
                t = new_tx; last_t = new_ty;
            }
        }
    }

    let (g, cx, cy) = x.extended_gcd(y);
    
    (g, &cx * last_s + &cy * s, cx * last_t + cy * t)
}

*/
