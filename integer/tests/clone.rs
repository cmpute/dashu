mod helper_macros;

#[test]
fn test_clone_inline_and_heap() {
    // value (and sign) preserved across clone, for inline and heap reprs
    let ubigs = [
        ubig!(0),
        ubig!(5),
        ubig!(0x10000000000000000),   // 2^64, two-word inline
        ubig!(1) << 200,              // heap
        (ubig!(1) << 256) - ubig!(1), // heap, all ones
    ];
    for v in &ubigs {
        assert_eq!(&v.clone(), v);
    }
    let ibigs = [
        ibig!(0),
        ibig!(5),
        ibig!(-5),
        ibig!(1) << 200,
        -(ibig!(1) << 200),
    ];
    for v in &ibigs {
        assert_eq!(&v.clone(), v);
    }
}

#[test]
fn test_clone_from_transitions() {
    // clone_from across every inline/heap combination of (dst, src) — including
    // dst heap-allocated with an inline src (dst's buffer must be freed) and
    // inline dst with a heap src (must allocate).
    let vals = [
        ubig!(0),
        ubig!(7),
        ubig!(0x10000000000000000), // inline cap 2
        ubig!(1) << 130,            // heap
        ubig!(1) << 200,            // heap, larger
    ];
    for src in &vals {
        for dst0 in &vals {
            let mut dst = dst0.clone();
            dst.clone_from(src);
            assert_eq!(&dst, src, "clone_from {dst0} <- {src}");
        }
    }
    // signed: heap negative -> inline negative -> heap positive
    let mut x = -(ibig!(1) << 200);
    x.clone_from(&ibig!(-3));
    assert_eq!(x, ibig!(-3));
    x.clone_from(&(ibig!(1) << 200));
    assert_eq!(x, ibig!(1) << 200);
}
