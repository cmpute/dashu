use std::collections::hash_map::DefaultHasher;
use std::hash::Hasher;

use pyo3::exceptions::{PyOverflowError, PyTypeError};
use pyo3::ffi;
use pyo3::intern;
use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyDict, PyLong};

use crate::types::{IPy, UPy, PyWords};
use dashu_int::{IBig, UBig};
use num_order::NumHash;

fn py_to_long_or_big(ob: &PyAny) -> PyResult<(i64, bool)> {
    let py = ob.py();

    unsafe {
        let ptr = ob.as_ptr();
        if ffi::PyLong_Check(ptr) == 0 {
            return Err(PyTypeError::new_err(
                "only Python integers can be automatically converted to an UBig instance.",
            ));
        }

        let mut overflow: i32 = 0;
        let v = ffi::PyLong_AsLongLongAndOverflow(ptr, &mut overflow);

        if v == -1 && PyErr::occurred(py) {
            Err(PyErr::fetch(py))
        } else {
            Ok((v, overflow != 0))
        }
    }
}

impl UPy {
    // Conversion from python integer object.
    //
    // The most efficient way here is to use ffi::_PyLong_AsByteArray.
    // However, the conversion should not performed frequently, so the stable
    // API `to_bytes` is preferred here.
    fn wrap(ob: &PyAny) -> PyResult<Self> {
        let py = ob.py();

        let (v, overflow) = py_to_long_or_big(ob)?;

        if !overflow {
            if let Ok(n) = u64::try_from(v) {
                Ok(UPy(UBig::from(n)))
            } else {
                Err(PyOverflowError::new_err("can't convert negative int to unsigned"))
            }
        } else {
            let bit_len: usize = ob.call_method0(intern!(py, "bit_length"))?.extract()?;
            let byte_len = (bit_len + 7) / 8;
            let bytes: &PyBytes = ob
                .call_method1(intern!(py, "to_bytes"), (byte_len, intern!(py, "little")))?
                .downcast()?;
            Ok(UPy(UBig::from_le_bytes(bytes.as_bytes())))
        }
    }
}

#[pymethods]
impl UPy {
    #[new]
    fn __new__(ob: &PyAny, radix: Option<u32>) -> PyResult<Self> {
        let string: PyResult<&str> = ob.extract();
        if let Ok(s) = string {
            let n = if let Some(r) = radix {
                UBig::from_str_radix(s, r)
            } else {
                UBig::from_str_with_radix_prefix(s).map(|v| v.0)
            };
            Ok(UPy(n.map_err(crate::utils::parse_error_to_py)?))
        } else {
            if radix.is_some() {
                Err(PyTypeError::new_err("can't convert non-string with explicit base"))
            } else {
                Self::wrap(ob)
            }
        }
    }
    fn __repr__(&self) -> String {
        format!("<UBig {:#?}>", self.0)
    }
    fn __str__(&self) -> String {
        format!("{}", self.0)
    }
    fn __hash__(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        self.0.num_hash(&mut hasher);
        hasher.finish()
    }
    fn __int__(&self, py: Python) -> PyResult<PyObject> {
        let bytes = self.0.to_le_bytes();
        let bytes_obj = PyBytes::new(py, &bytes);
        py.get_type::<PyLong>()
            .call_method1(intern!(py, "from_bytes"), (bytes_obj, intern!(py, "little")))
            .map(PyObject::from)
    }

    fn to_words(&self) -> PyWords {
        todo!()
    }
    #[staticmethod]
    fn from_words(words: &PyWords) -> Self {
        todo!()
    }
}

impl IPy {
    // Conversion from python integer object.
    //
    // The most efficient way here is to use ffi::_PyLong_AsByteArray.
    // However, the conversion should not performed frequently, so the stable
    // API `to_bytes` is preferred here.
    fn wrap(ob: &PyAny) -> PyResult<Self> {
        let py = ob.py();

        let (v, overflow) = py_to_long_or_big(ob)?;

        if !overflow {
            Ok(IPy(IBig::from(v)))
        } else {
            let bit_len: usize = ob.call_method0(intern!(py, "bit_length"))?.extract()?;
            let byte_len = (bit_len + 7) / 8;

            let kwargs = PyDict::new(py);
            kwargs.set_item(intern!(py, "signed"), true).unwrap();
            let bytes: &PyBytes = ob
                .call_method(
                    intern!(py, "to_bytes"),
                    (byte_len, intern!(py, "little")),
                    Some(kwargs),
                )?
                .downcast()?;
            Ok(IPy(IBig::from_le_bytes(bytes.as_bytes())))
        }
    }
}

#[pymethods]
impl IPy {
    #[new]
    #[inline]
    fn __new__(ob: &PyAny, radix: Option<u32>) -> PyResult<Self> {
        let string: PyResult<&str> = ob.extract();
        if let Ok(s) = string {
            let n = if let Some(r) = radix {
                IBig::from_str_radix(s, r)
            } else {
                IBig::from_str_with_radix_prefix(s).map(|v| v.0)
            };
            Ok(IPy(n.map_err(crate::utils::parse_error_to_py)?))
        } else {
            if radix.is_some() {
                Err(PyTypeError::new_err("can't convert non-string with explicit base"))
            } else {
                Self::wrap(ob)
            }
        }
    }
    fn __repr__(&self) -> String {
        format!("<IBig {:#?}>", self.0)
    }
    fn __str__(&self) -> String {
        format!("{}", self.0)
    }
    fn __hash__(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        self.0.num_hash(&mut hasher);
        hasher.finish()
    }
    fn __int__(&self, py: Python) -> PyResult<PyObject> {
        let bytes = self.0.to_le_bytes();
        let bytes_obj = PyBytes::new(py, &bytes);

        let kwargs = PyDict::new(py);
        kwargs.set_item(intern!(py, "signed"), true).unwrap();
        py.get_type::<PyLong>()
            .call_method(
                intern!(py, "from_bytes"),
                (bytes_obj, intern!(py, "little")),
                Some(kwargs),
            )
            .map(PyObject::from)
    }
}
