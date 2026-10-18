use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::source::snippet;
use rustc_errors::Applicability;
use rustc_hir::def::Res;
use rustc_hir::{BinOpKind, Expr, ExprKind, HirId, QPath, UnOp};
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::ty;
use rustc_session::{declare_lint, declare_lint_pass};

declare_lint! {
    /// ### What it does
    ///
    /// Checks for `if`/`else` expressions that select between a value and its negation based on a
    /// `dashu_base::Sign`, e.g.
    ///
    /// ```rust,ignore
    /// let hi_signed = if sign == Sign::Negative {
    ///     -hi.clone()
    /// } else {
    ///     hi.clone()
    /// };
    /// ```
    ///
    /// ### Why is this bad?
    ///
    /// Since `Sign` only has the `Positive` and `Negative` variants, the whole expression can be
    /// written as `sign * value` (via the `Mul<Sign>` impls), which is shorter and cannot get the
    /// two branches out of sync.
    ///
    /// The suggestion consumes the value instead of cloning it, so check that the value is not
    /// used afterwards. Also note that `Mul<Sign>` is not implemented for `Ball` yet, so
    /// suggestions on `Ball` values do not compile until it is.
    ///
    /// ### Example
    ///
    /// ```rust,ignore
    /// let result = if sign == Sign::Negative { -res } else { res };
    /// ```
    ///
    /// Use instead:
    ///
    /// ```rust,ignore
    /// let result = sign * res;
    /// ```
    pub IF_SIGN_MUL,
    Warn,
    "this `if`/`else` selects between a value and its negation based on a `Sign`; \
     consider `sign * value`"
}

declare_lint_pass!(IfSignMul => [IF_SIGN_MUL]);

impl<'tcx> LateLintPass<'tcx> for IfSignMul {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        if expr.span.from_expansion() {
            return;
        }
        let ExprKind::If(cond, then_body, Some(else_body)) = expr.kind else {
            return;
        };
        if matches!(else_body.kind, ExprKind::If(..)) {
            return; // else-if chain, not a simple binary selection
        }
        let cond = match cond.kind {
            ExprKind::DropTemps(inner) => inner,
            _ => cond,
        };
        let Some((sign_expr, neg_first)) = classify_cond(cx, cond) else {
            return;
        };
        let (Some(then_expr), Some(else_expr)) = (pure_block(then_body), pure_block(else_body))
        else {
            return;
        };
        let (neg_branch, pos_branch) = if neg_first {
            (then_expr, else_expr)
        } else {
            (else_expr, then_expr)
        };
        let Some((neg_operand, true)) = signed_operand(neg_branch) else {
            return;
        };
        let Some((pos_operand, false)) = signed_operand(pos_branch) else {
            return;
        };
        if !same_local(cx, neg_operand, pos_operand) {
            return;
        }
        let used_clone = is_clone_call(neg_branch) || is_clone_call(pos_branch);
        let sugg = format!(
            "{} * {}",
            snippet(cx, sign_expr.span, ".."),
            snippet(cx, neg_operand.span, "..")
        );
        span_lint_and_sugg(
            cx,
            IF_SIGN_MUL,
            expr.span,
            "this `if`/`else` selects between a value and its negation based on a `Sign`",
            "try",
            sugg,
            if used_clone {
                Applicability::MaybeIncorrect
            } else {
                Applicability::MachineApplicable
            },
        );
    }
}

/// Classify the condition of an `if`: returns the expression holding the `Sign` value and
/// whether the `then` branch is the negated one. Handles `sign ==/!= Sign::Negative/Positive`
/// in either comparison order.
fn classify_cond<'tcx>(
    cx: &LateContext<'tcx>,
    cond: &'tcx Expr<'tcx>,
) -> Option<(&'tcx Expr<'tcx>, bool)> {
    let ExprKind::Binary(op, lhs, rhs) = cond.kind else {
        return None;
    };
    let eq_first = match op.node {
        BinOpKind::Eq => true,
        BinOpKind::Ne => false,
        _ => return None,
    };
    let (sign_expr, variant) = if let Some(variant) = sign_variant(cx, lhs) {
        (rhs, variant)
    } else if let Some(variant) = sign_variant(cx, rhs) {
        (lhs, variant)
    } else {
        return None;
    };
    // `== Sign::Negative` / `!= Sign::Positive`: the `then` branch is the negated one.
    let neg_first = eq_first == (variant == "Negative");
    Some((sign_expr, neg_first))
}

/// If the expression is a path to one of `dashu_base::Sign`'s unit variants, return its name.
fn sign_variant(cx: &LateContext<'_>, e: &Expr<'_>) -> Option<&'static str> {
    let ExprKind::Path(QPath::Resolved(None, path)) = e.kind else {
        return None;
    };
    let name = match path.segments.last()?.ident.name.as_str() {
        "Negative" => "Negative",
        "Positive" => "Positive",
        _ => return None,
    };
    if !is_dashu_sign_ty(cx, cx.typeck_results().expr_ty(e)) {
        return None;
    }
    Some(name)
}

/// Whether the type is `dashu_base::Sign`. The def path is compared via `get_def_path`, which
/// includes the crate name even for local types -- `def_path_str` would not, leaving the lint
/// blind inside the very crates that define the types.
fn is_dashu_sign_ty(cx: &LateContext<'_>, ty: ty::Ty<'_>) -> bool {
    let ty::Adt(adt, _) = ty.kind() else {
        return false;
    };
    let path = cx.get_def_path(adt.did());
    path.len() == 3
        && path[0].as_str() == "dashu_base"
        && path[1].as_str() == "sign"
        && path[2].as_str() == "Sign"
}

/// `(operand, is_negated)`: peel a `.clone()` call and a unary negation.
fn signed_operand<'tcx>(e: &'tcx Expr<'tcx>) -> Option<(&'tcx Expr<'tcx>, bool)> {
    let e = clone_recv(e).unwrap_or(e);
    match e.kind {
        ExprKind::Unary(UnOp::Neg, inner) => {
            let inner = clone_recv(inner).unwrap_or(inner);
            Some((inner, true))
        }
        _ => Some((e, false)),
    }
}

fn clone_recv<'tcx>(e: &'tcx Expr<'tcx>) -> Option<&'tcx Expr<'tcx>> {
    if let ExprKind::MethodCall(segment, recv, [], _) = e.kind
        && segment.ident.name.as_str() == "clone"
    {
        Some(recv)
    } else {
        None
    }
}

fn is_clone_call(e: &Expr<'_>) -> bool {
    clone_recv(e).is_some()
}

/// An `if` branch only counts when it is a block containing just a tail expression.
fn pure_block<'tcx>(e: &'tcx Expr<'tcx>) -> Option<&'tcx Expr<'tcx>> {
    match e.kind {
        ExprKind::Block(block, None) if block.stmts.is_empty() => block.expr,
        _ => None,
    }
}

fn same_local(cx: &LateContext<'_>, a: &Expr<'_>, b: &Expr<'_>) -> bool {
    match (local_id(cx, a), local_id(cx, b)) {
        (Some(x), Some(y)) => x == y,
        _ => false,
    }
}

fn local_id(cx: &LateContext<'_>, e: &Expr<'_>) -> Option<HirId> {
    let ExprKind::Path(qpath) = e.kind else {
        return None;
    };
    match cx.qpath_res(&qpath, e.hir_id) {
        Res::Local(id) => Some(id),
        _ => None,
    }
}
