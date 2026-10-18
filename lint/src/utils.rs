use rustc_hir::def::Res;
use rustc_hir::{Expr, ExprKind, QPath, UnOp};
use rustc_lint::LateContext;
use rustc_middle::ty;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Number {
    Unsigned,
    Signed,
    Float,
    CachedFloat,
    Ratio,
    Relaxed,
}

pub fn number(cx: &LateContext<'_>, e: &Expr<'_>) -> Option<Number> {
    let ty::Adt(adt, _) = cx.typeck_results().expr_ty(e).peel_refs().kind() else {
        return None;
    };
    let path = cx.get_def_path(adt.did());
    let path: Vec<_> = path.iter().map(|s| s.as_str()).collect();
    match path.as_slice() {
        ["dashu_int", "ubig", "UBig"] => Some(Number::Unsigned),
        ["dashu_int", "ibig", "IBig"] => Some(Number::Signed),
        ["dashu_float", "fbig", "FBig"] => Some(Number::Float),
        ["dashu_float", "fbig_cached", "CachedFBig"] => Some(Number::CachedFloat),
        ["dashu_ratio", "rbig", "RBig"] => Some(Number::Ratio),
        ["dashu_ratio", "rbig", "Relaxed"] => Some(Number::Relaxed),
        _ => None,
    }
}

pub fn integer(cx: &LateContext<'_>, e: &Expr<'_>) -> bool {
    matches!(number(cx, e), Some(Number::Unsigned | Number::Signed))
}

pub fn float(cx: &LateContext<'_>, e: &Expr<'_>) -> bool {
    matches!(number(cx, e), Some(Number::Float | Number::CachedFloat))
}

pub fn is_sign(cx: &LateContext<'_>, e: &Expr<'_>) -> bool {
    let ty::Adt(adt, _) = cx.typeck_results().expr_ty(e).peel_refs().kind() else {
        return false;
    };
    let path = cx.get_def_path(adt.did());
    path.len() == 3
        && path[0].as_str() == "dashu_base"
        && path[1].as_str() == "sign"
        && path[2].as_str() == "Sign"
}

pub fn sign_variant(cx: &LateContext<'_>, e: &Expr<'_>) -> Option<bool> {
    if !is_sign(cx, e) {
        return None;
    }
    let ExprKind::Path(QPath::Resolved(None, path)) = e.kind else {
        return None;
    };
    match path.segments.last()?.ident.name.as_str() {
        "Negative" => Some(true),
        "Positive" => Some(false),
        _ => None,
    }
}

pub fn peel<'tcx>(e: &'tcx Expr<'tcx>) -> &'tcx Expr<'tcx> {
    match e.kind {
        ExprKind::Block(block, None) if block.stmts.is_empty() => block.expr.map_or(e, peel),
        ExprKind::DropTemps(inner) => peel(inner),
        _ => e,
    }
}

pub fn unborrow<'tcx>(e: &'tcx Expr<'tcx>) -> &'tcx Expr<'tcx> {
    match peel(e).kind {
        ExprKind::AddrOf(_, _, inner) => unborrow(inner),
        _ => peel(e),
    }
}

/// Only compare places, never calls or indexing: rewriting must not change side effects.
pub fn same_place(cx: &LateContext<'_>, a: &Expr<'_>, b: &Expr<'_>) -> bool {
    let (a, b) = (unborrow(a), unborrow(b));
    match (a.kind, b.kind) {
        (ExprKind::Path(pa), ExprKind::Path(pb)) => {
            matches!((cx.qpath_res(&pa, a.hir_id), cx.qpath_res(&pb, b.hir_id)),
                (Res::Local(x), Res::Local(y)) if x == y)
        }
        (ExprKind::Field(a, fa), ExprKind::Field(b, fb)) => {
            fa.name == fb.name && same_place(cx, a, b)
        }
        (ExprKind::Unary(UnOp::Deref, a), ExprKind::Unary(UnOp::Deref, b)) => same_place(cx, a, b),
        _ => false,
    }
}

pub fn clone_recv<'tcx>(cx: &LateContext<'_>, e: &'tcx Expr<'tcx>) -> Option<&'tcx Expr<'tcx>> {
    if let ExprKind::MethodCall(name, recv, [], _) = e.kind
        && name.ident.name.as_str() == "clone"
        && number(cx, recv).is_some()
    {
        Some(recv)
    } else {
        None
    }
}

pub fn value_place<'tcx>(cx: &LateContext<'_>, e: &'tcx Expr<'tcx>) -> &'tcx Expr<'tcx> {
    unborrow(clone_recv(cx, e).unwrap_or(e))
}

pub fn integer_constant(cx: &LateContext<'_>, e: &Expr<'_>, value: u128) -> bool {
    let e = unborrow(e);
    if let ExprKind::Path(QPath::TypeRelative(_, name)) = e.kind {
        return integer(cx, e)
            && name.ident.name.as_str() == if value == 0 { "ZERO" } else { "ONE" };
    }
    if let ExprKind::Path(QPath::Resolved(_, path)) = e.kind
        && integer(cx, e)
    {
        return path
            .segments
            .last()
            .is_some_and(|s| s.ident.name.as_str() == if value == 0 { "ZERO" } else { "ONE" });
    }
    clippy_utils::is_integer_const(cx, e, value)
}
