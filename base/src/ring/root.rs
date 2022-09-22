use super::RootRem;
use crate::DivRem;

trait NormalizedRootRem: Sized {
    type OutputRoot;

    /// Square root with the normalized input such that highest or second
    /// highest bit are set. For internal use only.
    fn normalized_sqrt_rem(self) -> (Self::OutputRoot, Self);

    /// Cubic root with the normalized input such that at least one of the
    /// highest three bits are set. For internal use only.
    fn normalized_cbrt_rem(self) -> (Self::OutputRoot, Self);
}

// Estimations of normalized 1/sqrt(x) with 9 bits precision. Specifically
// (rsqrt_tab[i] + 0x100) / 0x200 ≈ (sqrt(32) / sqrt(32 + i))
const RSQRT_TAB: [u8; 96] = [
    0xfc, 0xf4, 0xed, 0xe6, 0xdf, 0xd9, 0xd3, 0xcd, 0xc7, 0xc2, 0xbc, 0xb7, 0xb2, 0xad, 0xa9, 0xa4,
    0xa0, 0x9c, 0x98, 0x94, 0x90, 0x8c, 0x88, 0x85, 0x81, 0x7e, 0x7b, 0x77, 0x74, 0x71, 0x6e, 0x6b,
    0x69, 0x66, 0x63, 0x61, 0x5e, 0x5b, 0x59, 0x57, 0x54, 0x52, 0x50, 0x4d, 0x4b, 0x49, 0x47, 0x45,
    0x43, 0x41, 0x3f, 0x3d, 0x3b, 0x39, 0x37, 0x36, 0x34, 0x32, 0x30, 0x2f, 0x2d, 0x2c, 0x2a, 0x28,
    0x27, 0x25, 0x24, 0x22, 0x21, 0x1f, 0x1e, 0x1d, 0x1b, 0x1a, 0x19, 0x17, 0x16, 0x15, 0x14, 0x12,
    0x11, 0x10, 0x0f, 0x0d, 0x0c, 0x0b, 0x0a, 0x09, 0x08, 0x07, 0x06, 0x05, 0x04, 0x03, 0x02, 0x01,
];

// Estimations of normalized 1/cbrt(x) with 9 bits precision. Specifically
// (rcbrt_tab[i] + 0x100) / 0x200 ≈ (cbrt(8) / cbrt(8 + i))
const RCBRT_TAB: [u8; 56] = [
    0xf6, 0xe4, 0xd4, 0xc6, 0xb9, 0xae, 0xa4, 0x9b, 0x92, 0x8a, 0x83, 0x7c, 0x76, 0x70, 0x6b, 0x66,
    0x61, 0x5c, 0x57, 0x53, 0x4f, 0x4b, 0x48, 0x44, 0x41, 0x3e, 0x3b, 0x38, 0x35, 0x32, 0x2f, 0x2d,
    0x2a, 0x28, 0x25, 0x23, 0x21, 0x1f, 0x1d, 0x1b, 0x19, 0x17, 0x15, 0x13, 0x11, 0x10, 0x0e, 0x0c,
    0x0b, 0x09, 0x08, 0x06, 0x05, 0x03, 0x02, 0x01,
];

/// Fix the estimation error of `sqrt(n)`, `s` is the (mutable) estimation variable,
/// This procedure requires s <= `sqrt(n)`, returns the error `n - s^2`.
macro_rules! fix_sqrt_error {
    ($t:ty, $n:ident, $s:ident) => {{
        let mut e = $n - ($s as $t).pow(2);
        let mut elim = 2 * $s as $t + 1;
        while e >= elim {
            $s += 1;
            e -= elim;
            elim += 2;
        }
        e
    }};
}

/// Fix the estimation error of `cbrt(n)`, `c` is the (mutable) estimation variable,
/// This procedure requires c <= `cbrt(n)`, returns the error `n - c^3`.
macro_rules! fix_cbrt_error {
    ($t:ty, $n:ident, $c:ident) => {{
        let cc = ($c as $t).pow(2);
        let mut e = $n - cc * ($c as $t);
        let mut elim = 3 * (cc + $c as $t) + 1;
        while e >= elim {
            $c += 1;
            e -= elim;
            elim += 6 * ($c as $t);
        }
        e
    }};
}

