// Utility functions for experiments
use std::vec::Vec;
use either::Either;
use concrete_ntt::{native32, native64, prime32, prime64};
use pyo3::{exceptions::PyRuntimeError, prelude::*};

#[pyclass]
#[derive(Clone)]
struct NttPlan32(Either<prime32::Plan, native32::Plan32>);

#[pyclass]
#[derive(Clone)]
struct NttPlan64(Either<prime64::Plan, native64::Plan32>);

type FFT5x32 = (Vec<u32>, Vec<u32>, Vec<u32>, Vec<u32>, Vec<u32>);

#[pymethods]
impl NttPlan32 {
    #[new]
    fn try_new(size: usize, modulus: Option<u32>) -> PyResult<Self> {
        let err = PyRuntimeError::new_err("failed to create plan");
        Ok(Self(if let Some(m) = modulus {
            Either::Left(prime32::Plan::try_new(size, m).ok_or(err)?)
        } else {
            Either::Right(native32::Plan32::try_new(size).ok_or(err)?)
        }))
    }

    fn __repr__(&self) -> String {
        match &self.0 {
            Either::Left(plan) => format!("{:#?}", plan),
            Either::Right(plan) => format!("{:#?}", plan),
        }
    }
}

#[pymethods]
impl NttPlan64 {
    #[new]
    fn try_new(size: usize, modulus: Option<u64>) -> PyResult<Self> {
        let err = PyRuntimeError::new_err("failed to create plan");
        Ok(Self(if let Some(m) = modulus {
            Either::Left(prime64::Plan::try_new(size, m).ok_or(err)?)
        } else {
            Either::Right(native64::Plan32::try_new(size).ok_or(err)?)
        }))
    }

    fn __repr__(&self) -> String {
        match &self.0 {
            Either::Left(plan) => format!("{:#?}", plan),
            Either::Right(plan) => format!("{:#?}", plan),
        }
    }

    fn fwd(&self, mut buffer: Vec<u64>) -> Either<Vec<u64>, FFT5x32> {
        match &self.0 {
            Either::Left(plan) => {
                plan.fwd(&mut buffer);
                Either::Left(buffer)
            },
            Either::Right(plan) => {
                let mut output = (Vec::new(), Vec::new(), Vec::new(), Vec::new(), Vec::new());
                output.0.resize(buffer.len(), 0);
                output.1.resize(buffer.len(), 0);
                output.2.resize(buffer.len(), 0);
                output.3.resize(buffer.len(), 0);
                output.4.resize(buffer.len(), 0);
                plan.fwd(&buffer, &mut output.0, &mut output.1, &mut output.2, &mut output.3, &mut output.4);
                Either::Right(output)
            }
        }
    }

    fn inv(&self, input: Either<Vec<u64>, FFT5x32>) -> Vec<u64> {
        match &self.0 {
            Either::Left(plan) => {
                let mut input = input.unwrap_left();
                plan.inv(&mut input);
                input
            },
            Either::Right(plan) => {
                let mut input = input.unwrap_right();
                let mut buffer = Vec::new();
                buffer.resize(input.0.len(), 0);
                plan.inv(&mut buffer, &mut input.0, &mut input.1, &mut input.2, &mut input.3, &mut input.4);
                buffer
            }
        }
    }

    fn polymul(&self, mut lhs: Vec<u64>, mut rhs: Vec<u64>) -> Vec<u64> {
        match &self.0 {
            Either::Left(plan) => {
                plan.fwd(&mut lhs);
                plan.fwd(&mut rhs);
                plan.mul_assign_normalize(&mut lhs, &rhs);
                plan.inv(&mut lhs);
                lhs
            },
            Either::Right(plan) => {
                let mut output = Vec::new();
                output.resize(lhs.len(), 0);
                plan.negacyclic_polymul(&mut output, &lhs, &rhs);
                output
            }
        }
    }

    fn mul(&self, lhs: Either<Vec<u64>, FFT5x32>, rhs: Either<Vec<u64>, FFT5x32>) -> Either<Vec<u64>, FFT5x32> {
        match &self.0 {
            Either::Left(plan) => {
                let (lhs, rhs) = (lhs.unwrap_left(), rhs.unwrap_left());
                let mut output = Vec::new();
                output.resize(lhs.len(), 0);
                plan.mul_accumulate(&mut output, &lhs, &rhs);
                Either::Left(output)
            },
            Either::Right(_plan) => {
                todo!()
            }
        }
    }
}

pub fn register(py: Python<'_>, m: &PyModule) -> PyResult<()> {
    let module = PyModule::new(py, "experimental")?;
    module.add_class::<NttPlan32>()?;
    module.add_class::<NttPlan64>()?;
    module.add("P_SOLINAS", prime64::Solinas::P)?;

    m.add_submodule(module)
}
