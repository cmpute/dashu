use core::ops::Add;
use dashu_base::Gcd;

use crate::rbig::{RBig, Relaxed};

impl Add for RBig {
    type Output = RBig;
    fn add(self, rhs: Self) -> Self::Output {
        let (a, b) = self.into_parts();
        let (c, d) = rhs.into_parts();
        let g_bd = (&b).gcd(&d);

        // a/b + c/d = (ad + bc)/bd
        if g_bd.is_one() {
            let left = a * &d;
            let right = c * &b;
            RBig::from_parts(left + right, b * d)
        } else {
            let ddg = d / &g_bd;
            let left = &ddg * a;
            let right = &b / g_bd * c;
            RBig::from_parts(left + right, b * ddg)
        }
    }
}

impl<'r> Add<&'r RBig> for RBig {
    type Output = RBig;
    fn add(self, rhs: &RBig) -> Self::Output {
        let (a, b) = self.into_parts();
        let (c, d) = (rhs.numerator(), rhs.denominator());
        let g_bd = (&b).gcd(d);

        // a/b + c/d = (ad + bc)/bd
        if g_bd.is_one() {
            let left = a * d;
            let right = c * &b;
            RBig::from_parts(left + right, b * d)
        } else {
            let bdg = b / &g_bd;
            let right = &bdg * c;
            let left = d / g_bd * a;
            RBig::from_parts(left + right, bdg * d)
        }
    }
}

impl<'l> Add<RBig> for &'l RBig {
    type Output = RBig;
    fn add(self, rhs: RBig) -> Self::Output {
        let (a, b) = (self.numerator(), self.denominator());
        let (c, d) = rhs.into_parts();
        let g_bd = (b).gcd(&d);

        // a/b + c/d = (ad + bc)/bd
        if g_bd.is_one() {
            let left = a * &d;
            let right = c * b;
            RBig::from_parts(left + right, b * d)
        } else {
            let ddg = d / &g_bd;
            let left = &ddg * a;
            let right = b / g_bd * c;
            RBig::from_parts(left + right, b * ddg)
        }
    }
}

impl<'l, 'r> Add<&'r RBig> for &'l RBig {
    type Output = RBig;
    fn add(self, rhs: &RBig) -> Self::Output {
        let (a, b) = (self.numerator(), self.denominator());
        let (c, d) = (rhs.numerator(), rhs.denominator());
        let g_bd = b.gcd(d);

        // a/b + c/d = (ad + bc)/bd
        if g_bd.is_one() {
            let left = a * d;
            let right = c * b;
            RBig::from_parts(left + right, b * d)
        } else {
            let ddg = d / &g_bd;
            let left = &ddg * a;
            let right = b / g_bd * c;
            RBig::from_parts(left + right, b * ddg)
        }
    }
}

impl Add for Relaxed {
    type Output = Relaxed;
    fn add(self, rhs: Self) -> Self::Output {
        let (a, b) = self.into_parts();
        let (c, d) = rhs.into_parts();

        // a/b + c/d = (ad + bc)/bd
        let left = a * &d;
        let right = c * &b;
        Relaxed::from_parts(left + right, b * d)
    }
}
