use crate::utils::{integer, number};
use clippy_utils::diagnostics::span_lint_and_help;
use rustc_hir::{BinOpKind, Expr, ExprKind};
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::{declare_lint, declare_lint_pass};

declare_lint! {
    /// Detect absolute values immediately discarded by magnitude-only operations.
    /// Complex norms and ordinary comparisons against possibly negative values are excluded.
    pub REDUNDANT_ABS,
    Warn,
    "this magnitude-only operation does not need an absolute-value temporary"
}
declare_lint_pass!(AbsOps => [REDUNDANT_ABS]);

fn abs_recv<'tcx>(cx: &LateContext<'_>, e: &'tcx Expr<'tcx>) -> Option<&'tcx Expr<'tcx>> {
    if let ExprKind::MethodCall(name, recv, [], _) = e.kind
        && name.ident.name.as_str() == "abs"
        && number(cx, recv).is_some()
    {
        Some(recv)
    } else {
        None
    }
}

impl<'tcx> LateLintPass<'tcx> for AbsOps {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        if expr.span.from_expansion() {
            return;
        }
        let help = match expr.kind {
            ExprKind::MethodCall(name, recv, [_], _)
                if name.ident.name.as_str() == "abs_cmp" && abs_recv(cx, recv).is_some() =>
            {
                "call `abs_cmp` directly on the original value"
            }
            ExprKind::MethodCall(name, recv, [], _)
                if name.ident.name.as_str() == "log2_bounds"
                    && let Some(abs) = abs_recv(cx, recv)
                    && integer(cx, abs) =>
            {
                // EstimatedLog2 on IBig already measures the magnitude. In particular,
                // `x.clone().abs().log2_bounds()` needs neither the clone nor the abs.
                "call `log2_bounds` directly on the original integer without cloning or taking its absolute value"
            }
            ExprKind::MethodCall(name, recv, [rhs], _)
                if name.ident.name.as_str() == "cmp"
                    && abs_recv(cx, recv).is_some()
                    && abs_recv(cx, crate::utils::unborrow(rhs)).is_some() =>
            {
                "compare the original values with `dashu_base::AbsOrd::abs_cmp`"
            }
            ExprKind::Binary(op, a, b)
                if matches!(
                    op.node,
                    BinOpKind::Eq
                        | BinOpKind::Ne
                        | BinOpKind::Lt
                        | BinOpKind::Le
                        | BinOpKind::Gt
                        | BinOpKind::Ge
                ) && abs_recv(cx, a).is_some()
                    && abs_recv(cx, b).is_some() =>
            {
                "compare the original values with `dashu_base::AbsOrd::abs_cmp`"
            }
            _ => return,
        };
        span_lint_and_help(
            cx,
            REDUNDANT_ABS,
            expr.span,
            "this magnitude-only operation does not need an absolute-value temporary",
            None,
            help,
        );
    }
}
