use super::EstimatedLog2;

// 8bit fixed point estimation of log2(x), x from 0x80 to 0xff, rounding down.
#[cfg(not(feature = "std"))]
const LOG2_TAB: [u8; 128] = [
    0x00, 0x02, 0x05, 0x08, 0x0b, 0x0e, 0x10, 0x13, 0x16, 0x19, 0x1b, 0x1e, 0x21, 0x23, 0x26, 0x28,
    0x2b, 0x2e, 0x30, 0x33, 0x35, 0x38, 0x3a, 0x3d, 0x3f, 0x41, 0x44, 0x46, 0x49, 0x4b, 0x4d, 0x50,
    0x52, 0x54, 0x57, 0x59, 0x5b, 0x5d, 0x60, 0x62, 0x64, 0x66, 0x68, 0x6a, 0x6d, 0x6f, 0x71, 0x73,
    0x75, 0x77, 0x79, 0x7b, 0x7d, 0x7f, 0x81, 0x84, 0x86, 0x88, 0x8a, 0x8c, 0x8d, 0x8f, 0x91, 0x93,
    0x95, 0x97, 0x99, 0x9b, 0x9d, 0x9f, 0xa1, 0xa2, 0xa4, 0xa6, 0xa8, 0xaa, 0xac, 0xad, 0xaf, 0xb1,
    0xb3, 0xb5, 0xb6, 0xb8, 0xba, 0xbc, 0xbd, 0xbf, 0xc1, 0xc2, 0xc4, 0xc6, 0xc8, 0xc9, 0xcb, 0xcd,
    0xce, 0xd0, 0xd1, 0xd3, 0xd5, 0xd6, 0xd8, 0xda, 0xdb, 0xdd, 0xde, 0xe0, 0xe1, 0xe3, 0xe5, 0xe6,
    0xe8, 0xe9, 0xeb, 0xec, 0xee, 0xef, 0xf1, 0xf2, 0xf4, 0xf5, 0xf7, 0xf8, 0xfa, 0xfb, 0xfd, 0xfe,
];

/// A 8bit fixed point estimation of log2(n), the result
/// is always less than the exact value and estimation error ≤ 2.
#[cfg(not(feature = "std"))]
const fn log2_fp8(n: u16) -> u16 {
    debug_assert!(n > 0xff); // if the input is small, it should be powered first

    let nbits = (u16::BITS - n.leading_zeros()) as u16;
    if n < 0x200 {
        // err = 0~2 in this range, use extra 1 bit to reduce error
        let lookup = LOG2_TAB[(n >> 1) as usize - 0x80];
        let est = lookup as u16 + (7 + 1) * 256;
        est + (n < 354 && n & 1 > 0) as u16
    } else if n < (0x4000 + 0x80) {
        // err = 0~3, use extra 2 bits to reduce error
        let shift = nbits - 8;
        let mask = n >> (shift - 2);
        let lookup = LOG2_TAB[(mask >> 2) as usize - 0x80];
        let est = lookup as u16 + (7 + shift) * 256;

        // err could be 0 if mask & 3 < 3
        est + (mask & 3 == 3) as u16
    } else {
        // err = 0~3, use extra 7 bits to reduce error
        let shift = nbits - 8;
        let mask = n >> (shift - 7);
        let top_est = LOG2_TAB[(mask >> 7) as usize - 0x80];
        let est = top_est as u16 + (7 + shift) * 256;

        // err could be 0 if mask & 127 < 80
        est + (mask & 127 >= 80) as u16
    }
}

