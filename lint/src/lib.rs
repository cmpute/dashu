#![feature(rustc_private)]
#![warn(unused_extern_crates)]

extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_lint;
extern crate rustc_middle;
extern crate rustc_session;

mod abs_ops;
mod if_sign_mul;
mod integer_ops;
mod powers;
mod sign_ops;
mod utils;

dylint_linting::dylint_library!();

#[unsafe(no_mangle)]
pub fn register_lints(sess: &rustc_session::Session, lint_store: &mut rustc_lint::LintStore) {
    dylint_linting::init_config(sess);
    lint_store.register_lints(&[
        if_sign_mul::IF_SIGN_MUL,
        sign_ops::MANUAL_SIGN_FROM_BOOL,
        sign_ops::MANUAL_SIGN_ARITHMETIC,
        integer_ops::MANUAL_IS_MULTIPLE_OF,
        integer_ops::MANUAL_BIT_TEST,
        integer_ops::MANUAL_DIV_REM,
        abs_ops::REDUNDANT_ABS,
        powers::PREFER_SQR_CUBIC,
        powers::UNFUSED_MUL_ADD,
    ]);
    lint_store.register_late_pass(|_| Box::new(if_sign_mul::IfSignMul));
    lint_store.register_late_pass(|_| Box::new(sign_ops::SignOps));
    lint_store.register_late_pass(|_| Box::<integer_ops::IntegerOps>::default());
    lint_store.register_late_pass(|_| Box::new(abs_ops::AbsOps));
    lint_store.register_late_pass(|_| Box::<powers::Powers>::default());
}

#[cfg(test)]
mod tests {
    macro_rules! ui_tests {
        ($($example:ident),+ $(,)?) => {$(
            #[test]
            fn $example() {
                let name = env!("CARGO_PKG_NAME").replace('-', "_");
                dylint_testing::ui_test_example(&name, stringify!($example));
            }
        )+};
    }
    ui_tests!(
        if_sign_mul,
        sign_ops,
        integer_ops,
        abs_ops,
        powers,
        test_exemptions,
        default_levels
    );
}
