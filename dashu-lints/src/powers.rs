use crate::utils::{Number, float, number, peel, same_place, value_place};
use clippy_utils::{
    diagnostics::span_lint_and_help, is_in_const_context, is_in_test, is_integer_const,
};
use rustc_hir::{BinOpKind, Expr, ExprKind, HirId};
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::{declare_lint, impl_lint_pass};

declare_lint! {
    /// Detect repeated products and integer powers of two or three. Floating-point cubes
    /// change rounding boundaries, and IBig squares change the return type to UBig;
    /// consequently this lint provides help only, never an automatic replacement.
    /// Complex numbers are excluded because their multiplication and square use different kernels.
    pub PREFER_SQR_CUBIC,
    Warn,
    "a number has a dedicated square or cubic operation"
}
declare_lint! {
    /// Opt-in advice for floating multiply-then-add expressions. Fusing operations changes
    /// their rounding boundaries, even with a fixed rounding-mode type. Never auto-fix.
    pub UNFUSED_MUL_ADD,
    Allow,
    "consider a fused multiply-add if single rounding is intended"
}

#[derive(Default)]
pub struct Powers {
    covered: Vec<HirId>,
}
impl_lint_pass!(Powers => [PREFER_SQR_CUBIC, UNFUSED_MUL_ADD]);

fn product<'tcx>(expr: &'tcx Expr<'tcx>) -> Option<(&'tcx Expr<'tcx>, &'tcx Expr<'tcx>)> {
    if let ExprKind::Binary(op, a, b) = peel(expr).kind
        && op.node == BinOpKind::Mul
    {
        Some((a, b))
    } else {
        None
    }
}

impl<'tcx> LateLintPass<'tcx> for Powers {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        if expr.hir_id != peel(expr).hir_id
            || expr.span.from_expansion()
            || is_in_const_context(cx)
            || is_in_test(cx.tcx, expr.hir_id)
        {
            return;
        }
        if let ExprKind::Binary(op, a, c) = expr.kind
            && op.node == BinOpKind::Add
            && let Some((a, b)) = product(a)
            && float(cx, a)
            && float(cx, b)
            && float(cx, c)
            && cx.typeck_results().expr_ty(a).peel_refs()
                == cx.typeck_results().expr_ty(b).peel_refs()
            && cx.typeck_results().expr_ty(a).peel_refs()
                == cx.typeck_results().expr_ty(c).peel_refs()
        {
            span_lint_and_help(
                cx,
                UNFUSED_MUL_ADD,
                expr.span,
                "this floating-point expression rounds the multiplication before the addition",
                None,
                "consider `a.fma(&b, &c, Sign::Positive)` if single rounding is intended; this can change the result",
            );
        }
        if self.covered.contains(&expr.hir_id) {
            return;
        }
        let (base, power) = if let Some((a, b)) = product(expr) {
            let mut cube = None;
            for (inner, outer) in [(a, b), (b, a)] {
                if let Some((x, y)) = product(inner)
                    && same_place(cx, value_place(cx, x), value_place(cx, y))
                    && same_place(cx, value_place(cx, x), value_place(cx, outer))
                {
                    cube = Some((x, inner.hir_id));
                    break;
                }
            }
            if let Some((base, inner)) = cube {
                self.covered.push(inner);
                (base, 3)
            } else if same_place(cx, value_place(cx, a), value_place(cx, b)) {
                (a, 2)
            } else {
                return;
            }
        } else if let ExprKind::MethodCall(name, base, [exp], _) = expr.kind
            && name.ident.name.as_str() == "pow"
            && matches!(number(cx, base), Some(Number::Unsigned | Number::Signed))
        {
            if is_integer_const(cx, exp, 2) {
                (base, 2)
            } else if is_integer_const(cx, exp, 3) {
                (base, 3)
            } else {
                return;
            }
        } else {
            return;
        };
        let Some(kind) = number(cx, base) else {
            return;
        };
        let help = match (kind, power) {
            (Number::Signed, 2) => {
                "consider `sqr()`; its result is UBig, so convert back to IBig when required"
            }
            (Number::Float | Number::CachedFloat, 3) => {
                "consider `cubic()` if single rounding of the exact cube is intended; it can differ from two rounded multiplications"
            }
            (_, 2) => "use the dedicated `sqr()` method",
            _ => "use the dedicated `cubic()` method",
        };
        span_lint_and_help(
            cx,
            PREFER_SQR_CUBIC,
            expr.span,
            "this number has a dedicated square or cubic operation",
            None,
            help,
        );
    }
}
