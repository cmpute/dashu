use crate::utils::{is_sign, peel, sign_variant};
use clippy_utils::{diagnostics::span_lint_and_help, is_in_const_context};
use rustc_hir::{BinOpKind, Expr, ExprKind};
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::{declare_lint, declare_lint_pass};

declare_lint! {
    /// Detect boolean-to-Sign branches outside const contexts. `true` means Negative.
    pub MANUAL_SIGN_FROM_BOOL,
    Warn,
    "a boolean branch constructs a Sign; use Sign::from"
}
declare_lint! {
    /// Detect manual Sign multiplication, negation, and identity. More specific than
    /// MANUAL_SIGN_FROM_BOOL so a sign operation receives just one diagnostic.
    pub MANUAL_SIGN_ARITHMETIC,
    Warn,
    "a branch manually computes a Sign operation"
}

declare_lint_pass!(SignOps => [MANUAL_SIGN_FROM_BOOL, MANUAL_SIGN_ARITHMETIC]);

impl<'tcx> LateLintPass<'tcx> for SignOps {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        if expr.span.from_expansion() || is_in_const_context(cx) {
            return;
        }
        let ExprKind::If(cond, then, Some(otherwise)) = expr.kind else {
            return;
        };
        let (Some(then_neg), Some(else_neg)) =
            (sign_variant(cx, peel(then)), sign_variant(cx, peel(otherwise)))
        else {
            return;
        };
        if then_neg == else_neg {
            return;
        }
        let arithmetic = if let ExprKind::Binary(op, a, b) = peel(cond).kind
            && matches!(op.node, BinOpKind::Eq | BinOpKind::Ne)
            && is_sign(cx, a)
            && is_sign(cx, b)
        {
            // Comparing with a variant is either the identity or negation. Comparing
            // two signs constructs their product (possibly with its sign flipped).
            if let Some(variant_neg) = sign_variant(cx, a).or_else(|| sign_variant(cx, b)) {
                Some(if then_neg == (variant_neg == (op.node == BinOpKind::Eq)) {
                    "use the original sign directly"
                } else {
                    "use unary `-` on the original sign"
                })
            } else {
                Some(if then_neg == (op.node == BinOpKind::Ne) {
                    "use the product of the two signs"
                } else {
                    "use the negated product of the two signs"
                })
            }
        } else {
            None
        };
        let (lint, message, help) = if let Some(help) = arithmetic {
            (MANUAL_SIGN_ARITHMETIC, "this branch manually computes a sign operation", help)
        } else {
            (
                MANUAL_SIGN_FROM_BOOL,
                "this branch converts a boolean to a sign",
                if then_neg {
                    "use `Sign::from(condition)`"
                } else {
                    "use `Sign::from(!condition)`"
                },
            )
        };
        span_lint_and_help(cx, lint, expr.span, message, None, help);
    }
}