/// A 8bit fixed point estimation of log2(n), the result
/// is always greater than the exact value and estimation error ≤ 2.
///
/// # Panics
///
/// Panics if n is a power of two, in which case the log should
/// be trivially handled.
#[cfg(not(feature = "std"))]
const fn ceil_log2_fp8(n: u16) -> u16 {
    debug_assert!(n > 0xff); // if the input is small, it should be powered first
    debug_assert!(!n.is_power_of_two());

    let nbits = (u16::BITS - n.leading_zeros()) as u16;
    if n < 0x80 {
        // err = 0 in this range
        let shift = 8 - nbits;
        let top_est = LOG2_TAB[(n << shift) as usize - 0x80];
        top_est as u16 + (7 - shift) * 256 + 1
    } else if n < 0x200 {
        // err = 0 in 0x80 ~ 0x100, err = 0~2 in 0x100 ~ 0x200
        let shift = nbits - 8;
        let top_est = LOG2_TAB[(n >> shift) as usize - 0x80];
        let est = top_est as u16 + (7 + shift) * 256 + 1;

        if n > 0x100 && n & 1 == 1 {
            est + 2
        } else {
            est
        }
    } else {
        // err = 0~3, use extra 2 bits to reduce error
        let shift = nbits - 8;
        let mask10 = n >> (shift - 2);
        let mask8 = mask10 >> 2;
        if mask8 == 255 {
            0x100 + (7 + shift) * 256
        } else {
            // find next item in LOG2_TAB
            let top_est = LOG2_TAB[mask8 as usize + 1 - 0x80];
            let est = top_est as u16 + (7 + shift) * 256 + 1;
            est - (mask10 & 3 == 0) as u16
        }
    }
}

/// Implementation of the nightly f32::next_up()
#[cfg(feature = "std")]
#[inline]
fn next_up(f: f32) -> f32 {
    debug_assert!(!f.is_nan() && !f.is_infinite());
    use std::cmp::Ordering::*;

    match f.partial_cmp(&0.).unwrap() {
        Equal => f32::from_bits(1),
        Less => f32::from_bits(f.to_bits() - 1),
        Greater => f32::from_bits(f.to_bits() + 1),
    }
}

/// Implementation of the nightly f32::next_down()
#[cfg(feature = "std")]
#[inline]
fn next_down(f: f32) -> f32 {
    debug_assert!(!f.is_nan() && !f.is_infinite());
    use std::cmp::Ordering::*;

    match f.partial_cmp(&0.).unwrap() {
        Equal => f32::from_bits(1 | (1 << 31)),
        Less => f32::from_bits(f.to_bits() + 1),
        Greater => f32::from_bits(f.to_bits() - 1),
    }
}

#[cfg(not(feature = "std"))]
impl EstimatedLog2 for u8 {
    #[inline]
    fn log2_bounds(&self) -> (f32, f32) {
        match *self {
            0 => (f32::NEG_INFINITY, f32::NEG_INFINITY),
            1 => (0., 0.),
            i if i.is_power_of_two() => {
                let log = self.trailing_zeros() as f32;
                (log, log)
            }
            3 => (1.5849625, 1.5849626),
            i if i < 16 => {
                let pow = (i as u16).pow(4);
                let lb = log2_fp8(pow) as f32 / 256.0;
                let ub = ceil_log2_fp8(pow) as f32 / 256.0;
                (lb / 4., ub / 4.)
            }
            i => {
                let pow = (i as u16).pow(2);
                let lb = log2_fp8(pow) as f32 / 256.0;
                let ub = ceil_log2_fp8(pow) as f32 / 256.0;
                (lb / 2., ub / 2.)
            }
        }
    }
}

#[cfg(not(feature = "std"))]
impl EstimatedLog2 for u16 {
    #[inline]
    fn log2_bounds(&self) -> (f32, f32) {
        if *self <= 0xff {
            return (*self as u8).log2_bounds();
        } else if self.is_power_of_two() {
            let log = self.trailing_zeros() as f32;
            return (log, log);
        }

        let lb = log2_fp8(*self) as f32 / 256.0;
        let ub = ceil_log2_fp8(*self) as f32 / 256.0;
        (lb, ub)
    }
}