impl NormalizedRootRem for u16 {
    type OutputRoot = u8;

    fn normalized_sqrt_rem(self) -> (u8, u16) {
        debug_assert!(self.leading_zeros() <= 1);

        // retrieved r ≈ √32 / √(n >> 9) * 0x200 = 1 / √(n >> 14) * 2^9 = 2^16 / √n.
        let r = 0x100 | RSQRT_TAB[(self >> 9) as usize - 32] as u32; // 9 bits
        let s = (r * self as u32) >> 16;
        let mut s = (s - 1) as u8; // to make sure s is an underestimate

        // then fix the estimation error
        let e = fix_sqrt_error!(u16, self, s);
        (s, e)
    }

    fn normalized_cbrt_rem(self) -> (u8, u16) {
        debug_assert!(self.leading_zeros() <= 2);
        
        // retrieved r ≈ ∛8 / ∛(n >> 9) * 0x200 = 1 / ∛(n >> 12) * 2^9 = 2^13 / ∛n.
        let adjust = self.leading_zeros() == 0;
        let r = 0x100 | RCBRT_TAB[(self >> (9 + (3 * adjust as u8))) as usize - 8] as u32; // 9 bits
        let r2 = (r * r) >> (2 + 2 * adjust as u8);
        let c = (r2 * self as u32) >> 24;
        let mut c = (c - 1) as u8; // to make sure c is an underestimate

        // step6: fix the estimation error, at most 2 steps are needed
        // if we use more bits to estimate the initial guess, less steps can be required
        let e = fix_cbrt_error!(u16, self, c);
        (c, e)
    }
}

/// Get the high part of widening mul on two u16 integers
#[inline]
fn wmul16_hi(a: u16, b: u16) -> u16 {
    (((a as u32) * (b as u32)) >> 16) as u16
}

impl NormalizedRootRem for u32 {
    type OutputRoot = u16;

    fn normalized_sqrt_rem(self) -> (u16, u32) {
        // Use newton's method on 1/sqrt(n)
        // x_{i+1} = x_i * (3 - n*x_i^2) / 2
        debug_assert!(self.leading_zeros() <= 1);

        // step1: lookup initial estimation of normalized 1/√n. The lookup table uses the highest 7 bits,
        // since the input is normalized, the lookup index must be larger than 2**(7-2) = 32.
        // then the retrieved r ≈ √32 / √(n >> 25) * 0x200 = 1 / √(n >> 30) / 2^9 = 2^24 / √n.
        let n16 = (self >> 16) as u16;
        let r = 0x100 | RSQRT_TAB[(n16 >> 9) as usize - 32] as u32; // 9 bits

        // step2: first Newton iteration (without dividing by 2)
        // r will be an estimation of 2^(24+6) / √n with 16 bits effective precision
        let r = ((3 * r as u16) << 5) - (wmul32_hi(self, r * r * r) >> 11) as u16; // 15 bits

        // step3: √n = x * 1/√n
        let r = r << 1; // normalize to 16 bits, now r estimates 2^31 / √n
        let mut s = wmul16_hi(r, n16) << 1;
        s -= 4; // to make sure s is an underestimate

        // step4: second Newton iteration on √n
        let e = self - (s as u32) * (s as u32);
        s += wmul16_hi((e >> 16) as u16, r);

        // step5: fix the estimation error, at most 2 steps are needed
        // if we use more bits to estimate the initial guess, less steps can be required
        let e = fix_sqrt_error!(u32, self, s);
        (s, e)
    }

