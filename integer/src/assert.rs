#[inline]
pub(crate) const fn assert_in_const_fn(val: bool) {
    [(); 1][!val as usize]
}

// TODO: deprecate these, and bump the MSRV to 1.57?
macro_rules! debug_assert_in_const_fn {
    ($val:expr) => {
        #[cfg(debug_assertions)]
        crate::assert::assert_in_const_fn($val);
    };
}

pub(crate) use debug_assert_in_const_fn;
