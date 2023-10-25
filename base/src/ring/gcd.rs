use super::{ExtendedGcd, Gcd};
use core::mem::replace;

trait UncheckedGcd<Rhs = Self> {
    type Output;

    /// GCD with assumptions that (1) at least one of the input is not zero, (2) the
    /// two operands are relatively close, (3) the factor 2 is removed from the operands.
    /// For internal use only.
    fn unchecked_gcd(self, rhs: Rhs) -> Self::Output;
}

trait UncheckedExtendedGcd<Rhs = Self> {
    type OutputGcd;
    type OutputCoeff;

    /// Extended GCD with assumptions that (1) at least one of the input is not zero,
    /// (2) the first oprand is larger than the second. For internal use only.
    fn unchecked_gcd_ext(self, rhs: Rhs)
        -> (Self::OutputGcd, Self::OutputCoeff, Self::OutputCoeff);
}

macro_rules! impl_unchecked_gcd_ops_prim {
    ($($U:ty | $I:ty;)*) => {$(
        impl UncheckedGcd for $U {
            type Output = $U;

            #[inline]
            fn unchecked_gcd(self, rhs: Self) -> Self::Output {
                debug_assert!(self | rhs > 0);
                debug_assert!(self & rhs & 1 > 0);

                let (mut a, mut b) = (self, rhs);

                // the binary GCD algorithm
                while a != b {
                    if a > b {
                        a -= b;
                        a >>= a.trailing_zeros();
                    } else {
                        b -= a;
                        b >>= b.trailing_zeros();
                    }
                }
                a
            }
        }
        impl UncheckedExtendedGcd for $U {
            type OutputGcd = $U;
            type OutputCoeff = $I;

            #[inline]
            fn unchecked_gcd_ext(self, rhs: $U) -> ($U, $I, $I) {
                debug_assert!(self | rhs > 0);
                debug_assert!(self >= rhs);

                // keep r = self * s + rhs * t
                let (mut last_r, mut r) = (self, rhs);
                let (mut last_s, mut s) = (1, 0);
                let (mut last_t, mut t) = (0, 1);

                loop {
                    let quo = last_r / r;
                    let new_r = last_r - quo * r;
                    if new_r == 0 {
                        return (r, s, t)
                    }
                    last_r = replace(&mut r, new_r);
                    let new_s = last_s - quo as $I * s;
                    last_s = replace(&mut s, new_s);
                    let new_t = last_t - quo as $I * t;
                    last_t = replace(&mut t, new_t);
                }

            }
        }
    )*};
    ($($U:ty | $I:ty => $HU:ty | $HI:ty;)*) => {$( // treat the integers as two parts
        impl UncheckedGcd for $U {
            type Output = $U;

            fn unchecked_gcd(self, rhs: Self) -> Self::Output {
                debug_assert!(self | rhs > 0);
                debug_assert!(self & rhs & 1 > 0);
                let (mut a, mut b) = (self, rhs);

                // the binary GCD algorithm
                while a != b {
                    if (a | b) >> <$HU>::BITS == 0 {
                        // forward to single width int
                        return (a as $HU).unchecked_gcd(b as $HU) as $U;
                    }
                    if a > b {
                        a -= b;
                        a >>= a.trailing_zeros();
                    } else {
                        b -= a;
                        b >>= b.trailing_zeros();
                    }
                }
                a
            }
        }
        impl UncheckedExtendedGcd for $U {
            type OutputGcd = $U;
            type OutputCoeff = $I;

            fn unchecked_gcd_ext(self, rhs: $U) -> ($U, $I, $I) {
                debug_assert!(self | rhs > 0);
                debug_assert!(self >= rhs);

                // keep r = self * s + rhs * t
                let (mut last_r, mut r) = (self, rhs);
                let (mut last_s, mut s) = (1, 0);
                let (mut last_t, mut t) = (0, 1);

                // normal euclidean algorithm on double width integers
                while r >> <$HU>::BITS > 0 {
                    let quo = last_r / r;
                    let new_r = last_r - quo * r;
                    if new_r == 0 {
                        return (r, s, t);
                    }
                    last_r = replace(&mut r, new_r);
                    let new_s = last_s - quo as $I * s;
                    last_s = replace(&mut s, new_s);
                    let new_t = last_t - quo as $I * t;
                    last_t = replace(&mut t, new_t);
                }

                // reduce double by single
                let r = r as $HU;
                let quo = last_r / r as $U;
                let new_r = (last_r - quo * r as $U) as $HU;
                if new_r == 0 {
                    return (r as $U, s, t);
                }
                let new_s = last_s - quo as $I * s;
                let new_t = last_t - quo as $I * t;

                // forward to single width int
                let (g, cx, cy) = r.unchecked_gcd_ext(new_r);
                let (cx, cy) = (cx as $I, cy as $I);
                (g as $U, &cx * s + &cy * new_s, cx * t + cy * new_t)
            }
        }
    )*}
}
impl_unchecked_gcd_ops_prim!(u8 | i8; u16 | i16; usize | isize;);
#[cfg(target_pointer_width = "16")]
impl_unchecked_gcd_ops_prim!(u32 | i32 => u16 | i16; u64 | i64 => u32 | i32; u128 | i128 => u64 | i64;);
#[cfg(target_pointer_width = "32")]
impl_unchecked_gcd_ops_prim!(u32 | i32;);
#[cfg(target_pointer_width = "32")]
impl_unchecked_gcd_ops_prim!(u64 | i64 => u32 | i32; u128 | i128 => u64 | u64;);
#[cfg(target_pointer_width = "64")]
impl_unchecked_gcd_ops_prim!(u32 | i32; u64 | i64;);
#[cfg(target_pointer_width = "64")]
impl_unchecked_gcd_ops_prim!(u128 | i128 => u64 | i64;);

