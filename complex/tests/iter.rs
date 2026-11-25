//! `Sum` / `Product` identities: the cached wrapper must agree with `CBig`.

use dashu_cmplx::{CBig, CachedCBig};

#[test]
fn empty_sum_and_product_are_the_identities() {
    let sum: CBig = core::iter::empty::<CBig>().sum();
    let product: CBig = core::iter::empty::<CBig>().product();
    assert_eq!(sum, <CBig>::ZERO);
    assert_eq!(product, <CBig>::ONE);

    let cached_sum: CachedCBig = core::iter::empty::<CachedCBig>().sum();
    let cached_sum_ref: CachedCBig = core::iter::empty::<&CachedCBig>().sum();
    assert_eq!(cached_sum.as_cbig(), &sum);
    assert_eq!(cached_sum_ref.as_cbig(), &sum);

    let cached_product: CachedCBig = core::iter::empty::<CachedCBig>().product();
    let cached_product_ref: CachedCBig = core::iter::empty::<&CachedCBig>().product();
    assert_eq!(cached_product.as_cbig(), &product);
    assert_eq!(cached_product_ref.as_cbig(), &product);
    assert_eq!(cached_product.as_cbig().precision(), product.precision());
}

#[test]
fn cached_product_matches_cbig() {
    let parts = [(3, 4), (1, 2), (-2, 5)];
    let plain: Vec<CBig> = parts
        .iter()
        .map(|&(re, im)| CBig::from_parts(re.into(), im.into()))
        .collect();
    let cached: Vec<CachedCBig> = parts
        .iter()
        .map(|&(re, im)| CachedCBig::from_parts(re.into(), im.into()))
        .collect();
    for n in 0..=parts.len() {
        let want: CBig = plain[..n].iter().product();
        let got: CachedCBig = cached[..n].iter().product();
        assert_eq!(got.as_cbig(), &want, "{n} factors");
        let got: CachedCBig = cached[..n].iter().cloned().product();
        assert_eq!(got.as_cbig(), &want, "{n} owned factors");
    }
}
