use crate::{arch::{self, word::Word}, mul, math::mul_add_2carry, primitive::{double_word, split_dword}};

pub fn square<'a>(
    b: &mut [Word],
    a: &'a [Word]
) {
    debug_assert!(b.len() == a.len() * 2);

    /*
     * A simple algorithm for squaring
     * 
     * take a = a0 + a1*B + a2*B^2 + a3*B^3 as an example
     * to calculate a^2 = (a0 + a1*B + a2*B^2 + a3*B^3) ^ 2
     * 
     * first
     * b += a0 * (a1 + a2*B + a3*B^2) * B
     * b += a1 * (a2 + a3*B) * B^3
     * b += a2 * a3 * B^5

     * then
     * b = b * 2 + (a0^2 + a1^2*B^2 + a2^2*B^4 + a3^2*B^6)
     * the square and shifting can be fused in a single run
     *
     */

    // first step (triangular part)
    let mut c0 = false;
    for (i, m) in a.iter().enumerate() {
        let offset = i * 2 + 1;
        let carry = mul::add_mul_word_same_len_in_place(&mut b[offset..offset + a.len() - i], *m, &a[i..]);
        let (carry, carry_next) = arch::add::add_with_carry(*b.last().unwrap(), carry, c0);
        *b.last_mut().unwrap() = carry;
        c0 = carry_next;
    }

    // second step (diagonal part)
    let (mut c1, mut c2) = (false, false);
    for (m, b01) in a.iter().zip(b.chunks_exact_mut(2)) {
        let b0 = b01.first().unwrap();
        let b1 = b01.last().unwrap();

        // new [b0, b1] = m^2 + 2 * [b0, b1] + c1 + c2
        let (s0, s1) = mul_add_2carry(*m, *m, *b0, *b0);
        let s = double_word(s0, s1);
        let wb1 = double_word(0, *b1);
        let (s, oc1) = s.overflowing_add(wb1 + c1 as u128);
        let (s, oc2) = s.overflowing_add(wb1 + c2 as u128);
        let (s0, s1) = split_dword(s);

        *b01.first_mut().unwrap() = s0;
        *b01.last_mut().unwrap() = s1;
        c1 = oc1;
        c2 = oc2;
    }

    // aggregate carry bits
    *b.last_mut().unwrap() += c0 as Word + c1 as Word + c2 as Word;
}
