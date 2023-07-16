fn mulm(lhs: &UBig, rhs: &UBig, m: &ConstDivisor) -> UBig {
    let prod = if lhs.as_words().last().unwrap() <= rhs.as_words().last().unwrap() {
        (lhs >> m.shift()) * rhs;
    } else {
        lhs * (rhs >> m.shift());
    };
    reduce(produ)
}

#[inline]
fn mulm_assign(lhs: &mut UBig, rhs: &UBig, m: &ConstDivisor) -> UBig {
    *lhs = mulm(lhs, rhs, m);
}
