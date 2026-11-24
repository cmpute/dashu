//! `neg` / `conj` / `proj` / `mul_i` at the context layer: exact under the operand's own (or a
//! wider) context, rounded to the context precision — and flagged — under a narrower one.

use dashu_base::Approximation::*;
use dashu_cmplx::{CBig, Context, FBig};
use dashu_float::round::mode::HalfEven;
use dashu_float::round::Rounding;
use dashu_float::{Context as FloatContext, Repr};

type F = FBig<HalfEven, 2>;
type C = CBig<HalfEven, 2>;

fn f(sig: i32, exp: isize, p: usize) -> F {
    F::from_repr(Repr::new(sig.into(), exp), FloatContext::new(p))
}

#[test]
fn exact_ops_round_under_a_narrower_context() {
    // z = 0 + (3/8)i at 2 bits, under a 1-bit context: 3/8 is a tie, rounded to even (1/2)
    let z = C::from_parts(f(0, 0, 2), f(3, -3, 2));
    let ctx = Context::<HalfEven>::new(1);
    let zero = C::from_parts(F::ZERO, F::ZERO);

    let neg = ctx.neg(&z).unwrap();
    assert_eq!(neg, ctx.add(&(-&z), &zero).unwrap());
    assert_eq!(
        neg,
        Inexact(C::from_parts(f(0, 0, 1), f(-1, -1, 1)), (Rounding::NoOp, Rounding::SubOne))
    );
    assert!(neg.value().re().is_neg_zero());

    let conj = ctx.conj(&z).unwrap();
    assert_eq!(
        conj,
        Inexact(C::from_parts(f(0, 0, 1), f(-1, -1, 1)), (Rounding::NoOp, Rounding::SubOne))
    );
    let proj = ctx.proj(&z).unwrap();
    assert_eq!(proj, ctx.mul(&z, &C::ONE).unwrap());
    assert_eq!(
        proj,
        Inexact(C::from_parts(f(0, 0, 1), f(1, -1, 1)), (Rounding::NoOp, Rounding::AddOne))
    );
    let rot = ctx.mul_i(&z, false).unwrap();
    assert_eq!(
        rot,
        Inexact(C::from_parts(f(-1, -1, 1), f(0, 0, 1)), (Rounding::SubOne, Rounding::NoOp))
    );
    let rot = ctx.mul_i(&z, true).unwrap();
    assert_eq!(rot.value().re(), &Repr::<2>::new(1.into(), -1));

    // an operand far wider than the context (7/8 at 3 bits, and at the four standard widths):
    // every part of the result fits the context precision
    for p in [3usize, 20, 50, 100, 500] {
        let wide = (F::ONE << p as isize) - F::ONE; // p one-bits
        let w = C::from_parts(
            wide.clone().with_precision(p).value(),
            (-wide).with_precision(p).value(),
        );
        for q in [1usize, 2, p / 2 + 1] {
            let ctx = Context::<HalfEven>::new(q);
            for r in [
                ctx.neg(&w),
                ctx.conj(&w),
                ctx.proj(&w),
                ctx.mul_i(&w, false),
                ctx.mul_i(&w, true),
            ] {
                let r = r.unwrap();
                assert!(matches!(r, Inexact(..)), "p={p}, q={q}");
                let r = r.value();
                assert_eq!(r.precision(), q);
                assert!(r.re().digits() <= q && r.im().digits() <= q, "p={p}, q={q}: {r:?}");
                let _ = r.into_parts(); // must not trip the precision invariant
            }
        }
    }
}

#[test]
fn exact_ops_stay_exact_under_an_equal_or_wider_context() {
    for p in [20usize, 50, 100, 500] {
        let z = C::from_parts(f(3, -3, p), f(-5, 7, p));
        for ctx in [
            Context::<HalfEven>::new(p),
            Context::new(p + 9),
            Context::new(0),
        ] {
            let neg = ctx.neg(&z).unwrap();
            assert!(matches!(neg, Exact(_)));
            assert_eq!(neg.value(), C::from_parts(f(-3, -3, p), f(5, 7, p)));
            let conj = ctx.conj(&z).unwrap();
            assert_eq!(conj, Exact(C::from_parts(f(3, -3, p), f(5, 7, p))));
            assert_eq!(conj.value().precision(), ctx.precision());
            assert_eq!(ctx.proj(&z).unwrap(), Exact(z.clone()));
            assert_eq!(
                ctx.mul_i(&z, false).unwrap(),
                Exact(C::from_parts(f(5, 7, p), f(3, -3, p)))
            );
        }
        // the operators use the operand's own context
        assert_eq!((-&z).precision(), p);
        assert_eq!(-(-&z), z);
    }
    // a guard digit left by an inexact subtraction is not rounded away by the operand's own context
    let a = f((1 << 20) - 1, 0, 20);
    let b = f(1, -3, 20);
    let diff = a - b;
    let z = C::from_parts(diff.clone(), diff.clone());
    assert_eq!((-&z).re(), (-diff).repr());
}
