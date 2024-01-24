use std::collections::hash_map::DefaultHasher;
use std::hash::Hasher;
use std::os::raw::c_longlong;

use dashu_base::BitTest;
use pyo3::exceptions::{PyIndexError, PyNotImplementedError};
use pyo3::prelude::*;
use pyo3::types::PySlice;
use pyo3::{
    exceptions::{PyOverflowError, PyTypeError},
    ffi, intern,
    types::{PyBytes, PyDict, PyLong},
};

use crate::types::{IPy, PyWords, UPy};
use crate::utils::parse_signed_index;
use dashu_int::{IBig, UBig};
use num_order::NumHash;

// error messages
const ERRMSG_LENGTH_TOO_LARGE: &'static str = "the integer has too many bits for indexing";
const ERRMSG_STEPSIZE_TOO_LARGE: &'static str =
    "bit slicing with step size larger than 1 is not supported yet";
const ERRMSG_UBIG_WRONG_SRC_TYPE: &'static str =
    "only integers or strings can be converted to a UBig instance";
const ERRMSG_INT_WITH_RADIX: &'static str = "can't convert non-string with explicit base";
const ERRMSG_WRONG_INDEX_TYPE: &'static str = "indices must be integers or slices";
const ERRMSG_UBIG_FROM_NEG: &'static str = "can't convert negative int to unsigned";
const ERRMSG_UBIG_BITS_OOR: &'static str = "bits index out of range";

