// PyO3 0.29 deprecates the automatic `FromPyObject` impl for `Clone` pyclasses in favor of
// an explicit opt-in (`#[pyclass(from_py_object)]`). We extract pyclasses via `PyRef` (which
// has its own impl) and as `&T` arguments, so the deprecated auto-impl is harmless here; it is
// tracked as a follow-up to switch to the explicit derive before the auto-impl is removed.
#![allow(deprecated)]

mod cache;
mod complex;
mod convert;
mod float;
mod format;
mod int;
mod math;
mod rational;
mod types;
mod utils;
mod words;

use pyo3::prelude::*;

/// A Python module implemented in Rust. The name of this function must match
/// the `lib.name` setting in the `Cargo.toml`, else Python will not be able to
/// import the module.
#[pymodule]
fn dashu(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<types::PySign>()?;
    m.add_class::<types::PyWords>()?;
    m.add_class::<types::UPy>()?;
    m.add_class::<types::IPy>()?;
    m.add_class::<types::FPy>()?;
    m.add_class::<types::DPy>()?;
    m.add_class::<types::RPy>()?;
    m.add_class::<types::CPy>()?;
    m.add_class::<cache::PyCache>()?;

    m.add_function(wrap_pyfunction!(utils::auto, m)?)?;
    m.add_function(wrap_pyfunction!(utils::autos, m)?)?;
    m.add_function(wrap_pyfunction!(utils::get_precision, m)?)?;
    m.add_function(wrap_pyfunction!(utils::set_precision, m)?)?;

    // module-level math functions
    m.add_function(wrap_pyfunction!(math::sin, m)?)?;
    m.add_function(wrap_pyfunction!(math::cos, m)?)?;
    m.add_function(wrap_pyfunction!(math::tan, m)?)?;
    m.add_function(wrap_pyfunction!(math::asin, m)?)?;
    m.add_function(wrap_pyfunction!(math::acos, m)?)?;
    m.add_function(wrap_pyfunction!(math::atan, m)?)?;
    m.add_function(wrap_pyfunction!(math::atan2, m)?)?;
    m.add_function(wrap_pyfunction!(math::sinh, m)?)?;
    m.add_function(wrap_pyfunction!(math::cosh, m)?)?;
    m.add_function(wrap_pyfunction!(math::tanh, m)?)?;
    m.add_function(wrap_pyfunction!(math::asinh, m)?)?;
    m.add_function(wrap_pyfunction!(math::acosh, m)?)?;
    m.add_function(wrap_pyfunction!(math::atanh, m)?)?;
    m.add_function(wrap_pyfunction!(math::exp, m)?)?;
    m.add_function(wrap_pyfunction!(math::expm1, m)?)?;
    m.add_function(wrap_pyfunction!(math::log, m)?)?;
    m.add_function(wrap_pyfunction!(math::log1p, m)?)?;
    m.add_function(wrap_pyfunction!(math::ln, m)?)?;
    m.add_function(wrap_pyfunction!(math::ln_1p, m)?)?;
    m.add_function(wrap_pyfunction!(math::sqrt, m)?)?;
    m.add_function(wrap_pyfunction!(math::cbrt, m)?)?;
    m.add_function(wrap_pyfunction!(math::nth_root, m)?)?;
    m.add_function(wrap_pyfunction!(math::powf, m)?)?;
    m.add_function(wrap_pyfunction!(math::powi, m)?)?;
    m.add_function(wrap_pyfunction!(math::hypot, m)?)?;
    m.add_function(wrap_pyfunction!(math::gcd, m)?)?;
    m.add_function(wrap_pyfunction!(math::gcd_ext, m)?)?;
    m.add_function(wrap_pyfunction!(math::lcm, m)?)?;
    Ok(())
}
