mod helper_macros;

#[test]
fn test_ceil_floor() {
    let test_cases = [
        // (ratio, ceil, floor)
        (rbig!(~0), ibig!(0), ibig!(0)),
        (rbig!(~1), ibig!(1), ibig!(1)),
        (rbig!(~-1), ibig!(-1), ibig!(-1)),
        (rbig!(~10), ibig!(10), ibig!(10)),
        (rbig!(~-10), ibig!(-10), ibig!(-10)),
        (rbig!(~4/2), ibig!(2), ibig!(2)),
        (rbig!(~4/3), ibig!(2), ibig!(1)),
        (rbig!(~-4/3), ibig!(-1), ibig!(-2)),
        (rbig!(~0xffff/0xfe), ibig!(259), ibig!(258)),
        (rbig!(~-0xffff/0xfe), ibig!(-258), ibig!(-259)),
        (rbig!(~0xfffe/0xff), ibig!(257), ibig!(256)),
        (rbig!(~-0xfffe/0xff), ibig!(-256), ibig!(-257)),
    ];

    for (ratio, ceil, floor) in test_cases {
        assert_eq!(ratio.ceil(), ceil);
        assert_eq!(ratio.floor(), floor);

        let ratio = ratio.canonicalize();
        assert_eq!(ratio.ceil(), ceil);
        assert_eq!(ratio.floor(), floor);
    }
}

#[test]
fn test_trunc_fract() {
    let test_cases = [
        // (ratio, trunc, fract)
        (rbig!(~0), ibig!(0), rbig!(~0)),
        (rbig!(~1), ibig!(1), rbig!(~0)),
        (rbig!(~10), ibig!(10), rbig!(~0)),
        (rbig!(~4/2), ibig!(2), rbig!(~0)),
        (rbig!(~4/3), ibig!(1), rbig!(~1/3)),
        (rbig!(~0xffff/0xfe), ibig!(258), rbig!(~3/0xfe)),
        (rbig!(~0xfffe/0xff), ibig!(256), rbig!(~0xfe/0xff)),
    ];
    
    for (ratio, trunc, fract) in test_cases {
        assert_eq!(ratio.trunc(), trunc);
        assert_eq!(ratio.fract(), fract);
        assert_eq!((-&ratio).trunc(), -&trunc);
        assert_eq!((-&ratio).fract(), -&fract);
        assert_eq!(ratio.clone().split_at_point(), (trunc.clone(), fract.clone()));
        assert_eq!((-ratio.clone()).split_at_point(), (-trunc.clone(), -fract.clone()));

        let ratio = ratio.canonicalize();
        let fract = fract.canonicalize();
        assert_eq!(ratio.trunc(), trunc);
        assert_eq!(ratio.fract(), fract);
        assert_eq!((-&ratio).trunc(), -&trunc);
        assert_eq!((-&ratio).fract(), -&fract);
        assert_eq!(ratio.clone().split_at_point(), (trunc.clone(), fract.clone()));
        assert_eq!((-ratio.clone()).split_at_point(), (-trunc.clone(), -fract.clone()));
    }
}
