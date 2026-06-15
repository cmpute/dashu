use crate::{
    arch::{
        self,
        word::{DoubleWord, Word},
    },
    math::{mul_add_2carry, mul_add_carry},
    primitive::{double_word, split_dword},
};

pub fn square(b: &mut [Word], a: &[Word]) {
    debug_assert!(b.len() == a.len() * 2);
    let n = a.len();
    // `b` is zero on entry (contract from sqr::sqr).
    //
    // a^2 = (diagonal) + 2 * (upper triangle)
    // The upper triangle is accumulated first, then doubled in place while the
    // diagonal a[i]^2 terms are added, fusing the shift into a single pass.

    // ---- first step: upper (off-diagonal) triangle ----
    //
    // For each pair of source limbs (a[i], a[i+1]) we add their off-diagonal
    // products against the shared suffix a[i+2..]:
    //   * the lone corner product a[i] * a[i+1] (column 2i+1), and
    //   * (a[i] + a[i+1]*B) * a[i+2..] via a two-word mpn_addmul_2-style kernel
    //     (two independent mul-accumulate chains, one accumulator load/store
    //     per two multiplier limbs), mirroring the multiplication basecase.
    // This halves the accumulator memory traffic of the triangle versus a
    // one-limb-per-row sweep.
    //
    // Carries are propagated in-place: each pair's high product words land at
    // b[n+i], b[n+i+1] and the next pair's kernel sweep re-reads b[n+i+1] and
    // propagates further, so only a single pending carry word `cy` ever escapes
    // a sweep. (The triangle T < B^(2n-1) for any realistic n, so `cy` resolves
    // to 0; it is folded into the final aggregation exactly like the old code's
    // trailing carry.)
    let mut cy: Word = 0;
    let mut i = 0;
    while i + 2 < n {
        let m0 = a[i];
        let m1 = a[i + 1];
        let rhs = &a[i + 2..];
        let base = 2 * i + 2; // kernel covers columns 2i+2 .. n+i

        // corner a[i]*a[i+1] at column 2i+1 (= base-1)
        let (lo, hi) = mul_add_carry(m0, m1, 0);
        let (v, c1) = b[base - 1].overflowing_add(lo);
        b[base - 1] = v;
        // hi + c1 fits in a Word (hi <= Word::MAX-1, c1 in {0,1}); seed the
        // kernel's low carry chain with it so no separate ripple is needed.
        let init_lo = hi.wrapping_add(Word::from(c1));

        // two-word kernel: b[base..] += (m0 + m1*B) * rhs
        let mut carry_lo = init_lo;
        let mut carry_hi: Word = 0;
        for (x, &y) in b[base..base + rhs.len()].iter_mut().zip(rhs.iter()) {
            (*x, carry_lo) = mul_add_2carry(y, m0, *x, carry_lo);
            (carry_lo, carry_hi) = mul_add_2carry(y, m1, carry_lo, carry_hi);
        }

        // add the high product words plus the pending carry at columns n+i, n+i+1
        let p = base + rhs.len(); // = n + i
        let (s, c) = arch::add::add_with_carry(b[p], carry_lo, cy);
        b[p] = s;
        let (s, c2) = arch::add::add_with_carry(b[p + 1], carry_hi, Word::from(c));
        b[p + 1] = s;
        cy = Word::from(c2);

        i += 2;
    }
    // leftover single row when n is even: a[n-2]*a[n-1] at column 2n-3
    if i == n - 2 {
        let (lo, hi) = mul_add_carry(a[i], a[i + 1], 0);
        let base = 2 * i + 1; // = 2n-3
        let (v, c1) = b[base].overflowing_add(lo);
        b[base] = v;
        // column 2n-2 = base+1 also receives the pending carry `cy`
        let (s, c) = arch::add::add_with_carry(b[base + 1], hi, Word::from(c1));
        let (s, c2) = arch::add::add_with_carry(s, cy, Word::from(c));
        b[base + 1] = s;
        cy = Word::from(c2);
    }

    // ---- second step: double the triangle and add the diagonal a[i]^2 ----
    let (mut c1, mut c2) = (false, false);
    for (m, b01) in a.iter().zip(b.chunks_exact_mut(2)) {
        let b0 = b01.first().unwrap();
        let b1 = b01.last().unwrap();

        // new [b0, b1] = m^2 + 2 * [b0, b1] + c1 + c2
        let (s0, s1) = mul_add_2carry(*m, *m, *b0, *b0);
        let s = double_word(s0, s1);
        let wb1 = double_word(0, *b1);
        let (s, oc1) = s.overflowing_add(wb1 + c1 as DoubleWord);
        let (s, oc2) = s.overflowing_add(wb1 + c2 as DoubleWord);
        let (s0, s1) = split_dword(s);

        *b01.first_mut().unwrap() = s0;
        *b01.last_mut().unwrap() = s1;
        c1 = oc1;
        c2 = oc2;
    }

    // aggregate carry bits (cy is the triangle's trailing carry, ~always 0)
    *b.last_mut().unwrap() += cy + c1 as Word + c2 as Word;
}
