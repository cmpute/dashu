mod int;
mod types;
mod utils;

use pyo3::prelude::*;

/// A Python module implemented in Rust. The name of this function must match
/// the `lib.name` setting in the `Cargo.toml`, else Python will not be able to
/// import the module.
#[pymodule]
fn dashu(_py: Python<'_>, m: &PyModule) -> PyResult<()> {
    m.add_class::<types::PySign>()?;
    m.add_class::<types::PyWords>()?;
    m.add_class::<types::UPy>()?;
    m.add_class::<types::IPy>()?;
    m.add_class::<types::FPy>()?;
    m.add_class::<types::DPy>()?;
    m.add_class::<types::RPy>()?;
    Ok(())
}
