//! Definitions of panic cases

pub const fn panic_divide_by_0() -> ! {
    panic!("Divisor or denominator must not be zero!")
}
