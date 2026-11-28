use crate::utils::{Number, number, peel, unborrow};
use clippy_utils::{diagnostics::span_lint_and_help, is_in_const_context, is_in_test};
use rustc_hir::{AssignOpKind, BinOpKind, Expr, ExprKind, UnOp};
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::ty;
use rustc_session::{declare_lint, impl_lint_pass};

declare_lint! {
    /// Detect a computed primitive integer that is converted to a Dashu integer only to be the
    /// right operand of `+`, `-`, `*` or `/`, all of which take the primitive directly. Literals
    /// and tests are skipped: `IBig::from(2)` is as clear as `2u8`, and tests keep their operator
    /// coverage between converted and primitive operands.
    pub REDUNDANT_BIG_CONVERSION,
    Warn,
    "a primitive integer is converted to a Dashu integer only for an operator that takes it directly"
}

pub struct Conversion;
impl_lint_pass!(Conversion => [REDUNDANT_BIG_CONVERSION]);

/// Whether the right operand of the operator can be a primitive integer with the result
/// unchanged: it is still the Dashu integer. (`%` and the bit operators return the primitive
/// instead, and no operator takes a Dashu integer on the right of a primitive.)
fn takes_primitive(op: BinOpKind) -> bool {
    matches!(op, BinOpKind::Add | BinOpKind::Sub | BinOpKind::Mul | BinOpKind::Div)
}

fn takes_primitive_assign(op: AssignOpKind) -> bool {
    matches!(
        op,
        AssignOpKind::AddAssign
            | AssignOpKind::SubAssign
            | AssignOpKind::MulAssign
            | AssignOpKind::DivAssign
    )
}

fn primitive_integer(cx: &LateContext<'_>, expr: &Expr<'_>) -> bool {
    matches!(cx.typeck_results().expr_ty(expr).peel_refs().kind(), ty::Int(_) | ty::Uint(_))
}

/// A literal, negated or not.
fn literal(expr: &Expr<'_>) -> bool {
    match expr.kind {
        ExprKind::Lit(_) => true,
        ExprKind::Unary(UnOp::Neg, inner) => matches!(inner.kind, ExprKind::Lit(_)),
        _ => false,
    }
}

/// The Dashu integer that `expr` converts a computed primitive integer into, when `expr` is
/// nothing but conversions: `from`, `from_word` or `from_dword` of `dashu-int`, nested or not.
/// A literal primitive does not count: the operators cannot infer the type of a bare one, so
/// converting it is the clearer spelling.
fn converted_primitive<'tcx>(cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) -> Option<Number> {
    let expr = peel(expr);
    let target = number(cx, expr)?;
    // only the Dashu integers have the operators with primitives
    if !matches!(target, Number::Unsigned | Number::Signed) {
        return None;
    }
    let ExprKind::Call(func, [argument]) = expr.kind else {
        return None;
    };
    // Only the conversions themselves count, whatever a function of another name or crate is
    // called. (`IBig::from` resolves to the method of the `From` trait; the Dashu integer it
    // produces is already checked above.)
    let def = cx.typeck_results().type_dependent_def_id(func.hir_id)?;
    let path = cx.get_def_path(def);
    let path: Vec<_> = path.iter().map(|s| s.as_str()).collect();
    match path.as_slice() {
        ["core", "convert", "From", "from"] => {}
        ["dashu_int", .., "from_word" | "from_dword"] => {}
        _ => return None,
    }
    let argument = unborrow(argument);
    // `IBig::from(UBig::from_dword(k))` is a chain: only where it starts matters
    if converted_primitive(cx, argument).is_some() {
        return Some(target);
    }
    (primitive_integer(cx, argument) && !literal(argument)).then_some(target)
}

impl<'tcx> LateLintPass<'tcx> for Conversion {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        if expr.span.from_expansion() || is_in_const_context(cx) || is_in_test(cx.tcx, expr.hir_id)
        {
            return;
        }
        let (takes, lhs, rhs) = match expr.kind {
            ExprKind::Binary(op, lhs, rhs) => (takes_primitive(op.node), lhs, rhs),
            ExprKind::AssignOp(op, lhs, rhs) => (takes_primitive_assign(op.node), lhs, rhs),
            _ => return,
        };
        if !takes {
            return;
        }
        let Some(converted) = converted_primitive(cx, rhs) else {
            return;
        };
        // Both sides must be the same Dashu integer, the only one the operator is defined for
        if number(cx, lhs) != Some(converted) {
            return;
        }
        span_lint_and_help(
            cx,
            REDUNDANT_BIG_CONVERSION,
            rhs.span,
            "this primitive integer is converted to a Dashu integer only for the operator",
            None,
            "the operator takes the primitive integer directly",
        );
    }
}
