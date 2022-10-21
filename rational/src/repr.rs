use core::mem;
use dashu_base::Gcd;
use dashu_int::{IBig, UBig};

pub struct Repr {
    pub(crate) numerator: IBig,
    pub(crate) denominator: UBig,
}

impl Repr {
    /// Remove the common factors between numerator and denominator
    pub fn reduce(&mut self) {
        let (sign, n) = mem::take(&mut self.numerator).into_parts();
        let g = (&n).gcd(&self.denominator);
        self.denominator /= &g;
        self.numerator = IBig::from_parts(sign, n / g);
    }

    /// Remove only common factor of power of 2, which is cheap
    pub fn reduce2(&mut self) {
        let n_zeros = self.numerator.trailing_zeros().unwrap_or_default();
        let d_zeros = self.denominator.trailing_zeros().unwrap();
        let zeros = n_zeros.min(d_zeros);

        if zeros > 0 {
            self.numerator >>= zeros;
            self.denominator >>= zeros;
        }
    }
}
