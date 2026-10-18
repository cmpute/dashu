use crate::utils::{integer, integer_constant, number, peel, same_place, unborrow};
use clippy_utils::{diagnostics::span_lint_and_help, is_in_const_context, is_in_test};
use rustc_hir::{BinOpKind, Block, Expr, ExprKind, HirId, Stmt, StmtKind};
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::{declare_lint, impl_lint_pass};

declare_lint! {
    /// Detect a Dashu integer remainder used only to test divisibility.
    /// Ignore tests, which may intentionally compare the remainder and divisibility kernels.
    pub MANUAL_IS_MULTIPLE_OF,
    Warn,
    "an integer remainder is used only to test divisibility"
}
declare_lint! {
    /// Detect a one-bit integer mask used as a boolean test. Primitive integers are excluded.
    pub MANUAL_BIT_TEST,
    Warn,
    "an integer mask can be tested with BitTest::bit"
}
declare_lint! {
    /// Detect adjacent quotient/remainder calculations on unchanged integer places, including
    /// a remainder guard returning early before computing the quotient. No dataflow is assumed
    /// across intervening statements. Tests retain their independent operator comparisons.
    pub MANUAL_DIV_REM,
    Warn,
    "the same integer operands are divided separately for quotient and remainder"
}

#[derive(Default)]
pub struct IntegerOps {
    combined: Vec<HirId>,
}
impl_lint_pass!(IntegerOps => [MANUAL_IS_MULTIPLE_OF, MANUAL_BIT_TEST, MANUAL_DIV_REM]);

fn zero_test<'tcx>(
    cx: &LateContext<'_>,
    expr: &'tcx Expr<'tcx>,
) -> Option<(&'tcx Expr<'tcx>, bool)> {
    let expr = peel(expr);
    match expr.kind {
        ExprKind::MethodCall(name, recv, [], _) if name.ident.name.as_str() == "is_zero" => {
            Some((peel(recv), true))
        }
        ExprKind::Binary(op, a, b) if matches!(op.node, BinOpKind::Eq | BinOpKind::Ne) => {
            let value = if integer_constant(cx, a, 0) {
                b
            } else if integer_constant(cx, b, 0) {
                a
            } else {
                return None;
            };
            Some((peel(value), op.node == BinOpKind::Eq))
        }
        _ => None,
    }
}

fn binary<'tcx>(
    expr: &'tcx Expr<'tcx>,
    kind: BinOpKind,
) -> Option<(&'tcx Expr<'tcx>, &'tcx Expr<'tcx>)> {
    if let ExprKind::Binary(op, a, b) = peel(expr).kind
        && op.node == kind
    {
        Some((a, b))
    } else {
        None
    }
}

fn init<'tcx>(stmt: &'tcx Stmt<'tcx>) -> Option<&'tcx Expr<'tcx>> {
    match stmt.kind {
        StmtKind::Let(local) if local.els.is_none() => local.init,
        _ => None,
    }
}

fn exits(expr: &Expr<'_>) -> bool {
    match expr.kind {
        ExprKind::Ret(_) => true,
        ExprKind::Block(block, None) => match (block.stmts, block.expr) {
            ([], Some(expr)) => exits(expr),
            ([stmt], None) => {
                matches!(stmt.kind, StmtKind::Semi(e) | StmtKind::Expr(e) if exits(e))
            }
            _ => false,
        },
        _ => false,
    }
}

impl<'tcx> LateLintPass<'tcx> for IntegerOps {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        if expr.hir_id != peel(expr).hir_id
            || expr.span.from_expansion()
            || is_in_const_context(cx)
            || is_in_test(cx.tcx, expr.hir_id)
        {
            return;
        }
        let Some((value, _)) = zero_test(cx, expr).or_else(|| {
            if let ExprKind::MethodCall(name, recv, [], _) = expr.kind
                && name.ident.name.as_str() == "is_one"
            {
                Some((peel(recv), false))
            } else {
                None
            }
        }) else {
            return;
        };
        if let Some((a, b)) = binary(value, BinOpKind::BitAnd)
            && integer(cx, a)
            && number(cx, a) == number(cx, b)
            && (integer_constant(cx, a, 1) || integer_constant(cx, b, 1))
        {
            span_lint_and_help(
                cx,
                MANUAL_BIT_TEST,
                expr.span,
                "this integer mask only tests the least significant bit",
                None,
                "use `value.bit(0)` (or its negation) with `dashu_base::BitTest`",
            );
        } else if let Some((a, b)) = binary(value, BinOpKind::Rem)
            && zero_test(cx, expr).is_some()
            && integer(cx, a)
            && number(cx, a) == number(cx, b)
            && !self.combined.contains(&expr.hir_id)
        {
            span_lint_and_help(
                cx,
                MANUAL_IS_MULTIPLE_OF,
                expr.span,
                "this remainder is only used to test divisibility",
                None,
                "use `dividend.is_multiple_of(&divisor)` (or its negation)",
            );
        }
    }

    fn check_block(&mut self, cx: &LateContext<'tcx>, block: &'tcx Block<'tcx>) {
        if block.span.from_expansion()
            || is_in_const_context(cx)
            || is_in_test(cx.tcx, block.hir_id)
        {
            return;
        }
        for pair in block.stmts.windows(2) {
            let Some(second) = init(&pair[1]) else {
                continue;
            };
            let first = init(&pair[0]);
            let mut guard = None;
            let operands = if let Some(first) = first {
                binary(first, BinOpKind::Div)
                    .zip(binary(second, BinOpKind::Rem))
                    .or_else(|| binary(first, BinOpKind::Rem).zip(binary(second, BinOpKind::Div)))
            } else if let StmtKind::Semi(e) | StmtKind::Expr(e) = pair[0].kind
                && let ExprKind::If(cond, then, None) = e.kind
                && exits(then)
                && let Some((rem, false)) = zero_test(cx, cond)
            {
                guard = Some(peel(cond).hir_id);
                binary(rem, BinOpKind::Rem).zip(binary(second, BinOpKind::Div))
            } else {
                None
            };
            if let Some(((a, b), (c, d))) = operands
                && integer(cx, a)
                && same_place(cx, a, c)
                && same_place(cx, b, d)
                // A let binding must not shadow one of the original operand names.
                // HirId comparison below also rejects shadowed paths in the second operation.
                && number(cx, unborrow(a)) == number(cx, unborrow(c))
            {
                if let Some(id) = guard {
                    self.combined.push(id);
                }
                span_lint_and_help(
                    cx,
                    MANUAL_DIV_REM,
                    second.span,
                    "these integer operands are divided separately for quotient and remainder",
                    None,
                    "compute both with `dashu_base::DivRem::div_rem`; for a checked exact quotient, consider `DivExact`",
                );
            }
        }
    }
}
