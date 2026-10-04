#![feature(rustc_private)]
#![warn(unused_extern_crates)]

extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_lint;
extern crate rustc_middle;
extern crate rustc_session;

mod if_sign_mul;

dylint_linting::dylint_library!();

#[unsafe(no_mangle)]
pub fn register_lints(sess: &rustc_session::Session, lint_store: &mut rustc_lint::LintStore) {
    dylint_linting::init_config(sess);
    lint_store.register_lints(&[if_sign_mul::IF_SIGN_MUL]);
    lint_store.register_late_pass(|_| Box::new(if_sign_mul::IfSignMul));
}

#[cfg(test)]
mod tests {
    #[test]
    fn ui_examples() {
        // The library file is named with underscores (`libdashu_lints.so`), and dylint_testing
        // requires the name to match it.
        let name = env!("CARGO_PKG_NAME").replace('-', "_");
        dylint_testing::ui_test_examples(&name);
    }
}