    fn normalized_cbrt_rem(self) -> (u16, u32) {
        // Use newton's method on 1/cbrt(n)
        // x_{i+1} = x_i * (4 - n*x_i^3) / 3
        debug_assert!(self.leading_zeros() <= 2);

        // step1: lookup initial estimation of 1/∛x. The lookup table uses the highest 6 bits up to 30rd.
        // if the input is 32/31 bit, then shift it to 29/28 bit.
        // retrieved r ≈ ∛8 / ∛(n >> 24) * 0x200 = 1 / ∛(n >> 27) * 2^9 = 2^18 / ∛n.
        let adjust = self.leading_zeros() < 2;
        let n16 = (self >> (16 + 3 * adjust as u8)) as u16;
        let r = 0x100 | RCBRT_TAB[(n16 >> 8) as usize - 8] as u32; // 9 bits

        // step2: first Newton iteration
        // required shift = 18 * 3 - 11 - 16 * 2 - * 2 = 11
        // afterwards, r ≈ 2^(18+11-4) / ∛n
        let r3 = (r * r * r) >> 11;
        let t = (4 << 11) - wmul16_hi(n16, r3 as u16); // 13 bits
        let mut r = ((r * t as u32 / 3) >> 4) as u16; // 16 bits
        r >>= adjust as u8; // recover the adjustment if needed

        // step5: ∛x = x * (1/∛x)^2
        let r = r - 10; // to make sure c is an underestimate
        let mut c = wmul16_hi(r, wmul16_hi(r, (self >> 16) as u16)) >> 2;

        // step6: fix the estimation error, at most 2 steps are needed
        // if we use more bits to estimate the initial guess, less steps can be required
        let e = fix_cbrt_error!(u32, self, c);
        (c, e)
    }
}

/// Get the high part of widening mul on two u32 integers
#[inline]
fn wmul32_hi(a: u32, b: u32) -> u32 {
    (((a as u64) * (b as u64)) >> 32) as u32
}

impl NormalizedRootRem for u64 {
    type OutputRoot = u32;

    fn normalized_sqrt_rem(self) -> (u32, u64) {
        // Use newton's method on 1/sqrt(n)
        // x_{i+1} = x_i * (3 - n*x_i^2) / 2
        debug_assert!(self.leading_zeros() <= 1);

        // step1: lookup initial estimation of normalized 1/√n. The lookup table uses the highest 7 bits,
        // since the input is normalized, the lookup index must be larger than 2**(7-2) = 32.
        // then the retrieved r ≈ √32 / √(n >> 57) * 0x200 = 1 / √(n >> 62) / 2^9 = 2^40 / √n.
        let n32 = (self >> 32) as u32;
        let r = 0x100 | RSQRT_TAB[(n32 >> 25) as usize - 32] as u32; // 9 bits

        // step2: first Newton iteration (without dividing by 2)
        // afterwards, r ≈ 2^(40+22) / √n with 16 bits effective precision
        let r = ((3 * r) << 21) - wmul32_hi(n32, (r * r * r) << 5); // 31 bits

        // step3: second Newton iteration (without dividing by 2)
        // afterwards, r ≈ 2^(40+19) / √n with 32 bits effective precision
        let t = (3 << 28) - wmul32_hi(r, wmul32_hi(r, n32)); // 29 bits
        let r = wmul32_hi(r, t); // 28 bits

        // step4: √n = x * 1/√n
        let r = r << 4; // normalize to 32 bits, now r estimates 2^63 / √n
        let mut s = wmul32_hi(r, n32) << 1;
        s -= 10; // to make sure s is an underestimate

        // step5: third Newton iteration on √n
        let e = self - (s as u64) * (s as u64);
        s += wmul32_hi((e >> 32) as u32, r);

        // step6: fix the estimation error, at most 2 steps are needed
        // if we use more bits to estimate the initial guess, less steps can be required
        let e = fix_sqrt_error!(u64, self, s);
        (s, e)
    }

