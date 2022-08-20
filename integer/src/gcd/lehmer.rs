use alloc::alloc::Layout;
use core::{mem, ptr, slice};
use dashu_base::{ExtendedGcd, Gcd};

use crate::{
    arch::word::{DoubleWord, SignedDoubleWord, SignedWord, Word},
    cmp::cmp_in_place,
    div,
    helper_macros::debug_assert_zero,
    memory::{self, Memory},
    mul,
    primitive::{
        extend_word, highest_dword, locate_top_word_plus_one, signed_extend_word, split_dword,
        split_signed_dword, WORD_BITS,
    },
    shift,
    sign::Sign,
};

/// Remove the leading zero words in an owning reference. Return
/// a null slice if the input is zero (but the start pointer still
/// points to the same address as input).
#[inline]
fn trim_leading_zeros(words: &mut [Word]) -> &mut [Word] {
    words.split_at_mut(locate_top_word_plus_one(words)).0
}

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

        a = r;
        b = s;
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

        d = r;
        c = s;
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
        _ => 0,
    };
    let shift = x_hi2.leading_zeros();
    let (_, x_hi) = split_dword(x_hi2 << shift);
    let (_, y_hi) = split_dword(y_hi2 << shift);
    (x_hi, y_hi)
}

/// Same as [lehmer_guess] but use the highest double word
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

        a = r;
        b = s;
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

        d = r;
        c = s;
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
        }
        1 => (0, highest_dword(y)),
        2 => (0, extend_word(*y.last().unwrap())),
        _ => (0, 0),
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
    debug_assert!(x.len() >= y.len() && x.len() - y.len() <= 1);
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

    // if the carry words are not zero, then at most one additional step is required
    if x_carry != 0 {
        let x_top = x.last_mut().unwrap();
        debug_assert_eq!(y_carry as SignedDoubleWord, c * signed_extend_word(*x_top));
        let (x_new, cx) =
            split_signed_dword(a * signed_extend_word(*x_top) + x_carry as SignedDoubleWord);
        debug_assert_eq!(cx, 0);
        *x_top = x_new;
    }
}

/// Temporary memory required for gcd.
#[inline]
pub fn memory_requirement_up_to(lhs_len: usize, rhs_len: usize) -> Layout {
    // Required memory:
    // - temporary space for the division in the euclidean step
    div::memory_requirement_exact(lhs_len, rhs_len)
}

