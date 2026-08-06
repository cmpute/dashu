//! Implement zeroize traits.

use crate::{cbig::CBig, repr::Context};
use dashu_float::round::Round;
use dashu_int::Word;
use zeroize::Zeroize;

impl<R: Round> Zeroize for Context<R> {
    #[inline]
    fn zeroize(&mut self) {
        self.0.zeroize();
    }
}

impl<R: Round, const B: Word> Zeroize for CBig<R, B> {
    #[inline]
    fn zeroize(&mut self) {
        self.re.zeroize();
        self.im.zeroize();
        self.context.zeroize();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dashu_float::round::mode::HalfEven;

    #[test]
    fn zeroize_clears_cbig() {
        type F = dashu_float::FBig<HalfEven, 10>;
        let mut z = CBig::<HalfEven, 10>::from_parts(F::from(3u8), F::from(4u8));
        z.zeroize();
        let re_zero = z.re().is_pos_zero() || z.re().is_neg_zero();
        let im_zero = z.im().is_pos_zero() || z.im().is_neg_zero();
        assert!(re_zero && im_zero, "parts zeroed: {z:?}");
    }
}
