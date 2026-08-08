use super::{DivExact, DivExactAssign};

macro_rules! impl_div_exact_prim {
    ($($T:ty)*) => {$(
        impl DivExact for $T {
            type Output = $T;
            #[inline]
            fn div_exact(self, rhs: $T) -> Option<$T> {
                (rhs != 0 && self % rhs == 0).then(|| self / rhs)
            }
        }
        impl DivExactAssign for $T {
            #[inline]
            fn div_exact_assign(&mut self, rhs: $T) -> bool {
                if rhs != 0 && *self % rhs == 0 {
                    *self /= rhs;
                    true
                } else {
                    false
                }
            }
        }
    )*}
}
impl_div_exact_prim!(u8 u16 u32 u64 u128 usize i8 i16 i32 i64 i128 isize);

#[cfg(test)]
mod tests {
    // std adds `u32::div_exact` behind the `exact_div` feature (issue #139911), with the same
    // `Option` semantics for non-zero divisors; until it stabilizes, method-call syntax on
    // primitives resolves to this trait method and emits a `future_incompatible`
    // `unstable_name_collisions` warning. Once stabilized the inherent method silently shadows the
    // trait impl — the only difference is `rhs == 0` (std panics, this returns `None`).
    #![allow(unstable_name_collisions)]
    use super::*;

    #[test]
    fn test_div_exact() {
        assert_eq!(10u32.div_exact(5), Some(2));
        assert_eq!(10u32.div_exact(3), None);
        assert_eq!(0u32.div_exact(7), Some(0)); // 0 is divisible by any non-zero value
        assert_eq!(10u32.div_exact(0), None); // division by zero is not exact
        assert_eq!(10i32.div_exact(-5), Some(-2));
        assert_eq!((-10i32).div_exact(5), Some(-2));
    }

    #[test]
    fn test_div_exact_assign() {
        let mut n = 10u32;
        assert!(n.div_exact_assign(5));
        assert_eq!(n, 2);
        assert!(!n.div_exact_assign(3)); // 3 doesn't divide 2
        assert_eq!(n, 2); // unchanged
        assert!(!n.div_exact_assign(0)); // division by zero leaves it unchanged
        assert_eq!(n, 2);
        assert!(n.div_exact_assign(2));
        assert_eq!(n, 1);
    }
}
