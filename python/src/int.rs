use crate::types::{UPy, IPy};

use pyo3::prelude::*;
use pyo3::exceptions::{PyTypeError, PyValueError};
use pyo3::ffi;
use pyo3::types::PyLong;

#[pymethods]
impl UPy {
    #[new]
    fn new(obj: &PyAny) -> PyResult<Self> {
        <UPy as FromPyObject>::extract(obj)
        // If #[cfg(Py_LIMITED_API)], then convert from bigint is allowed
        // If #[cfg(not(Py_LIMITED_API))], then only convert from long is allowed
    }

    fn unwrap(self) -> PyObject {
        todo!() // To builtin int
        // If #[cfg(Py_LIMITED_API)], then convert to bigint is allowed
        // If #[cfg(not(Py_LIMITED_API))], then only convert to long is allowed
    }
}

// conversion from python object
impl<'source> FromPyObject<'source> for UPy {
    fn extract(ob: &'source PyAny) -> PyResult<SubjectInput> {
        let py = ob.py();

        unsafe {
            let ptr = ob.as_ptr();
            if ffi::PyLong_Check(ptr) == 0 {
                return Err(PyValueError::new_err(
                    "Only Python integers can be automatically converted to an UBig instance."
                ));
            }
            // input is integer
            let mut overflow = 0;
            let v = ffi::PyLong_AsLongLongAndOverflow(ptr, &mut overflow);
            if v == -1 && PyErr::occurred(py) {
                Err(PyErr::fetch(py))
            } else if overflow != 0 {
                // some code below is from https://github.com/PyO3/pyo3/blob/main/src/conversions/num_bigint.rs
                let n_bits = ffi::_PyLong_NumBits(ptr) as usize;
                let n_bytes = match n_bits {
                    usize::MAX => return Err(PyErr::fetch(py)),
                    0 => 0,
                    n => (n as usize) / 8 + 1,
                };
                let long_ptr = ptr as *mut ffi::PyLongObject;
                let num_big = if n_bytes <= 64 {
                    let mut buffer = [0; 64];
                    let bptr = buffer.as_mut_ptr();
                    if ffi::_PyLong_AsByteArray(long_ptr, bptr, n_bytes, 1, 1) == -1 {
                        return Err(PyErr::fetch(py));
                    }
                    BigInt::from_signed_bytes_le(&buffer[..n_bytes])
                } else {
                    let mut buffer = vec![0; n_bytes];
                    let bptr = buffer.as_mut_ptr();
                    if ffi::_PyLong_AsByteArray(long_ptr, bptr, n_bytes, 1, 1) == -1 {
                        return Err(PyErr::fetch(py));
                    }
                    BigInt::from_signed_bytes_le(&buffer)
                };
                Ok(SubjectInput(BInt(num_big)))
            } else {
                Ok(SubjectInput(Int(v)))
            }
        }
    }
}