fn py_to_long_or_big(ob: &PyAny) -> PyResult<(c_longlong, bool)> {
    let py = ob.py();

    unsafe {
        let ptr = ob.as_ptr();
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
    // Conversion from python integer object, without type checking.
    //
    // The most efficient way here is to use ffi::_PyLong_AsByteArray.
    // However, the conversion should not performed frequently, so the stable
    // API `to_bytes` is preferred here.
    fn wrap(ob: &PyAny) -> PyResult<Self> {
        let (v, overflow) = py_to_long_or_big(ob)?;
        if !overflow {
            if let Ok(n) = u64::try_from(v) {
                Ok(UPy(UBig::from(n)))
            } else {
                Err(PyOverflowError::new_err(ERRMSG_UBIG_FROM_NEG))
            }
        } else {
            let py = ob.py();
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
        if ob.is_instance_of::<PyLong>() {
            if radix.is_some() {
                Err(PyTypeError::new_err(ERRMSG_INT_WITH_RADIX))
            } else {
                Self::wrap(ob)
            }
        } else if let Ok(s) = ob.extract() {
            let n = if let Some(r) = radix {
                UBig::from_str_radix(s, r)
            } else {
                UBig::from_str_with_radix_prefix(s).map(|v| v.0)
            };
            Ok(UPy(n.map_err(crate::utils::parse_error_to_py)?))
        } else {
            Err(PyTypeError::new_err(ERRMSG_UBIG_WRONG_SRC_TYPE))
        }
    }
    fn __repr__(&self) -> String {
        format!("<UBig {:#?}>", self.0)
    }
    fn __str__(&self) -> String {
        format!("{}", self.0)
    }
    fn __format__(&self) {
        todo!()
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

    // use as a bit vector
    fn __len__(&self) -> usize {
        self.0.bit_len()
    }
    fn __getitem__(&self, index: &PyAny) -> PyResult<PyObject> {
        let py = index.py();
        if let Ok(i) = <isize as FromPyObject>::extract(index) {
            let i = parse_signed_index(i, self.0.bit_len(), true)
                .ok_or(PyIndexError::new_err(ERRMSG_UBIG_BITS_OOR))?;
            Ok(self.0.bit(i).into_py(py))
        } else if let Ok(range) = index.downcast::<PySlice>() {
            let len = self
                .0
                .bit_len()
                .try_into()
                .map_err(|_| PyNotImplementedError::new_err(ERRMSG_LENGTH_TOO_LARGE))?;
            let indices = range.indices(len)?;
            if indices.step != 1 {
                return Err(PyNotImplementedError::new_err(ERRMSG_STEPSIZE_TOO_LARGE));
            }

            let mut data = self.0.clone();
            data.clear_high_bits(indices.stop as _);
            let split = Self(data.split_bits(indices.start as _).1);
            Ok(split.into_py(py))
        } else {
            Err(PyTypeError::new_err(ERRMSG_WRONG_INDEX_TYPE))
        }
    }
    fn __setitem__(&mut self, index: &PyAny, set: bool) -> PyResult<()> {
        if let Ok(i) = <isize as FromPyObject>::extract(index) {
            let i = parse_signed_index(i, self.0.bit_len(), true)
                .ok_or(PyIndexError::new_err(ERRMSG_UBIG_BITS_OOR))?;
            if set {
                self.0.set_bit(i)
            } else {
                self.0.clear_bit(i)
            }
            Ok(())
        } else if let Ok(range) = index.downcast::<PySlice>() {
            let len = self
                .0
                .bit_len()
                .try_into()
                .map_err(|_| PyNotImplementedError::new_err(ERRMSG_LENGTH_TOO_LARGE))?;
            let indices = range.indices(len)?;
            if indices.step != 1 {
                return Err(PyNotImplementedError::new_err(ERRMSG_STEPSIZE_TOO_LARGE));
            }

            // shortcut for clearing high bits
            if indices.stop == len as _ && !set {
                self.0.clear_high_bits(indices.start as _);
            }

            // here the operations rely on the And and Or ops.
            // they can be optimized if UBig implements more bit operations.
            if set {
                let ones = indices.stop - indices.start;
                let mask = UBig::ones(ones as _) << (indices.start as usize);
                self.0 |= mask;
            } else {
                let mask_lo = UBig::ones(indices.stop as _) - UBig::ones(indices.start as _);
                let mask = UBig::ones(len as _) - mask_lo;
                self.0 &= mask;
            }
            Ok(())
        } else {
            Err(PyTypeError::new_err(ERRMSG_WRONG_INDEX_TYPE))
        }
    }
    fn __delitem__(&mut self, index: &PyAny) -> PyResult<()> {
        fn remove_bits_in_middle(u: &mut UBig, start: usize, end: usize) {
            let (mut left, right) = core::mem::take(u).split_bits(end);
            left.clear_high_bits(end - start);
            *u = (right << start) | left;
        }

        if let Ok(i) = <isize as FromPyObject>::extract(index) {
            let i = parse_signed_index(i, self.0.bit_len(), true)
                .ok_or(PyIndexError::new_err(ERRMSG_UBIG_BITS_OOR))?;
            remove_bits_in_middle(&mut self.0, i, i + 1);
            Ok(())
        } else if let Ok(range) = index.downcast::<PySlice>() {
            let len = self
                .0
                .bit_len()
                .try_into()
                .map_err(|_| PyNotImplementedError::new_err(ERRMSG_LENGTH_TOO_LARGE))?;
            let indices = range.indices(len)?;
            if indices.step != 1 {
                return Err(PyNotImplementedError::new_err(ERRMSG_STEPSIZE_TOO_LARGE));
            }

            // shortcut for clearing high bits
            if indices.stop == len as _ {
                self.0.clear_high_bits(indices.start as _);
            } else if indices.start == 0 {
                self.0 >>= indices.stop as usize;
            } else {
                remove_bits_in_middle(&mut self.0, indices.start as _, indices.stop as _);
            }
            Ok(())
        } else {
            Err(PyTypeError::new_err(ERRMSG_WRONG_INDEX_TYPE))
        }
    }

    /********** interop **********/

    /// Get the underlying words representing this integer
    fn to_words(&self) -> PyWords {
        PyWords(self.0.as_words().to_vec())
    }
    /// Create an integer from a list of words
    #[staticmethod]
    fn from_words(words: &PyWords) -> Self {
        // TODO: accept a list of integers, using Vec<Word>::extract
        UPy(UBig::from_words(&words.0))
    }
    fn to_bytes(&self, py: Python) -> PyObject {
        PyBytes::new(py, &self.0.to_le_bytes()).into()
    }
    #[staticmethod]
    fn from_bytes(bytes: &PyBytes) -> Self {
        todo!()
    }
}

impl IPy {
    // Conversion from python integer object, without type checking.
    //
    // The most efficient way here is to use ffi::_PyLong_AsByteArray.
    // However, the conversion should not performed frequently, so the stable
    // API `to_bytes` is preferred here.
    fn wrap(ob: &PyAny) -> PyResult<Self> {
        let (v, overflow) = py_to_long_or_big(ob)?;
        if !overflow {
            Ok(IPy(IBig::from(v)))
        } else {
            let py = ob.py();
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
        if ob.is_instance_of::<PyLong>() {
            if radix.is_some() {
                Err(PyTypeError::new_err("can't convert non-string with explicit base"))
            } else {
                Self::wrap(ob)
            }
        } else if let Ok(s) = ob.extract() {
            let n = if let Some(r) = radix {
                IBig::from_str_radix(s, r)
            } else {
                IBig::from_str_with_radix_prefix(s).map(|v| v.0)
            };
            Ok(IPy(n.map_err(crate::utils::parse_error_to_py)?))
        } else {
            Err(PyTypeError::new_err(
                "only Python integers can be automatically converted to an IBig instance",
            ))
        }
    }
    fn __repr__(&self) -> String {
        format!("<IBig {:#?}>", self.0)
    }
    fn __str__(&self) -> String {
        format!("{}", self.0)
    }
    fn __format__(&self) {
        todo!()
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

    // use as a bit vector with very limited capabilities
    fn __len__(&self) -> usize {
        self.0.bit_len()
    }
    fn __getitem__(&self, i: usize) -> bool {
        self.0.bit(i)
    }
}
