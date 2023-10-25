use dashu_int::ops::*;

mod helper_macros;

#[test]
fn test_sqrt() {
    // some fixed cases
    let a = ubig!(10000);
    assert_eq!(a.sqrt(), ubig!(100));
    assert_eq!(a.sqrt_rem(), (ubig!(100), ubig!(0)));
    let a = ubig!(100000);
    assert_eq!(a.sqrt(), ubig!(316));
    assert_eq!(a.sqrt_rem(), (ubig!(316), ubig!(144)));
    let a = ubig!(1234567890123456789);
    assert_eq!(a.sqrt(), ubig!(1111111106));
    assert_eq!(a.sqrt_rem(), (ubig!(1111111106), ubig!(246913553)));
    let a = ubig!(12345679012345678987654320987654321);
    assert_eq!(a.sqrt(), ubig!(111111111111111111));
    assert_eq!(a.sqrt_rem(), (ubig!(111111111111111111), ubig!(0)));
    let a = ubig!(100788288067706660892852085821456193179743392153874910688885216801600345870807);
    assert_eq!(a.sqrt(), ubig!(317471712232297416216550966658362741242));
    assert_eq!(
        a.sqrt_rem(),
        (
            ubig!(317471712232297416216550966658362741242),
            ubig!(547939222817117722717438201919698168243)
        )
    );

    assert_eq!(ibig!(10000).sqrt(), ibig!(100));
    assert_eq!(ibig!(100000).sqrt(), ibig!(316));

    // sqrt on 2^i - 1
    for i in [10, 20, 50, 64, 100, 128, 200, 256, 512, 400, 512] {
        let a = (ubig!(1) << i) - ubig!(1);
        let s = (ubig!(1) << (i / 2)) - ubig!(1);
        assert_eq!(a.sqrt(), s);
        assert_eq!(a.sqrt_rem(), (s.clone(), s * 2u8), "failed when i = {}", i);
    }

    // sqrt on 3^(3+12*i)
    let sqrt3_cases = [
        (ubig!(5), ubig!(2)),             // sqrt(3^3)
        (ubig!(3787), ubig!(7538)),       // sqrt(3^15)
        (ubig!(2761448), ubig!(2428283)), // sqrt(3^27)
        (ubig!(2013095912), ubig!(2107864523)),
        (ubig!(1467546920229), ubig!(1934876898306)),
        (ubig!(1069841704847421), ubig!(1224877059345186)),
        (ubig!(779914602833770326), ubig!(501110532100715031)),
        (ubig!(568557745465818567888), ubig!(225657412133007072843)),
        (ubig!(414478596444581735990496), ubig!(553764985337871833514651)),
        (ubig!(302154896808100085537072070), ubig!(598857875470661932825826247)),
        (ubig!(220270919773104962356525539752), ubig!(186420045640482600067047651323)),
        (
            ubig!(160577500514593517557907118479516),
            ubig!(155515158234106646561085883459451),
        ),
        (
            ubig!(117060997875138674299714289371567517),
            ubig!(2066707243966299471754695247556498),
        ),
        (
            ubig!(85337467450976093564491716951872719899),
            ubig!(74283355028981034794886393634200214866),
        ),
        (
            ubig!(62211013771761572208514461657915212806688),
            ubig!(35437748659873332231051228234751469262203),
        ),
        (
            ubig!(45351829039614186140007042548620190136075759),
            ubig!(57415363151470492239180169175796859839103146),
        ),
        (
            ubig!(33061483369878741696065134017944118609199228772),
            ubig!(30190341552430022310094722410181632071126298123),
        ),
    ];
    for (i, (s, r)) in sqrt3_cases.into_iter().enumerate() {
        let e = 3 + 12 * i;
        let pow = ubig!(3).pow(e);
        assert_eq!(pow.sqrt(), s);
        assert_eq!(pow.sqrt_rem(), (s, r));
    }
}

#[test]
#[should_panic]
fn test_sqrt_negative_panic() {
    let _ = ibig!(-1).sqrt();
}

#[test]
fn test_nth_root() {
    assert_eq!(ubig!(2).nth_root(1), ubig!(2));
    assert_eq!(ubig!(2).nth_root(2), ubig!(1));
    assert_eq!(ubig!(2).nth_root(3), ubig!(1));
    assert_eq!(ibig!(-2).nth_root(1), ibig!(-2));
    assert_eq!(ibig!(2).nth_root(2), ibig!(1));
    assert_eq!(ibig!(-2).nth_root(3), ibig!(-1));
    assert_eq!(ibig!(-32).nth_root(5), ibig!(-2));

    let test_cases = [
        (
            ubig!(123456789),
            [
                ubig!(11111),
                ubig!(497),
                ubig!(105),
                ubig!(41),
                ubig!(22),
                ubig!(14),
            ],
        ),
        (
            (ubig!(1) << 512) + ubig!(1),
            [
                ubig!(115792089237316195423570985008687907853269984665640564039457584007913129639936),
                ubig!(2375668978229576954621987151322942598255746237355560),
                ubig!(340282366920938463463374607431768211456),
                ubig!(6690699980388625489511488543534),
                ubig!(48740834812604276470692694),
                ubig!(10427830626922116451064),
            ],
        ),
    ];

    for (n, roots) in test_cases {
        for (i, root) in roots.into_iter().enumerate() {
            assert_eq!(n.nth_root(i + 2), root);
        }
    }

    // large order roots
    assert_eq!(ubig!(123456789).nth_root(26), ubig!(2));
    assert_eq!(ubig!(123456789).nth_root(27), ubig!(1));
    let n128 = (ubig!(1) << 128) + ubig!(1);
    assert_eq!(n128.nth_root(128), ubig!(2));
    assert_eq!(n128.nth_root(129), ubig!(1));
}

#[test]
#[should_panic]
fn test_zeroth_root_panic() {
    let _ = ubig!(2).nth_root(0);
}

#[test]
#[should_panic]
fn test_even_root_negative_panic() {
    let _ = ibig!(-1).nth_root(4);
}