    fn normalized_cbrt_rem(self) -> (u32, u64) {
        // Use newton's method on 1/cbrt(n)
        // x_{i+1} = x_i * (4 - n*x_i^3) / 3
        debug_assert!(self.leading_zeros() <= 2);

        // step1: lookup initial estimation of 1/∛x. The lookup table uses the highest 6 bits up to 63rd.
        // if the input has 64 bits, then shift it to 61 bits.
        // retrieved r ≈ ∛8 / ∛(n >> 57) * 0x200 = 1 / ∛(n >> 60) * 2^9 = 2^29 / ∛n.
        let adjust = self.leading_zeros() == 0;
        let n32 = (self >> (32 + 3 * adjust as u8)) as u32;
        let r = 0x100 | RCBRT_TAB[(n32 >> 25) as usize - 8] as u32; // 9 bits

        // step2: first Newton iteration
        // required shift = 29 * 3 - 32 * 2 = 23
        // afterwards, r ≈ 2^(29+23) / ∛n = 2^52 / ∛n
        let t = (4 << 23) - wmul32_hi(n32, r * r * r);
        let r = r * (t / 3); // 32 bits

        // step3: second Newton iteration
        // required shift = 52 * 3 - 32 * 4 = 28
        // afterwards, r ≈ 2^(52+28-32) / ∛n = 2^48 / ∛n
        let t = (4 << 28) - wmul32_hi(r, wmul32_hi(r, wmul32_hi(r, n32)));
        let mut r = wmul32_hi(r, t) / 3; // 28 bits
        r >>= adjust as u8; // recover the adjustment if needed

        // step4: ∛x = x * (1/∛x)^2 = x * (2^48/∛x)^2 / 2^(32*3)
        let r = r - 1; // to make sure c is an underestimate
        let mut c = wmul32_hi(r, wmul32_hi(r, (self >> 32) as u32));

        // step5: fix the estimation error, at most 3 steps are needed
        // if we use more bits to estimate the initial guess, less steps can be required
        let e = fix_cbrt_error!(u64, self, c);
        (c, e)
    }
}

impl NormalizedRootRem for u128 {
    type OutputRoot = u64;

    fn normalized_sqrt_rem(self) -> (u64, u128) {
        debug_assert!(self.leading_zeros() <= 1);

        /* 
         * the "Karatsuba Square Root" algorithm:
         * assume n = a*B^2 + b1*B + b0, B=2^k, a has 2k bits
         * 1. calculate sqrt on high part:
         *     s1, r1 = sqrt_rem(a)
         * 2. estimate the root with low part
         *     q, u = div_rem(r1*B + b1, 2*s1)
         *     s = s1*B + q
         *     r = u*B + b0 - q^2
         *    at this step, since a is normalized, we have s1 >= B/2,
         *    therefore q <= (r1*B + b1) / B < r1 + 1 <= B
         * 
         * 3. if a3 is normalized, then s is either correct or 1 too big.
         *    r is negative in the latter case, needs adjustment
         *     if r < 0 {
         *         r += 2*s - 1
         *         s -= 1
         *     }
         * 
         * Reference: Zimmermann, P. (1999). Karatsuba square root (Doctoral dissertation, INRIA).
         * https://hal.inria.fr/inria-00072854/en/
         */

        // step1: calculate sqrt on high parts
        let (a, b) = (self >> u64::BITS, self & u64::MAX as u128);
        let (a, b) = (a as u64, b as u64);
        let (s1, r1) = a.normalized_sqrt_rem();

        // step2: estimate the result with low parts
        // note that r1 <= 2*s1 < 2^(KBITS + 1)
        const KBITS: u32 = u64::BITS / 2;
        let r0 = r1 << (KBITS - 1) | b >> (KBITS + 1);
        let (mut q, mut u) = r0.div_rem(s1 as u64);
        if q >> KBITS > 0 {
            // if q >= B, reduce the overestimate
            q -= 1;
            u += s1 as u64;
        }

        let mut s = (s1 as u64) << KBITS | q;
        let r = (u << (KBITS + 1)) | (b & ((1 << (KBITS + 1)) - 1));
        let q2 = q * q;
        let mut borrow = (u >> (KBITS - 1)) as i8 - (r < q2) as i8;
        let mut r = r.wrapping_sub(q2);

        // step3: adjustment
        if borrow < 0 {
            r = r.wrapping_add(s);
            borrow += (r < s) as i8;
            s -= 1;
            r = r.wrapping_add(s);
            borrow += (r < s) as i8;
        }
        (s, (borrow as u128) << u64::BITS | r as u128)
    }

