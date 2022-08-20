use crate::repr::{Repr, Word};

#[inline]
pub(crate) fn check_inf_operands<const B: Word>(lhs: &Repr<B>, rhs: &Repr<B>) {
    if lhs.is_infinite() || rhs.is_infinite() {
        panic_operate_with_inf()
    }
}

/// Panics when operate with infinities
pub(crate) const fn panic_operate_with_inf() -> ! {
    panic!("binary operations with infinity is forbidden!")
}
