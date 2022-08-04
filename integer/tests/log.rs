mod helper_macros;

#[test]
fn test_log_ubig() {
    let test_cases = [
        // trivial cases
        (ubig!(1), ubig!(2), 0),
        (ubig!(1), ubig!(10), 0),
        (ubig!(1), ubig!(1000), 0),
        (ubig!(2), ubig!(2), 1),
        (ubig!(3), ubig!(2), 1),
        (ubig!(1) << 100, ubig!(2), 100),
        (ubig!(1) << 1000, ubig!(2), 1000),
        (ubig!(123456), ubig!(123456), 1),
        ((ubig!(1) << 100) - 1u8, (ubig!(1) << 100) - 1u8, 1),
        (ubig!(1) << 1000, ubig!(1) << 1000, 1),
        (ubig!(123456), ubig!(123457), 0),
        (ubig!(1) << 100, ubig!(1) << 101, 0),
        (ubig!(1) << 1000, ubig!(1) << 1001, 0),
        // small bases
        (ubig!(4), ubig!(3), 1),
        (ubig!(10), ubig!(3), 2),
        (ubig!(1) << 20, ubig!(3), 12),
        (ubig!(1) << 50, ubig!(3), 31),
        (ubig!(1) << 100, ubig!(3), 63),
        (ubig!(1) << 500, ubig!(3), 315),
        (ubig!(1) << 5000, ubig!(3), 3154),
        ((ubig!(1) << 20) - 1u8, ubig!(31), 4),
        ((ubig!(1) << 50) - 1u8, ubig!(31), 10),
        ((ubig!(1) << 100) - 1u8, ubig!(31), 20),
        ((ubig!(1) << 500) - 1u8, ubig!(31), 100),
        ((ubig!(1) << 5000) - 1u8, ubig!(31), 1009),
        ((ubig!(1) << 5000) - 1u8, ubig!(31), 1009),
        (ubig!(7).pow(11) - 1u8, ubig!(7), 10),
        (ubig!(7).pow(20), ubig!(3).pow(20) + 2u8, 1),
        (ubig!(7).pow(200), ubig!(3).pow(20) + 2u8, 17),
        (ubig!(7).pow(2000), ubig!(3).pow(20) + 2u8, 177),
        (ubig!(7).pow(40), ubig!(3).pow(40) + 2u8, 1),
        (ubig!(7).pow(400), ubig!(3).pow(40) + 2u8, 17),
        (ubig!(7).pow(4000), ubig!(3).pow(40) + 2u8, 177),
        // large bases
        (ubig!(2).pow(4000), ubig!(2).pow(400), 10),
        (ubig!(3).pow(4000), ubig!(2).pow(400), 15),
        (ubig!(5).pow(4000), ubig!(2).pow(400), 23),
        (ubig!(7).pow(4000), ubig!(2).pow(400), 28),
        (ubig!(3).pow(4000), ubig!(3).pow(400), 10),
        (ubig!(5).pow(4000), ubig!(3).pow(400), 14),
        (ubig!(7).pow(4000), ubig!(3).pow(400), 17),
        (ubig!(5).pow(4000), ubig!(5).pow(400), 10),
        (ubig!(7).pow(4000), ubig!(5).pow(400), 12),
        (ubig!(7).pow(4000), ubig!(7).pow(400), 10),
        // large bases with near perfect power
        (ubig!(2).pow(4000) - 1u8, ubig!(2).pow(400), 9),
        (ubig!(3).pow(4000) - 1u8, ubig!(3).pow(400), 9),
        (ubig!(5).pow(4000) - 1u8, ubig!(5).pow(400), 9),
        (ubig!(7).pow(4000) - 1u8, ubig!(7).pow(400), 9),
        (ubig!(2).pow(4000) + 1u8, ubig!(2).pow(400), 10),
        (ubig!(3).pow(4000) + 1u8, ubig!(3).pow(400), 10),
        (ubig!(5).pow(4000) + 1u8, ubig!(5).pow(400), 10),
        (ubig!(7).pow(4000) + 1u8, ubig!(7).pow(400), 10),
    ];
    for (pow, base, exp) in test_cases {
        assert_eq!(pow.log(&base), exp, "{}, {}, {}", pow, base, exp);
    }
}

#[test]
#[should_panic]
fn test_log_base_0() {
    let _ = ubig!(1234).log(&ubig!(0));
}

#[test]
#[should_panic]
fn test_log_base_1() {
    let _ = ubig!(1234).log(&ubig!(1));
}

#[test]
#[should_panic]
fn test_log_0() {
    let _ = ubig!(0).log(&ubig!(1234));
}