    fn normalized_cbrt_rem(self) -> (u64, u128) {
        debug_assert!(self.leading_zeros() <= 2);

        /* 
         * the following algorithm is similar to the "Karatsuba Square Root" above:
         * assume n = a*B^3 + b2*B^2 + b1*B + b0, B=2^k, a has roughly 3k bits
         * 1. calculate cbrt on high part:
         *     c1, r1 = cbrt_rem(a)
         * 2. estimate the root with low part
         *     q, u = div_rem(r1*B + b2, 3*c1^2)
         *     c = c1*B + q
         *     r = u*B^2 + b1*B + b0 - 3*c1*q^2*B - q^3
         * 
         * 3. if a5 is normalized, then only few adjustments are needed 
         *     while r < 0 {
         *         r += 3*c^2 - 3*c + 1
         *         c -= 1
         *     }
         */

        // step1: calculate cbrt on high 62 bits
        let (c1, r1) = if self.leading_zeros() > 0 {
            // actually on high 65 bits
            let a = (self >> 63) as u64;
            let (mut c, _) = a.normalized_cbrt_rem();
            c >>= 1;
            (c, (a >> 3) - (c as u64).pow(3))
        } else {
            let a = (self >> 66) as u64;
            a.normalized_cbrt_rem()
        };

        // step2: estimate the root with low part
        const KBITS: u32 = 22;
        let r0 = ((r1 as u128) << KBITS) | (self >> (2 * KBITS) & ((1 << KBITS) - 1));
        let (q, u) = r0.div_rem(3 * (c1 as u128).pow(2));
        let mut c = ((c1 as u64) << KBITS) + (q as u64); // q might be larger than B
        // r = u*B^2 + b1*B + b0 - 3*c1*q^2*B - q^3
        let t1 = (u << (2 * KBITS)) | (self & ((1 << (2 * KBITS)) - 1));
        let t2 = ((3*(c1 as u128) << KBITS) + q) * q.pow(2);
        let mut r = t1 as i128 - t2 as i128;

        // step3: adjustment, finishes in at most 4 steps
        while r < 0 {
            r += 3 * (c as i128 - 1) * c as i128 + 1;
            c -= 1;
        }
        (c, r as u128)
    }
}

// The implementation for u8 is very naive, because it's rarely used
impl RootRem for u8 {
    type Output = u8;

    #[inline]
    fn sqrt_rem(self) -> (u8, u8) {
        // brute-force search, because there are only 16 possibilites.
        let mut s = 0;
        let e = fix_sqrt_error!(u8, self, s);
        (s, e)
    }

    #[inline]
    fn cbrt_rem(self) -> (u8, u8) {
        // brute-force search, because there are only 7 possibilites.
        let mut c = 0;
        let e = fix_cbrt_error!(u8, self, c);
        (c, e)
    }

    #[inline]
    fn nth_root_rem(self, _n: usize) -> (u8, u8) {
        unimplemented!()
    }
}

macro_rules! impl_rootrem_using_normalized {
    ($($t:ty)*) => {$(
        impl RootRem for $t {
            type Output = $t;

            #[inline]
            fn sqrt_rem(self) -> ($t, $t) {
                if self == 0 {
                    return (0, 0);
                }
        
                // normalize the input and call the normalized subroutine
                let shift = self.leading_zeros() & !1; // make sure shift is divisible by 2
                let (root, mut rem) = (self << shift).normalized_sqrt_rem();
                let root = (root >> (shift / 2)) as $t;
                if shift != 0 {
                    rem = self - root.pow(2);
                }
                (root, rem)
            }
        
            fn cbrt_rem(self) -> ($t, $t) {
                if self == 0 {
                    return (0, 0);
                }
        
                // normalize the input and call the normalized subroutine
                let mut shift = self.leading_zeros();
                shift -= shift % 3; // make sure shift is divisible by 3
                let (root, mut rem) = (self << shift).normalized_cbrt_rem();
                let root = (root >> (shift / 3)) as $t;
                if shift != 0 {
                    rem = self - root.pow(3);
                }
                (root, rem)
            }
        
            fn nth_root_rem(self, _n: usize) -> ($t, $t) {
                unimplemented!()
            }
        }        
    )*};
}
impl_rootrem_using_normalized!(u16 u32 u64 u128);

