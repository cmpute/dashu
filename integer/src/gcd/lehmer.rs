use core::{cmp::Ordering, mem};
use crate::{
    arch::word::{Word, SignedWord, SignedDoubleWord},
    cmp::cmp_in_place,
    div, shift,
    memory::Memory,
    primitive::{highest_dword, extend_word, split_dword, signed_extend_word, split_signed_dword}
};

/// Estimate the quotient and bezout coefficients using the highest word,
/// return the coefficients (a, b, c, d) such that gcd(x, y) = gcd(ax - by, dy - cx)
/// 
/// If the guess has completely failed, then (b and c will be zero.)
// XXX: try using the highest double word
fn lehmer_guess(mut xbar: Word, mut ybar: Word) -> (Word, Word, Word, Word) {
    debug_assert!(xbar >= ybar);
    let (mut a, mut b, mut c, mut d) = (1, 0, 0, 1);
    while ybar != 0 {
        let q = xbar / ybar;

        let r = a + q * c;
        let s = b + q * d;
        let t = xbar - q * ybar;

        // could check r and s so that it won't lead to overflow last
        if t < s || t + r > ybar - c {
            break;
        }

        a = r; b = s;
        xbar = t;

        if xbar == b {
            break;
        }

        let q = ybar / xbar;

        let r = d + q * b;
        let s = c + q * a;
        let t = ybar - q * xbar;

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

/// Calculate (x, y) = (a*x - b*y, d*y - c*x) in a single run
fn lehmer_step(x: &mut [Word], y: &mut [Word], a: Word, b: Word, c: Word, d: Word) {
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
}

fn gcd_in_place(lhs: &mut [Word], rhs: &mut [Word], memory: &mut Memory) -> usize {
    // keep x >= y though the algorithm
    let (mut x, mut y) = match cmp_in_place(lhs, rhs) {
        Ordering::Equal => return lhs.len(),
        Ordering::Greater => (lhs, rhs),
        Ordering::Less => (rhs, lhs)
    };

    while y.len() >= 2 {
        // Guess the coefficients based on the highest words
        let (x_hi, y_hi) = highest_word_normalized(x, y);
        let (a, b, c, d) = lehmer_guess(x_hi, y_hi);

        if b == 0 {
            // the guess has failed, do a euclidean step (x, y) = (y, x % y)
            let (shift, _) = div::div_rem_unnormalized_in_place(x, y, memory);
            let r = &mut x[..y.len()];
            let y_low_bits = shift::shr_in_place(y, shift);
            let r_low_bits = shift::shr_in_place(r, shift);
            debug_assert!(y_low_bits | r_low_bits == 0);

            x = mem::replace(&mut y, r);
        } else {
            // this step could be optimized with a specialized routine
            lehmer_step(x, y, a, b, c, d);
            if cmp_in_place(x, y).is_le() {
                mem::swap(&mut x, &mut y);
            }
        }
    }

    // TODO: forward to double word gcd
    unimplemented!()
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
