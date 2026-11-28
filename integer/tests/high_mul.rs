use dashu_int::{high, UBig, Word};

/// Operand of `len` words with the word value 1 at each listed index.
fn sparse(len: usize, idx: &[usize]) -> UBig {
    let mut w = vec![0 as Word; len];
    for &i in idx {
        w[i] = 1;
    }
    UBig::from_words(&w)
}

/// Check the `mul_high` contract (one-sided value, error bound, exact sticky flag) against the
/// full product.
fn check(a: &UBig, b: &UBig, n: usize) {
    let (wa, wb) = (a.as_words().len(), b.as_words().len());
    let (v, sticky) = high::mul_high(a, b, n);
    let full = a * b;
    let s = (wa + wb - n) * Word::BITS as usize;
    let top = &full >> s;
    assert!(v <= top, "one-sidedness, n = {n}, a = {a:?}, b = {b:?}");
    assert!(&top - &v <= UBig::from(n + 2), "error bound, n = {n}, a = {a:?}, b = {b:?}");
    assert_eq!(sticky, full != (&top << s), "sticky, n = {n}, a = {a:?}, b = {b:?}");
}

#[test]
fn test_mul_high_sticky_sparse_reported() {
    // A low word of one operand times a middle word of the other lands below the window,
    // in a part of the product that the recursive split does not compute.
    check(&sparse(97, &[1, 96]), &sparse(97, &[24, 96]), 97);
    check(&sparse(97, &[1, 96]), &sparse(97, &[23, 96]), 97);
    check(&sparse(97, &[1, 96]), &sparse(97, &[0, 24, 96]), 97);
    check(&sparse(96, &[1, 95]), &sparse(96, &[24, 95]), 96);
    check(&sparse(100, &[50, 99]), &sparse(100, &[1, 99]), 100);
    for n in [126, 127, 200, 380] {
        check(&sparse(n, &[1, n - 1]), &sparse(n, &[95, n - 1]), n);
    }
}

#[test]
fn test_mul_high_sticky_sparse_sweep() {
    // One low/middle word in each operand, placed around the boundaries of the recursive
    // split (l = n / 4, k = n - l), plus the top word.
    for n in [97usize, 100, 131, 200, 385] {
        let (l, k) = (n / 4, n - n / 4);
        let positions = [0, 1, l - 1, l, l + 1, n / 2, k - 2, k - 1, k, n - 2];
        for &i in &positions {
            for &j in &positions {
                let (a, b) = (sparse(n, &[i, n - 1]), sparse(n, &[j, n - 1]));
                check(&a, &b, n);
                // windows narrower than the operands truncate them first
                check(&a, &b, n - 1);
            }
        }
    }
}