// XXX: maybe forward sqrt to f32/f64 if std enabled, don't forward cbrt

#[cfg(test)]
mod tests {
    use super::*;
    use rand::random;

    #[test]
    fn test_sqrt() {
        assert_eq!(u8::MAX.sqrt_rem(), (15, 30));
        assert_eq!(u16::MAX.sqrt_rem(), (u8::MAX as u16, (u8::MAX as u16) * 2));
        assert_eq!(u32::MAX.sqrt_rem(), (u16::MAX as u32, (u16::MAX as u32) * 2));
        assert_eq!(u64::MAX.sqrt_rem(), (u32::MAX as u64, (u32::MAX as u64) * 2));
        assert_eq!(u128::MAX.sqrt_rem(), (u64::MAX as u128, (u64::MAX as u128) * 2));

        assert_eq!((u8::MAX / 2).sqrt_rem(), (11, 6));
        assert_eq!((u16::MAX / 2).sqrt_rem(), (181, 6));
        assert_eq!((u32::MAX / 2).sqrt_rem(), (46340, 88047));
        assert_eq!((u64::MAX / 2).sqrt_rem(), (3037000499, 5928526806));
        assert_eq!((u128::MAX / 2).sqrt_rem(), (13043817825332782212, 9119501915260492783));

        macro_rules! random_case {
            ($T:ty) => {
                let n: $T = random();
                let (root, rem) = n.sqrt_rem();
                assert!(rem <= root * 2, "sqrt({}) remainder too large", n);
                assert_eq!(n, root * root + rem, "sqrt({}) != {}, {}", n, root, rem);
            };
        }

        const N: u32 = 10000;
        for _ in 0..N {
            random_case!(u8);
            random_case!(u16);
            random_case!(u32);
            random_case!(u64);
            random_case!(u128);
        }
    }

    #[test]
    fn test_cbrt() {
        assert_eq!(u8::MAX.cbrt_rem(), (6, 39));
        assert_eq!(u16::MAX.cbrt_rem(), (40, 1535));
        assert_eq!(u32::MAX.cbrt_rem(), (1625, 3951670));
        assert_eq!(u64::MAX.cbrt_rem(), (2642245, 19889396695490));
        assert_eq!(u128::MAX.cbrt_rem(), (6981463658331, 81751874631114922977532764));

        assert_eq!((u8::MAX / 2).cbrt_rem(), (5, 2));
        assert_eq!((u16::MAX / 2).cbrt_rem(), (31, 2976));
        assert_eq!((u32::MAX / 2).cbrt_rem(), (1290, 794647));
        assert_eq!((u64::MAX / 2).cbrt_rem(), (2097151, 13194133241856));
        assert_eq!((u128::MAX / 2).cbrt_rem(), (5541191377756, 58550521324026917344808511));
        assert_eq!((u8::MAX / 4).cbrt_rem(), (3, 36));
        assert_eq!((u16::MAX / 4).cbrt_rem(), (25, 758));
        assert_eq!((u32::MAX / 4).cbrt_rem(), (1023, 3142656));
        assert_eq!((u64::MAX / 4).cbrt_rem(), (1664510, 5364995536903));
        assert_eq!((u128::MAX / 4).cbrt_rem(), (4398046511103, 58028439341489006246363136));

        macro_rules! random_case {
            ($T:ty) => {
                let n: $T = random();
                let (root, rem) = n.cbrt_rem();
                let root2 = root * root;
                assert!(rem <= 3 * (root2 + root), "cbrt({}) remainder too large", n);
                assert_eq!(n, root2 * root + rem, "cbrt({}) != {}, {}", n, root, rem);
            };
        }

        const N: u32 = 10000;
        for _ in 0..N {
            random_case!(u16);
            random_case!(u32);
            random_case!(u64);
            random_case!(u128);
        }
    }
}