macro_rules! impl_gcd_ops_prim {
    ($($U:ty | $I:ty;)*) => {$(
        impl Gcd for $U {
            type Output = $U;

            #[inline]
            fn gcd(self, rhs: Self) -> Self::Output {
                let (mut a, mut b) = (self, rhs);
                if a == 0 || b == 0 {
                    if a == 0 && b == 0 {
                        panic_gcd_0_0();
                    }
                    return a | b;
                }

                // find common factors of 2
                let shift = (a | b).trailing_zeros();
                a >>= a.trailing_zeros();
                b >>= b.trailing_zeros();

                // reduce by division if the difference between operands is large
                let (za, zb) = (a.leading_zeros(), b.leading_zeros());
                const GCD_BIT_DIFF_THRESHOLD: u32 = 3;
                if za > zb.wrapping_add(GCD_BIT_DIFF_THRESHOLD) {
                    let r = b % a;
                    if r == 0 {
                        return a << shift;
                    } else {
                        b = r >> r.trailing_zeros();
                    }
                } else if zb > za.wrapping_add(4) {
                    let r = a % b;
                    if r == 0 {
                        return b << shift;
                    } else {
                        a = r >> r.trailing_zeros();
                    }
                }

                // forward to the gcd algorithm
                a.unchecked_gcd(b) << shift
            }
        }

        impl ExtendedGcd for $U {
            type OutputGcd = $U;
            type OutputCoeff = $I;

            #[inline]
            fn gcd_ext(self, rhs: $U) -> ($U, $I, $I) {
                let (mut a, mut b) = (self, rhs);

                // check if zero inputs
                match (a == 0, b == 0) {
                    (true, true) => panic_gcd_0_0(),
                    (true, false) => return (b, 0, 1),
                    (false, true) => return (a, 1, 0),
                    _ => {}
                }

                // find common factors of 2
                let shift = (a | b).trailing_zeros();
                a >>= shift;
                b >>= shift;

                // make sure a is larger than b
                if a >= b {
                    if b == 1 {
                        // this shortcut eliminates the overflow when a = <$T>::MAX and b = 1
                        (1 << shift, 0, 1)
                    } else {
                        // forward to the gcd algorithm
                        let (g, ca, cb) = a.unchecked_gcd_ext(b);
                        (g << shift, ca, cb)
                    }
                } else {
                    if a == 1 {
                        (1 << shift, 1, 0)
                    } else {
                        let (g, cb, ca) = b.unchecked_gcd_ext(a);
                        (g << shift, ca, cb)
                    }
                }
            }
        }
    )*}
}
impl_gcd_ops_prim!(u8 | i8; u16 | i16; u32 | i32; u64 | i64; u128 | i128; usize | isize;);

fn panic_gcd_0_0() -> ! {
    panic!("the greatest common divisor is not defined between zeros!")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple() {
        assert_eq!(12u8.gcd(18), 6);
        assert_eq!(16u16.gcd(2032), 16);
        assert_eq!(0x40000000u32.gcd(0xcfd41b91), 1);
        assert_eq!(
            0x80000000000000000000000000000000u128.gcd(0x6f32f1ef8b18a2bc3cea59789c79d441),
            1
        );
        assert_eq!(
            79901280795560547607793891992771245827u128.gcd(27442821378946980402542540754159585749),
            1
        );

        let result = 12u8.gcd_ext(18);
        assert_eq!(result, (6, -1, 1));
        let result = 16u16.gcd_ext(2032);
        assert_eq!(result, (16, 1, 0));
        let result = 0x40000000u32.gcd_ext(0xcfd41b91);
        assert_eq!(result, (1, -569926925, 175506801));
        let result =
            0x80000000000000000000000000000000u128.gcd_ext(0x6f32f1ef8b18a2bc3cea59789c79d441);
        assert_eq!(
            result,
            (
                1,
                59127885930508821681098646892310825630,
                -68061485417298041807799738471800882239
            )
        );
    }
}
