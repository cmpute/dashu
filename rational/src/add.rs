use core::ops::Add;
use dashu_base::Gcd;

use crate::rbig::{RBig, Relaxed};

impl Add for RBig {
    type Output = RBig;
    fn add(self, rhs: Self) -> Self::Output {
        // a/b + c/d = (ad + bc)/bd
        let (a, b) = self.into_parts();
        let (c, d) = rhs.into_parts();

        // let g_bd = b.gcd(d);
        unimplemented!()
    }
}

impl Add for Relaxed {
    type Output = Relaxed;
    fn add(self, rhs: Self) -> Self::Output {
        unimplemented!()
    }
}