#[cfg(not(feature = "std"))]
macro_rules! impl_log2_bounds_for_uint {
    ($($t:ty)*) => {$(
        impl EstimatedLog2 for $t {
            #[inline]
            fn log2_bounds(&self) -> (f32, f32) {
                if *self <= 0xff {
                    return (*self as u8).log2_bounds();
                } else if self.is_power_of_two() {
                    let log = self.trailing_zeros() as f32;
                    return (log, log);
                }

                let bits = <$t>::BITS - self.leading_zeros();
                if bits <= u16::BITS {
                    let lb = log2_fp8(*self as u16) as f32 / 256.0;
                    let ub = ceil_log2_fp8(*self as u16) as f32 / 256.0;
                    (lb, ub)
                } else {
                    let shift = bits - u16::BITS;
                    let hi = (*self >> shift) as u16;
                    let lb = log2_fp8(hi) as f32 / 256.0;
                    let ub = if hi == 1 << (u16::BITS - 1) {
                        // specially handled because ceil_log2_fp8 disallow a power of 2
                        (u16::BITS as u16 - 1) * 256 + 1
                    } else {
                        // in this case, the ceiling handled by the highest word
                        // will cover the requirement for ceiling the low bits
                        ceil_log2_fp8(hi)
                    };
                    let ub = ub as f32 / 256.0;
                    (lb + shift as f32, ub + shift as f32)
                }
            }
        }
    )*};
}

#[cfg(not(feature = "std"))]
impl_log2_bounds_for_uint!(u32 u64 u128 usize);

#[cfg(feature = "std")]
macro_rules! impl_log2_bounds_for_uint {
    ($($t:ty)*) => {$(
        impl EstimatedLog2 for $t {
            fn log2_bounds(&self) -> (f32, f32) {
                if *self == 0 {
                    return (f32::NEG_INFINITY, f32::NEG_INFINITY);
                }

                if self.is_power_of_two() {
                    let log = self.trailing_zeros() as f32;
                    (log, log)
                } else {
                    let nbits = Self::BITS - self.leading_zeros();
                    if nbits <= 24 {
                        // 24bit integer converted to f32 is lossless
                        let log = (*self as f32).log2();
                        (next_down(log), next_up(log))
                    } else {
                        let shifted = (self >> (nbits - 24)) as f32;
                        let est_lb = shifted.log2();
                        let est_ub = (shifted + 1.).log2();

                        let shift = (nbits - 24) as f32;
                        (next_down(est_lb + shift), next_up(est_ub + shift))
                    }
                }
            }

            #[inline]
            fn log2_est(&self) -> f32 {
                (*self as f32).log2()
            }
        }
    )*}
}

#[cfg(feature = "std")]
impl_log2_bounds_for_uint!(u8 u16 u32 u64 u128 usize);

macro_rules! impl_log2_bounds_for_int {
    ($($t:ty)*) => {$(
        impl EstimatedLog2 for $t {
            fn log2_bounds(&self) -> (f32, f32) {
                self.unsigned_abs().log2_bounds()
            }
        }
    )*};
}
impl_log2_bounds_for_int!(i8 i16 i32 i64 i128 isize);

#[cfg(not(feature = "std"))]
macro_rules! impl_log2_bounds_for_float {
    ($($t:ty)*) => {$(
        impl EstimatedLog2 for $t {
            fn log2_bounds(&self) -> (f32, f32) {
                use crate::FloatEncoding;
                use core::num::FpCategory::*;
        
                if *self == 0. {
                    (f32::NEG_INFINITY, f32::NEG_INFINITY)
                } else {
                    match self.decode() {
                        Ok((man, exp)) => {
                            let (est_lb, est_ub) = man.log2_bounds();
                            (est_lb + exp as f32, est_ub + exp as f32)
                        },
                        Err(Nan) => panic!("calling log2 on nans is forbidden!"),
                        Err(Infinite) => (f32::INFINITY, f32::INFINITY),
                        _ => unreachable!()
                    }
                }
            }
        }
    )*};
}
#[cfg(not(feature = "std"))]
impl_log2_bounds_for_float!(f32 f64);

