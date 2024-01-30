use std::collections::hash_map::DefaultHasher;
use std::hash::Hasher;
use std::vec::Vec;

use dashu_base::{BitTest, Signed};
use pyo3::exceptions::{PyIndexError, PyNotImplementedError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::PySlice;
use pyo3::{
    exceptions::{PyOverflowError, PyTypeError},
    types::{PyBytes, PyLong},
};

use crate::{
    convert::{
        convert_from_ibig, convert_from_ubig, parse_error_to_py, parse_signed_index, parse_to_ibig,
        parse_to_long, parse_to_ubig,
    },
    types::{IPy, PyWords, UPy},
};
use dashu_int::{IBig, UBig, Word};
use num_order::NumHash;

// error messages
const ERRMSG_LENGTH_TOO_LARGE: &'static str = "the integer has too many bits for indexing";
const ERRMSG_STEPSIZE_TOO_LARGE: &'static str =
    "bit slicing with step size larger than 1 is not supported yet";
const ERRMSG_UBIG_WRONG_SRC_TYPE: &'static str =
    "only integers or strings can be used to construct a UBig instance";
const ERRMSG_IBIG_WRONG_SRC_TYPE: &'static str =
    "only integers or strings can be used to construct an IBig instance";
const ERRMSG_FROM_WORDS_WRONG_TYPE: &'static str =
    "only list of integers or Words instance can be used in UBig.from_words()";
const ERRMSG_WRONG_ENDIANNESS: &'static str = "byteorder must be either 'little' or 'big'";
const ERRMSG_NEGATIVE_TO_UNSIGNED: &'static str = "can't convert negative int to unsigned";
const ERRMSG_INT_WITH_RADIX: &'static str = "can't convert non-string with explicit base";
const ERRMSG_WRONG_INDEX_TYPE: &'static str = "indices must be integers or slices";
const ERRMSG_UBIG_FROM_NEG: &'static str = "can't convert negative int to unsigned";
const ERRMSG_UBIG_BITS_OOR: &'static str = "bits index out of range";

#[pymethods]
impl UPy {
    #[new]
    fn __new__(ob: &PyAny, radix: Option<u32>) -> PyResult<Self> {
        if ob.is_instance_of::<PyLong>() {
            // create from int
            if radix.is_some() {
                return Err(PyTypeError::new_err(ERRMSG_INT_WITH_RADIX));
            }

            let (v, overflow) = parse_to_long(ob)?;
            if !overflow {
                if let Ok(n) = u64::try_from(v) {
                    Ok(UPy(UBig::from(n)))
                } else {
                    Err(PyOverflowError::new_err(ERRMSG_UBIG_FROM_NEG))
                }
            } else {
                Ok(UPy(parse_to_ubig(ob)?))
            }
        } else if let Ok(s) = ob.extract() {
            // create from string
            let n = if let Some(r) = radix {
                UBig::from_str_radix(s, r)
            } else {
                UBig::from_str_with_radix_prefix(s).map(|v| v.0)
            };
            Ok(UPy(n.map_err(parse_error_to_py)?))
        } else if let Ok(obj) = <PyRef<Self> as FromPyObject>::extract(ob) {
            Ok(UPy(obj.0.clone()))
        } else {
            Err(PyTypeError::new_err(ERRMSG_UBIG_WRONG_SRC_TYPE))
        }
    }
    fn unwrap(&self, py: Python) -> PyResult<PyObject> {
        convert_from_ubig(&self.0, py)
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

    fn __int__(&self, py: Python) -> PyResult<PyObject> {
        convert_from_ubig(&self.0, py)
    }
    /// Get the underlying words representing this integer
    fn to_words(&self) -> PyWords {
        PyWords(self.0.as_words().to_vec())
    }
    /// Create an integer from a list of words
    #[staticmethod]
    fn from_words(ob: &PyAny) -> PyResult<Self> {
        if let Ok(vec) = <Vec<Word> as FromPyObject>::extract(ob) {
            Ok(UPy(UBig::from_words(&vec)))
        } else if let Ok(words) = <PyRef<PyWords> as FromPyObject>::extract(ob) {
            Ok(UPy(UBig::from_words(&words.0)))
        } else {
            Err(PyTypeError::new_err(ERRMSG_FROM_WORDS_WRONG_TYPE))
        }
    }
    /// Convert the integer to bytes, like int.to_bytes().
    fn to_bytes(&self, byteorder: Option<&str>, py: Python) -> PyResult<PyObject> {
        let byteorder = byteorder.unwrap_or(&"little");
        let bytes = match byteorder {
            "little" => PyBytes::new(py, &self.0.to_le_bytes()),
            "big" => PyBytes::new(py, &self.0.to_be_bytes()),
            _ => {
                return Err(PyValueError::new_err(ERRMSG_WRONG_ENDIANNESS));
            }
        };
        Ok(bytes.into())
    }
    /// Create UBig from bytes, like int.from_bytes().
    #[staticmethod]
    fn from_bytes(bytes: &PyBytes, byteorder: Option<&str>) -> PyResult<Self> {
        let byteorder = byteorder.unwrap_or(&"little");
        let uint = match byteorder {
            "little" => UBig::from_le_bytes(bytes.as_bytes()),
            "big" => UBig::from_be_bytes(bytes.as_bytes()),
            _ => {
                return Err(PyValueError::new_err(ERRMSG_WRONG_ENDIANNESS));
            }
        };
        Ok(Self(uint))
    }
}

#[pymethods]
impl IPy {
    #[new]
    #[inline]
    fn __new__(ob: &PyAny, radix: Option<u32>) -> PyResult<Self> {
        if ob.is_instance_of::<PyLong>() {
            // create from int
            if radix.is_some() {
                return Err(PyTypeError::new_err(ERRMSG_INT_WITH_RADIX));
            }

            let (v, overflow) = parse_to_long(ob)?;
            if !overflow {
                Ok(IPy(IBig::from(v)))
            } else {
                Ok(IPy(parse_to_ibig(ob)?))
            }
        } else if let Ok(s) = ob.extract() {
            // create from string
            let n = if let Some(r) = radix {
                IBig::from_str_radix(s, r)
            } else {
                IBig::from_str_with_radix_prefix(s).map(|v| v.0)
            };
            Ok(IPy(n.map_err(parse_error_to_py)?))
        } else if let Ok(obj) = <PyRef<Self> as FromPyObject>::extract(ob) {
            Ok(IPy(obj.0.clone()))
        } else {
            Err(PyTypeError::new_err(ERRMSG_IBIG_WRONG_SRC_TYPE))
        }
    }
    fn unwrap(&self, py: Python) -> PyResult<PyObject> {
        convert_from_ibig(&self.0, py)
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

    // use as a bit vector with very limited capabilities
    fn __len__(&self) -> usize {
        self.0.bit_len()
    }
    fn __getitem__(&self, i: usize) -> bool {
        self.0.bit(i)
    }

    /********** interop **********/

    fn __int__(&self, py: Python) -> PyResult<PyObject> {
        convert_from_ibig(&self.0, py)
    }
    /// Convert the integer to bytes, like int.to_bytes().
    fn to_bytes(
        &self,
        byteorder: Option<&str>,
        signed: Option<bool>,
        py: Python,
    ) -> PyResult<PyObject> {
        let signed = signed.unwrap_or(false);
        if !signed && self.0.is_negative() {
            return Err(PyOverflowError::new_err(ERRMSG_NEGATIVE_TO_UNSIGNED));
        }

        let byteorder = byteorder.unwrap_or(&"little");
        let bytes = match byteorder {
            "little" => PyBytes::new(py, &self.0.to_le_bytes()),
            "big" => PyBytes::new(py, &self.0.to_be_bytes()),
            _ => {
                return Err(PyValueError::new_err(ERRMSG_WRONG_ENDIANNESS));
            }
        };
        Ok(bytes.into())
    }
    /// Create IBig from bytes, like int.from_bytes().
    #[staticmethod]
    fn from_bytes(
        bytes: &PyBytes,
        byteorder: Option<&str>,
        signed: Option<bool>,
    ) -> PyResult<Self> {
        let byteorder = byteorder.unwrap_or(&"little");
        let signed = signed.unwrap_or(false);
        let int = match byteorder {
            "little" => match signed {
                false => UBig::from_le_bytes(bytes.as_bytes()).into(),
                true => IBig::from_le_bytes(bytes.as_bytes()),
            },
            "big" => match signed {
                false => UBig::from_be_bytes(bytes.as_bytes()).into(),
                true => IBig::from_be_bytes(bytes.as_bytes()),
            },
            _ => {
                return Err(PyValueError::new_err(ERRMSG_WRONG_ENDIANNESS));
            }
        };
        Ok(Self(int))
    }
}
