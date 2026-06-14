//! NTT-based squaring for very large integers.
//!
//! Delegates to the NTT infrastructure in [`crate::mul::ntt`], specialized
//! for squaring: one forward transform instead of two, pointwise square
//! instead of multiply.

use crate::{
    add,
    arch::word::{SignedWord, Word},
    memory::Memory,
    mul::ntt::{self, NttGeometry},
    Sign::{self, *},
};

/// `c += sign * a²` with a.len() == c.len()/2.
///
/// Returns carry.
#[must_use]
#[inline]
pub(crate) fn add_signed_sqr_same_len(
    c: &mut [Word],
    sign: Sign,
    a: &[Word],
    memory: &mut Memory,
) -> SignedWord {
    let n = a.len();
    debug_assert!(c.len() == 2 * n);
    add_signed_sqr_conv(c, sign, a, memory)
}

/// Core NTT squaring: transform `a` once, pointwise-square, inverse.
fn add_signed_sqr_conv(
    c: &mut [Word],
    sign: Sign,
    a: &[Word],
    memory: &mut Memory,
) -> SignedWord {
    use crate::arch::ntt::{Lane, OMEGA_MAX, P0, P1, P2};
    use crate::mul::ntt::{bit_len, coeff_count, do_crt, transform};

    let la = a.len();
    debug_assert!(la > 0);
    let (b_pack, nn, k_eff) = ntt::select_params(la, la);
    let la_bits = bit_len(a);
    debug_assert!(la_bits > 0);

    let coeffs_a = coeff_count(la_bits, b_pack);
    let output_coeffs = 2 * coeffs_a - 1;
    let out_words = 2 * la;

    let geom = NttGeometry {
        nn,
        b_pack,
        k_eff,
        output_coeffs,
    };

    // Allocate: product, residues, a_lane, twiddle buffers.
    let (prod, mut m) = memory.allocate_slice_fill::<Word>(out_words, 0);
    let (residues, mut m) = m.allocate_slice_fill::<Lane>(k_eff * nn, 0);
    let (a_lane, mut m) = m.allocate_slice_fill::<Lane>(nn, 0);
    let (fwd_twiddles, mut m) = m.allocate_slice_fill::<Lane>(nn / 2, 0);
    let (inv_twiddles, _) = m.allocate_slice_fill::<Lane>(nn / 2, 0);

    for (pi, &omega) in OMEGA_MAX.iter().enumerate().take(k_eff) {
        match pi {
            0 => {
                transform::precompute_twiddles(fwd_twiddles, nn, omega, false, &P0);
                transform::precompute_twiddles(inv_twiddles, nn, omega, true, &P0);
                process_prime_square(a, a_lane, fwd_twiddles, inv_twiddles, residues, pi, &geom, &P0);
            }
            1 => {
                transform::precompute_twiddles(fwd_twiddles, nn, omega, false, &P1);
                transform::precompute_twiddles(inv_twiddles, nn, omega, true, &P1);
                process_prime_square(a, a_lane, fwd_twiddles, inv_twiddles, residues, pi, &geom, &P1);
            }
            2 => {
                transform::precompute_twiddles(fwd_twiddles, nn, omega, false, &P2);
                transform::precompute_twiddles(inv_twiddles, nn, omega, true, &P2);
                process_prime_square(a, a_lane, fwd_twiddles, inv_twiddles, residues, pi, &geom, &P2);
            }
            _ => unreachable!(),
        }
    }

    use crate::arch::ntt::{MODULI, CRT_INV_IJ};
    do_crt::<crate::arch::word::TripleWord>(prod, residues, &geom, &MODULI, &CRT_INV_IJ);

    match sign {
        Positive => add::add_signed_in_place(&mut c[..out_words], Positive, &prod[..out_words]),
        Negative => add::add_signed_in_place(&mut c[..out_words], Negative, &prod[..out_words]),
    }
}

/// Per-prime NTT squaring pipeline.
///
/// Packs + Montgomery converts + forward transforms `a`, pointwise
/// squares, inverse transforms, and stores the residues.
#[allow(clippy::too_many_arguments)]
fn process_prime_square<R: num_modular::Reducer<crate::arch::ntt::Lane>>(
    a: &[Word],
    a_lane: &mut [crate::arch::ntt::Lane],
    fwd_twiddles: &[crate::arch::ntt::Lane],
    inv_twiddles: &[crate::arch::ntt::Lane],
    residues: &mut [crate::arch::ntt::Lane],
    pi: usize,
    geom: &NttGeometry,
    r: &R,
) {
    use crate::mul::ntt::{pack, transform};

    let nn = geom.nn;

    // Transform a
    pack::pack(a_lane, a, geom.b_pack, nn);
    for c in a_lane[..nn].iter_mut() {
        *c = r.transform(*c);
    }
    transform::bit_reverse(&mut a_lane[..nn]);
    transform::forward(&mut a_lane[..nn], fwd_twiddles, r);

    // Pointwise square (no separate b̂ needed)
    for c in a_lane[..nn].iter_mut() {
        *c = r.sqr(*c);
    }

    // Inverse transform
    transform::inverse(&mut a_lane[..nn], inv_twiddles, r);
    for c in a_lane[..nn].iter_mut() {
        *c = r.residue(*c);
    }

    let offset = pi * nn;
    residues[offset..offset + nn].copy_from_slice(&a_lane[..nn]);
}
