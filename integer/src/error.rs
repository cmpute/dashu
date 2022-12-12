//! Definitions of panic cases

/// Panics when division by 0 is happening
pub(crate) const fn panic_divide_by_0() -> ! {
    panic!("divisor must not be 0")
}

/// Panics when try to allocate memory with size exceeding usize range
pub(crate) const fn panic_allocate_too_much() -> ! {
    panic!("try to allocate too much memory")
}

/// Panics when allocation failed
pub(crate) const fn panic_out_of_memory() -> ! {
    panic!("out of memory")
}

/// Panics when the `UBig` result is negative
pub(crate) const fn panic_negative_ubig() -> ! {
    panic!("UBig result must not be negative")
}

/// Panics when trying to do operations on `Modulo` values from different rings.
pub(crate) const fn panic_different_rings() -> ! {
    panic!("Modulo values from different rings")
}

/// Panics when the radix is not supported
pub(crate) fn panic_invalid_radix(radix: u32) -> ! {
    panic!("invalid radix: {}, only radix 2-36 are supported", radix);
}

/// Panics when the base is 0 or 1 in logarithm
pub(crate) fn panic_invalid_log_oprand() -> ! {
    panic!("logarithm is not defined for 0, base 0 and base 1!");
}

/// Panics when taking the zeroth root of an integer
pub(crate) fn panic_root_zeroth() -> ! {
    panic!("finding 0th root is not allowed!")
}

/// Panics when taking an even order root of an negative integer
pub(crate) fn panic_root_negative() -> ! {
    panic!("the root is a complex number!")
}

/// Panics when taking an inavlid inverse on a modulo number
pub(crate) fn panic_divide_by_invalid_modulo() -> ! {
    panic!("Division by a non-invertible Modulo")
}
