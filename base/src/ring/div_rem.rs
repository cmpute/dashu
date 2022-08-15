use super::{DivEuclid, DivRem, DivRemAssign, DivRemEuclid, RemEuclid};

macro_rules! impl_div_rem_ops_prim {
    ($($T:ty)*) => {$(
        impl DivRem for $T {
            type OutputDiv = $T;
            type OutputRem = $T;
            #[inline]
            fn div_rem(self, rhs: $T) -> ($T, $T) {
                (self / rhs, self % rhs)
            }
        }
        impl DivRemAssign for $T {
            type OutputRem = $T;
            #[inline]
            fn div_rem_assign(&mut self, rhs: $T) -> $T {
                let r = *self % rhs;
                *self /= rhs;
                r
            }
        }
        impl DivEuclid for $T {
            type Output = $T;
            #[inline]
            fn div_euclid(self, rhs: $T) -> $T {
                <$T>::div_euclid(self, rhs)
            }
        }
        impl RemEuclid for $T {
            type Output = $T;
            #[inline]
            fn rem_euclid(self, rhs: $T) -> $T {
                <$T>::rem_euclid(self, rhs)
            }
        }
        impl DivRemEuclid for $T {
            type OutputDiv = $T;
            type OutputRem = $T;
            #[inline]
            fn div_rem_euclid(self, rhs: $T) -> ($T, $T) {
                let (q, r) = (self / rhs, self % rhs);

                // depending on compiler to simplify the case for unsinged integers
                #[allow(unused_comparisons)]
                if r >= 0 {
                    (q, r)
                } else if rhs >= 0{
                    (q - 1, r + rhs)
                } else {
                    (q + 1, r - rhs)
                }
            }
        }
    )*}
}
impl_div_rem_ops_prim!(u8 u16 u32 u64 u128 usize i8 i16 i32 i64 i128);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple() {
        assert_eq!(7u32.div_rem(4), (1, 3));
        assert_eq!(7u32.div_rem_euclid(4), (1, 3));
        assert_eq!(7i32.div_rem(-4), (-1, 3));
        assert_eq!(7i32.div_rem_euclid(-4), (-1, 3));
        assert_eq!((-7i32).div_rem(4), (-1, -3));
        assert_eq!((-7i32).div_rem_euclid(4), (-2, 1));
        assert_eq!((-7i32).div_rem(-4), (1, -3));
        assert_eq!((-7i32).div_rem_euclid(-4), (2, 1));

        let mut n = 7u32;
        let r = n.div_rem_assign(4);
        assert!(n == 1 && r == 3);
    }
}