pub(crate) fn gcd_in_place(
    lhs: &mut [Word],
    rhs: &mut [Word],
    memory: &mut Memory,
) -> (usize, bool) {
    // keep x >= y though the algorithm, and track the source of x and y
    debug_assert!(cmp_in_place(lhs, rhs).is_ge());
    let (mut x, mut y, mut swapped) = (lhs, rhs, false);

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
            debug_assert_zero!(shift::shr_in_place(y, shift));
            debug_assert_zero!(shift::shr_in_place(r, shift));
            r = trim_leading_zeros(r);

            // swap: (x, y) = (y, r)
            x = mem::replace(&mut y, r);
            swapped = !swapped;
        } else {
            // The lehmer guess succeeded, use the coefficients to update x, y
            lehmer_step(x, y, a, b, c, d);
            x = trim_leading_zeros(x);
            y = trim_leading_zeros(y);
            if cmp_in_place(x, y).is_le() {
                mem::swap(&mut x, &mut y);
                swapped = !swapped;
            }
        }
    }

    if y.is_empty() {
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

/// Calculate `(x, y) = (a*x + b*y, c*x + d*y)` in a single run, the argument `len` is used
/// to bound the iteration range (this function operates on x[..len] and y[..len]). Returns carry words.
fn lehmer_ext_step(
    x: &mut [Word],
    y: &mut [Word],
    len: usize,
    a: Word,
    b: Word,
    c: Word,
    d: Word,
) -> (Word, Word) {
    debug_assert!(len <= x.len() && len <= y.len());
    debug_assert!(a <= SignedWord::MAX as Word && b <= SignedWord::MAX as Word);
    debug_assert!(c <= SignedWord::MAX as Word && d <= SignedWord::MAX as Word);
    let (a, b) = (extend_word(a), extend_word(b));
    let (c, d) = (extend_word(c), extend_word(d));

    let (mut x_carry, mut y_carry) = (0, 0);
    for (x_i, y_i) in x.iter_mut().zip(y.iter_mut()).take(len) {
        let (sx_i, sy_i) = (extend_word(*x_i), extend_word(*y_i));
        let (x_new, cx) = split_dword(a * sx_i + b * sy_i + extend_word(x_carry));
        let (y_new, cy) = split_dword(c * sx_i + d * sy_i + extend_word(y_carry));
        x_carry = cx;
        y_carry = cy;
        *x_i = x_new;
        *y_i = y_new;
    }
    (x_carry, y_carry)
}

/// Temporary memory required for extended gcd.
pub fn memory_requirement_ext_up_to(lhs_len: usize, rhs_len: usize) -> Layout {
    // Required memory:
    // - two numbers (t0 & t1) with at most the same size as lhs, add 1 buffer word
    // - temporary space for a division (for euclidean step), and later a mulitplication (for coeff update)
    let t_words = 2 * lhs_len + 2;
    memory::add_layout(
        memory::array_layout::<Word>(t_words),
        memory::max_layout(
            div::memory_requirement_exact(lhs_len, rhs_len), //
            mul::memory_requirement_up_to(lhs_len, lhs_len / 2), // for coeff update
        ),
    )
}

/// Extended binary GCD for two multi-digits numbers
pub fn gcd_ext_in_place(
    lhs: &mut [Word],
    rhs: &mut [Word],
    memory: &mut Memory,
) -> (usize, usize, Sign) {
    let (lhs_len, rhs_len) = (lhs.len(), rhs.len());
    let (lhs_ptr, rhs_ptr) = (lhs.as_mut_ptr(), rhs.as_mut_ptr());

    // keep x >= y though the algorithm, and track the source of x and y using the swapped flag
    debug_assert!(cmp_in_place(lhs, rhs).is_ge());
    let (mut x, mut y) = (lhs, rhs);
    let mut swapped = false;

    // the normal way is to have four variables s0, s1, t0, t1 and keep gcd(x, y) = gcd(lhs, rhs),
    // x = s0*lhs - t0*rhs, y = t1*rhs - s1*lhs. Here we simplify it by only tracking the
    // coefficient of rhs, so that x = -t0*rhs mod lhs, y = t1*rhs mod lhs,
    let (mut t0, mut memory) = memory.allocate_slice_fill::<Word>(lhs_len + 1, 0);
    let (mut t1, mut memory) = memory.allocate_slice_fill::<Word>(lhs_len + 1, 0);
    let (mut t0_len, mut t1_len) = (1, 1);
    *t1.first_mut().unwrap() = 1;

    // loop, reduce x, y until the smaller one (y) fits in a single word
    while y.len() > 1 {
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
            let (shift, q_top) = div::div_rem_unnormalized_in_place(x, y, &mut memory);
            let (mut r, mut q_lo) = x.split_at_mut(y.len());
            debug_assert_zero!(shift::shr_in_place(y, shift));
            debug_assert_zero!(shift::shr_in_place(r, shift));
            r = trim_leading_zeros(r);
            if q_top == 0 {
                q_lo = trim_leading_zeros(q_lo);
            }

            // update coefficient t0 += q*t1
            let qt1_len = q_lo.len() + t1_len;
            let mut t_carry = mul::add_signed_mul(
                &mut t0[..qt1_len],
                Sign::Positive,
                q_lo,
                &t1[..t1_len],
                &mut memory,
            ) as Word;
            if q_top > 0 {
                t_carry += mul::add_mul_word_in_place(
                    &mut t0[q_lo.len()..qt1_len.min(lhs_len)],
                    q_top,
                    &t1[..t1_len],
                );
            }
            if t_carry > 0 {
                t0[qt1_len] = t_carry;
                t0_len = qt1_len + 1;
            } else {
                t0_len = locate_top_word_plus_one(&t0[..qt1_len]);
            }

            // swap: (x, y) = (y, r)
            x = mem::replace(&mut y, r);
            mem::swap(&mut t0, &mut t1);
            mem::swap(&mut t0_len, &mut t1_len);
            swapped = !swapped;
        } else {
            // The lehmer guess succeeded, use the coefficients to update x, y and t0, t1
            lehmer_step(x, y, a, b, c, d);
            x = trim_leading_zeros(x);
            y = trim_leading_zeros(y);

            // (t0, t1) = (a*t0 - b*t1, d*t1 - c*t0), here we do unsigned operations.
            let tmax_len = t0_len.max(t1_len);
            let (t0_carry, t1_carry) = lehmer_ext_step(t0, t1, tmax_len, a, b, c, d);
            if t0_carry > 0 {
                t0[tmax_len] = t0_carry;
                t0_len = tmax_len + 1;
            } else {
                t0_len = locate_top_word_plus_one(&t0[..tmax_len]);
            }
            if t1_carry > 0 {
                t1[tmax_len] = t1_carry;
                t1_len = tmax_len + 1;
            } else {
                t1_len = locate_top_word_plus_one(&t1[..tmax_len]);
            }

            // make sure x > y
            if cmp_in_place(x, y).is_le() {
                mem::swap(&mut x, &mut y);
                mem::swap(&mut t0, &mut t1);
                mem::swap(&mut t0_len, &mut t1_len);
                swapped = !swapped;
            }
        }
    }

    // If y is zero, then the gcd result is in x now.
    // Note that y.len() == 0 is equivalent to y == 0, which is guaranteed by trim_leading_zeros.
    if y.is_empty() {
        unsafe {
            if !swapped {
                // if not swapped, then x is originated from lhs, copy it to rhs
                debug_assert!(x.as_ptr() == lhs_ptr);
                debug_assert!(x.len() <= rhs_len);
                ptr::copy_nonoverlapping(x.as_ptr(), rhs_ptr, x.len());
            }
            ptr::copy_nonoverlapping(t0.as_ptr(), lhs_ptr, t0_len);
        }
        let sign = if swapped {
            Sign::Positive
        } else {
            Sign::Negative
        };
        return (x.len(), t0_len, sign);
    }

    // before forwarding to single word gcd, first reduce x by y:
    // x_word = x % y; x /= y
    let y_word = *y.first().unwrap();
    let x_word = div::div_by_word_in_place(x, y_word);
    t0_len = x.len() + t1_len;
    debug_assert_zero!(mul::add_signed_mul(
        &mut t0[..t0_len],
        Sign::Positive,
        x,
        &t1[..t1_len],
        &mut memory,
    ));
    t0_len = locate_top_word_plus_one(&t0[..t0_len]);

    // forward to single word gcd
    let (g_word, cx, cy) = x_word.gcd_ext(y_word);
    swapped ^= cx < 0;

    // let lhs stores |b| = |cx| * t0 + |cy| * t1
    // by now, number of words in |b| should be close to lhs
    let (lhs, rhs) = unsafe {
        // SAFETY: we don't hold any reference to lhs and rhs now, so there will be no
        // data racing. The pointer and length are from the original slice, so the slice
        // will be valid.
        (
            slice::from_raw_parts_mut(lhs_ptr, lhs_len),
            slice::from_raw_parts_mut(rhs_ptr, rhs_len),
        )
    };
    *rhs.first_mut().unwrap() = g_word;
    lhs.fill(0);

    let (cx, cy) = (cx.unsigned_abs(), cy.unsigned_abs());
    debug_assert_zero!(mul::add_mul_word_in_place(lhs, cx, &t0[..t0_len]));
    debug_assert_zero!(mul::add_mul_word_in_place(lhs, cy, &t1[..t1_len]));
    let sign = if swapped {
        Sign::Positive
    } else {
        Sign::Negative
    };
    (1, locate_top_word_plus_one(lhs), sign)
}
