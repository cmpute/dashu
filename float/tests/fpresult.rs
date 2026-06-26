//! Cross-cutting `FpResult`/`FpError` contract tests: infinite inputs → `Err`,
//! infinite outputs → `Ok(±inf)`, and domain/indeterminate errors that span several
//! operations. Per-operation cases (div-by-zero, `0/0`, `exp` overflow, `powf` zero base,
//! `shr_assign`, …) live in the `#[cfg(test)] mod tests` blocks next to each operation.

use dashu_base::Approximation::*;
use dashu_base::Sign;
use dashu_float::{
    math::{FpError, FpResult},
    round::mode,
    Context, FBig, Repr,
};

fn r2(sig: i32, exp: isize) -> Repr<2> {
    Repr::new(sig.into(), exp)
}

#[test]
fn test_domain_errors() {
    let ctx = Context::<mode::HalfEven>::new(53);
    assert_eq!(ctx.sqrt::<2>(&r2(-1, 0)), Err(FpError::OutOfDomain));
    assert_eq!(ctx.ln::<2>(&r2(-1, 0), None), Err(FpError::OutOfDomain));
    assert_eq!(ctx.asin::<2>(&r2(2, 0), None), Err(FpError::OutOfDomain));
    assert_eq!(
        ctx.atan2::<2>(&Repr::<2>::zero(), &Repr::<2>::zero(), None),
        Err(FpError::OutOfDomain)
    );
}

#[test]
fn test_infinite_input_is_error() {
    let ctx = Context::<mode::HalfEven>::new(53);
    let inf = Repr::<2>::infinity();
    assert_eq!(ctx.add::<2>(&inf, &r2(1, 0)), Err(FpError::InfiniteInput));
    assert_eq!(ctx.mul::<2>(&inf, &r2(1, 0)), Err(FpError::InfiniteInput));
    assert_eq!(ctx.sqrt::<2>(&inf), Err(FpError::InfiniteInput));
    // exp(+inf) = +inf, exp(-inf) = +0
    assert!(ctx.unwrap_fp(ctx.exp::<2>(&inf, None)).repr().is_infinite());
    assert_eq!(ctx.exp::<2>(&inf, None).unwrap().value().repr().sign(), Sign::Positive);
    assert!(ctx
        .unwrap_fp(ctx.exp::<2>(&Repr::<2>::neg_infinity(), None))
        .repr()
        .is_zero());
    assert_eq!(ctx.sin::<2>(&inf, None), Err(FpError::InfiniteInput));
}

#[test]
fn test_fpresult_type_alias() {
    // FpResult is Result<Rounded<T>, FpError>.
    let r: FpResult<FBig> = Ok(Exact(FBig::ZERO));
    assert!(r.is_ok());
}
