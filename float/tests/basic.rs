use std::str::FromStr;

use dashu_int::ibig;
use dashu_float::{FBig, DBig};

#[test]
fn test_print() {
    let f = FBig::from(-1.2f32);
    let g = FBig::from(2.7f32);
    dbg!(f.clone()+g.clone());
    println!("f+g: {}", f+g);
    // dbg!(&f);
    // println!("{}", f);
    // for i in 0..10 {
    //     println!(".{}, {:.*}", i, i, f);
    // }
    let f = FBig::from_ratio(ibig!(3), ibig!(16), 10);
    dbg!(&f);
    println!("{}", f);
    // for i in 0..10 {
    //     println!(".{}, {:.*}", i, i, f);
    // }

    // let rf = f.clone().with_precision(100).recip().into_decimal();
    // dbg!(&rf);
    // println!("{}", rf);
    // for i in 0..10 {
    //     println!(".{}, {:.*}", i, i, rf);
    // }

    let d = f.with_precision(100).into_decimal();
    dbg!(&d);
    println!("{}", d);
    for i in 0..10 {
        println!(".{}, {:.*}", i, i, d);
    }

    let f = DBig::from_str("121241431345234523452.234523452345234523534e-12").unwrap();
    println!("{}", f);
}
