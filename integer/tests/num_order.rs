use num_order::NumOrd;

mod helper_macros;

#[test]
fn test_ord_with_float() {
    assert!(ibig!(0).num_eq(&0f32));
    assert!(ibig!(0).num_eq(&-0f32));
    assert!(ibig!(1).num_eq(&1f32));
    assert!(ibig!(-1).num_eq(&-1f32));

    assert!(ibig!(1).num_gt(&-1f32));
    assert!(ibig!(-1).num_gt(&-1.0001f32));
    assert!(ibig!(-100000).num_gt(&-100001f32));
}