#[cfg(feature = "std")]
macro_rules! impl_log2_bounds_for_float {
    ($($t:ty)*) => {$(
        impl EstimatedLog2 for $t {
            #[inline]
            fn log2_bounds(&self) -> (f32, f32) {
                assert!(!self.is_nan());

                if *self == 0. {
                    (f32::NEG_INFINITY, f32::NEG_INFINITY)
                } else if self.is_infinite() {
                    (f32::INFINITY, f32::INFINITY)
                } else {
                    let log2 = self.abs().log2() as f32;
                    (next_down(log2), next_up(log2))
                }
            }
        
            #[inline]
            fn log2_est(&self) -> f32 {
                assert!(!self.is_nan());

                if *self == 0. {
                    f32::NEG_INFINITY
                } else if self.is_infinite() {
                    f32::INFINITY
                } else {
                    self.abs().log2() as f32
                }
            }
        }
    )*};
}

#[cfg(feature = "std")]
impl_log2_bounds_for_float!(f32 f64);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(not(feature = "std"))]
    fn test_log2_fp8() {
        assert_eq!(log2_fp8(1234), 2628); // err = 0
        assert_eq!(log2_fp8(12345), 3478); // err = 1
        assert_eq!(log2_fp8(0x100), 2048); // err = 0
        assert_eq!(log2_fp8(0x101), 2049); // err = 0
        assert_eq!(log2_fp8(0xff00), 4094); // err = 0
        assert_eq!(log2_fp8(0xffff), 4095); // err = 0

        assert_eq!(ceil_log2_fp8(1234), 2631); // err = 2
        assert_eq!(ceil_log2_fp8(12345), 3480); // err = 0
        assert_eq!(ceil_log2_fp8(0x101), 2051); // err = 1
        assert_eq!(ceil_log2_fp8(0xff00), 4096); // err = 1
        assert_eq!(ceil_log2_fp8(0xffff), 4096); // err = 0
    }

    #[test]
    fn test_log2_bounds() {
        assert_eq!(0u8.log2_bounds(), (f32::NEG_INFINITY, f32::NEG_INFINITY));
        assert_eq!(0i8.log2_bounds(), (f32::NEG_INFINITY, f32::NEG_INFINITY));
        assert_eq!(0f32.log2_bounds(), (f32::NEG_INFINITY, f32::NEG_INFINITY));

        // small tests
        for i in 1..1000u16 {
            let (lb, ub) = i.log2_bounds();
            assert!(2f64.powf(lb as f64) <= i as f64);
            assert!(2f64.powf(ub as f64) >= i as f64);
            assert_eq!((-(i as i16)).log2_bounds(), (lb, ub));

            let (lb, ub) = (i as f32).log2_bounds();
            assert!(2f64.powf(lb as f64) <= i as f64);
            assert!(2f64.powf(ub as f64) >= i as f64);

            let (lb, ub) = (i as f64).log2_bounds();
            assert!(2f64.powf(lb as f64) <= i as f64);
            assert!(2f64.powf(ub as f64) >= i as f64);
        }

        // large tests
        for i in (0x4000..0x400000u32).step_by(0x1001) {
            let (lb, ub) = i.log2_bounds();
            assert!(2f64.powf(lb as f64) <= i as f64);
            assert!(2f64.powf(ub as f64) >= i as f64);
        }

        let (lb, ub) = 1e20f32.log2_bounds();
        assert!(2f64.powf(lb as f64) <= 1e20);
        assert!(2f64.powf(ub as f64) >= 1e20);
        assert_eq!((-1e20f32).log2_bounds(), (lb, ub));

        let (lb, ub) = 1e40f64.log2_bounds();
        assert!(2f64.powf(lb as f64) <= 1e40);
        assert!(2f64.powf(ub as f64) >= 1e40);
        assert_eq!((-1e40f64).log2_bounds(), (lb, ub));
    }
}
