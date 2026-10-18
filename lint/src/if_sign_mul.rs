use crate::utils::{Number, clone_recv, is_sign, number, peel, same_place, sign_variant};
use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::is_in_const_context;
use clippy_utils::source::snippet;
use rustc_errors::Applicability;
use rustc_hir::{BinOpKind, Expr, ExprKind, UnOp};
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::ty;
use rustc_session::{declare_lint, declare_lint_pass};

declare_lint! {
    /// Detect selection between a value and its negation based on a dashu_base::Sign.
    /// Keep the positive branch's clone and use value * sign (also supported by RBig).
    /// Only known types with the corresponding multiplication impl are diagnosed.
    pub IF_SIGN_MUL,
    Warn,
    "this branch applies a Sign to a value"
}

declare_lint_pass!(IfSignMul => [IF_SIGN_MUL]);

impl<'tcx> LateLintPass<'tcx> for IfSignMul {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        if expr.span.from_expansion() || is_in_const_context(cx) {
            return;
        }
        let ExprKind::If(cond, then, Some(otherwise)) = expr.kind else {
            return;
        };
        let ExprKind::Binary(op, a, b) = peel(cond).kind else {
            return;
        };
        if !matches!(op.node, BinOpKind::Eq | BinOpKind::Ne) {
            return;
        }
        let (sign, variant_neg) = if let Some(neg) = sign_variant(cx, a) {
            (b, neg)
        } else if let Some(neg) = sign_variant(cx, b) {
            (a, neg)
        } else {
            return;
        };
        let (negative, positive) = if variant_neg == (op.node == BinOpKind::Eq) {
            (peel(then), peel(otherwise))
        } else {
            (peel(otherwise), peel(then))
        };
        let negative = clone_recv(cx, negative).unwrap_or(negative);
        let ExprKind::Unary(UnOp::Neg, inner) = negative.kind else {
            return;
        };
        let a = clone_recv(cx, inner).unwrap_or(inner);
        let b = clone_recv(cx, positive).unwrap_or(positive);
        if !same_place(cx, a, b) {
            return;
        }
        let supported =
            matches!(
                number(cx, positive),
                Some(
                    Number::Signed
                        | Number::Float
                        | Number::CachedFloat
                        | Number::Ratio
                        | Number::Relaxed
                )
            ) || matches!(cx.typeck_results().expr_ty(positive).kind(), ty::Int(_) | ty::Float(_))
                || is_sign(cx, positive);
        if !supported {
            return;
        }
        let suggestion =
            format!("({}) * ({})", snippet(cx, positive.span, ".."), snippet(cx, sign.span, ".."));
        span_lint_and_sugg(
            cx,
            IF_SIGN_MUL,
            expr.span,
            "this `if`/`else` selects between a value and its negation based on a `Sign`",
            "consider",
            suggestion,
            Applicability::MaybeIncorrect,
        );
    }
}
