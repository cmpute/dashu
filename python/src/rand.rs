//! `dashu.rand` — random generation of the dashu numeric types.
//!
//! All functions draw from a per-thread RNG (`rand`'s `thread_rng`) and generate *uniform*
//! values: integers by bit length, floats/rationals/complexes in the unit interval/square.

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

use dashu::float::round::mode::{HalfAway, Zero};
use dashu::float::{DBig, FBig};
use dashu::integer::UBig;
use dashu::integer::rand::{UniformBits, bridge_v010};
use dashu::rational::RBig;

use crate::types::{CPy, DPy, FPy, IPy, RPy, UPy};
use crate::utils::default_precision;

/// Generate a random `UBig` with exactly `bits` random bits (uniform in `[0, 2^bits)`).
#[pyfunction]
pub fn ubig(bits: usize) -> UPy {
    let mut rng = ::rand::rng();
    UPy(UniformBits::new(bits).sample_ubig(&mut bridge_v010(&mut rng)))
}

/// Generate a random `IBig` with exactly `bits` random bits, sign chosen independently
/// (uniform in `(-2^bits, 2^bits)`, excluding `0`).
#[pyfunction]
pub fn ibig(bits: usize) -> IPy {
    let mut rng = ::rand::rng();
    IPy(UniformBits::new(bits).sample_ibig(&mut bridge_v010(&mut rng)))
}

fn checked_precision(precision: Option<usize>) -> PyResult<usize> {
    match precision {
        Some(0) => Err(PyValueError::new_err("precision must be a positive integer")),
        Some(p) => Ok(p),
        None => Ok(default_precision()),
    }
}

/// Generate a random binary `FBig` uniformly in `[0, 1)` at the given precision
/// (defaults to the module's global default precision).
#[pyfunction]
#[pyo3(signature = (precision = None))]
pub fn fbig(precision: Option<usize>) -> PyResult<FPy> {
    let mut rng = ::rand::rng();
    let p = checked_precision(precision)?;
    let v: FBig<Zero, 2> =
        dashu::float::rand::Uniform01::<2>::new(p).sample01::<Zero, _>(&mut bridge_v010(&mut rng));
    Ok(FPy(v))
}

/// Generate a random decimal `DBig` uniformly in `[0, 1)` at the given precision
/// (defaults to the module's global default precision).
#[pyfunction]
#[pyo3(signature = (precision = None))]
pub fn dbig(precision: Option<usize>) -> PyResult<DPy> {
    let mut rng = ::rand::rng();
    let p = checked_precision(precision)?;
    let v: DBig = dashu::float::rand::Uniform01::<10>::new(p)
        .sample01::<HalfAway, _>(&mut bridge_v010(&mut rng));
    Ok(DPy(v))
}

/// Generate a random `RBig` uniformly in `[0, 1)`. `max_denom_bits` caps the denominator's
/// bit length (default 64, matching `DoubleWord::MAX`); the denominator is
/// `2^max_denom_bits − 1`, the numerator is uniform below it.
#[pyfunction]
#[pyo3(signature = (max_denom_bits = 64))]
pub fn rbig(max_denom_bits: usize) -> RPy {
    if max_denom_bits == 0 {
        return RPy(RBig::from(0u8));
    }
    let mut rng = ::rand::rng();
    let limit = (UBig::ONE << max_denom_bits) - UBig::ONE;
    let dist = dashu::rational::rand::Uniform01::new(&limit);
    RPy(dist.sample_rbig(&mut bridge_v010(&mut rng)))
}

/// Generate a random `CBig` with each part uniform in `[0, 1)` (the unit square), at the
/// given precision (defaults to the module's global default precision).
#[pyfunction]
#[pyo3(signature = (precision = None))]
pub fn cbig(precision: Option<usize>) -> PyResult<CPy> {
    let mut rng = ::rand::rng();
    let p = checked_precision(precision)?;
    let dist = dashu::float::rand::Uniform01::<2>::new(p);
    let re: FBig<Zero, 2> = dist.sample01::<Zero, _>(&mut bridge_v010(&mut rng));
    let im: FBig<Zero, 2> = dist.sample01::<Zero, _>(&mut bridge_v010(&mut rng));
    Ok(CPy(dashu::complex::CBig::from_parts(re, im)))
}

/// The `dashu.rand` submodule.
#[pymodule]
pub fn rand(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(ubig, m)?)?;
    m.add_function(wrap_pyfunction!(ibig, m)?)?;
    m.add_function(wrap_pyfunction!(fbig, m)?)?;
    m.add_function(wrap_pyfunction!(dbig, m)?)?;
    m.add_function(wrap_pyfunction!(rbig, m)?)?;
    m.add_function(wrap_pyfunction!(cbig, m)?)?;
    Ok(())
}
