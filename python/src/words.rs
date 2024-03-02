use crate::types::PyWords;

use pyo3::{
    exceptions::{PyIndexError, PyTypeError, PyValueError},
    prelude::*,
    types::PySlice,
};

use dashu_int::Word;

const ERRMSG_WORDS_WRONG_SRC_TYPE: &str =
    "only a list of word-length integers can be used to construct a Words instance";
const ERRMSG_WORDS_OOR: &str = "words index out of range";
const ERRMSG_WORDS_INVALID_INDEX: &str = "words indices must be integers or slices";
const ERRMSG_WORDS_UNMATCH_INDEX: &str =
    "attempt to assign sequence to an extended slice with different length";
const ERRMSG_WORDS_INVALID_VALUE: &str = "words can only contain word-length integers";

#[pymethods]
impl PyWords {
    #[new]
    fn __new__(ob: &PyAny) -> PyResult<Self> {
        if let Ok(list) = <Vec<Word> as FromPyObject>::extract(ob) {
            Ok(PyWords(list))
        } else if let Ok(obj) = <PyRef<Self> as FromPyObject>::extract(ob) {
            Ok(PyWords(obj.0.clone()))
        } else {
            Err(PyTypeError::new_err(ERRMSG_WORDS_WRONG_SRC_TYPE))
        }
    }
    fn __repr__(&self) -> String {
        format!("<Words with {} items>", self.0.len())
    }
    fn __str__(&self) -> String {
        format!("{:?}", self.0)
    }
    fn __len__(&self) -> usize {
        self.0.len()
    }
    fn __getitem__(&self, index: &PyAny) -> PyResult<PyObject> {
        let py = index.py();
        if let Ok(n) = <isize as FromPyObject>::extract(index) {
            // parse negative index
            let n = if n < 0 {
                (self.0.len() as isize + n) as usize
            } else {
                n as usize
            };

            // get value
            let word = self
                .0
                .get(n)
                .copied()
                .ok_or(PyIndexError::new_err(ERRMSG_WORDS_OOR))?;
            Ok(word.into_py(py))
        } else if let Ok(slice) = index.downcast::<PySlice>() {
            let indices = slice.indices(self.0.len() as _)?;

            let new_vec = if indices.step >= 0 {
                let (span, skip) = (indices.stop - indices.start, indices.start);
                if span > 0 {
                    self.0
                        .iter()
                        .skip(skip as _)
                        .take(span as _)
                        .step_by(indices.step as _)
                        .copied()
                        .collect()
                } else {
                    Vec::new()
                }
            } else {
                let (span, skip) =
                    (indices.start - indices.stop, self.0.len() as isize - 1 - indices.start);
                if span > 0 {
                    self.0
                        .iter()
                        .rev()
                        .skip(skip as _)
                        .take(span as _)
                        .step_by(-indices.step as _)
                        .copied()
                        .collect()
                } else {
                    Vec::new()
                }
            };
            Ok(Self(new_vec).into_py(py))
        } else {
            Err(PyTypeError::new_err(ERRMSG_WORDS_INVALID_INDEX))
        }
    }
    fn __setitem__(&mut self, index: &PyAny, value: &PyAny) -> PyResult<()> {
        if let Ok(n) = <isize as FromPyObject>::extract(index) {
            let value: Word = value.extract()?;

            // parse negative index
            let n = if n < 0 {
                (self.0.len() as isize + n) as usize
            } else {
                n as usize
            };

            // assign value
            if let Some(w) = self.0.get_mut(n) {
                *w = value;
                Ok(())
            } else {
                Err(PyIndexError::new_err(ERRMSG_WORDS_OOR))
            }
        } else if let Ok(slice) = index.downcast::<PySlice>() {
            // parse inputs
            let indices = slice.indices(self.0.len() as _)?;
            let value: Vec<Word> = if let Ok(v) = <Vec<Word> as FromPyObject>::extract(value) {
                v
            } else if let Ok(v) = <PyRef<Self> as FromPyObject>::extract(value) {
                v.0.clone() // FIXME: how to prevent copy here?
            } else {
                return Err(PyTypeError::new_err(ERRMSG_WORDS_INVALID_VALUE));
            };

            // check that the indices and the values have the same length
            if indices.slicelength as usize != value.len() {
                return Err(PyValueError::new_err(ERRMSG_WORDS_UNMATCH_INDEX));
            }

            // assign the values
            if indices.step >= 0 {
                let (span, skip) = (indices.stop - indices.start, indices.start);
                if span > 0 {
                    self.0
                        .iter_mut()
                        .skip(skip as _)
                        .take(span as _)
                        .step_by(indices.step as _)
                        .zip(value.iter())
                        .for_each(|(l, r)| *l = *r);
                }
            } else {
                let (span, skip) =
                    (indices.start - indices.stop, self.0.len() as isize - 1 - indices.start);
                if span > 0 {
                    self.0
                        .iter_mut()
                        .rev()
                        .skip(skip as _)
                        .take(span as _)
                        .step_by(-indices.step as _)
                        .zip(value.iter())
                        .for_each(|(l, r)| *l = *r);
                }
            };
            Ok(())
        } else {
            Err(PyTypeError::new_err(ERRMSG_WORDS_INVALID_INDEX))
        }
    }
    fn __delitem__(&mut self, index: &PyAny) -> PyResult<()> {
        if let Ok(n) = <isize as FromPyObject>::extract(index) {
            // parse negative index
            let n = if n < 0 {
                (self.0.len() as isize + n) as usize
            } else {
                n as usize
            };

            // remove value
            if n < self.0.len() {
                self.0.remove(n);
                Ok(())
            } else {
                Err(PyIndexError::new_err(ERRMSG_WORDS_OOR))
            }
        } else if let Ok(slice) = index.downcast::<PySlice>() {
            let indices = slice.indices(self.0.len() as _)?;

            if indices.step >= 0 {
                let (span, skip) = (indices.stop - indices.start, indices.start);
                if span > 0 {
                    let (span, skip, step) = (span as usize, skip as usize, indices.step as usize);
                    self.0 = core::mem::take(&mut self.0)
                        .into_iter()
                        .enumerate()
                        .filter_map(|(i, v)| {
                            let in_slice = i >= skip && i < (skip + span) && (i - skip) % step == 0;
                            (!in_slice).then(|| v)
                        })
                        .collect();
                }
            } else {
                let (span, skip) =
                    (indices.start - indices.stop, self.0.len() as isize - 1 - indices.start);
                if span > 0 {
                    let (span, skip, step) = (span as usize, skip as usize, -indices.step as usize);
                    self.0 = core::mem::take(&mut self.0)
                        .into_iter()
                        .rev()
                        .enumerate()
                        .filter_map(|(i, v)| {
                            let in_slice = i >= skip && i < (skip + span) && (i - skip) % step == 0;
                            (!in_slice).then(|| v)
                        })
                        .rev()
                        .collect();
                }
            };
            Ok(())
        } else {
            Err(PyTypeError::new_err(ERRMSG_WORDS_INVALID_INDEX))
        }
    }

    fn __add__(&self, other: &PyAny) -> PyResult<Self> {
        let mut out = self.0.clone();
        if let Ok(list) = <Vec<Word> as FromPyObject>::extract(other) {
            out.extend(list);
        } else if let Ok(obj) = <PyRef<Self> as FromPyObject>::extract(other) {
            out.extend(obj.0.iter());
        } else {
            return Err(PyTypeError::new_err(ERRMSG_WORDS_WRONG_SRC_TYPE));
        }
        Ok(Self(out))
    }
    fn __mul__(&self, count: isize) -> Self {
        if count <= 0 {
            Self(Vec::new())
        } else {
            let mut out = Vec::with_capacity(count as usize * self.0.len());
            for _ in 0..count {
                out.extend(self.0.iter());
            }
            Self(out)
        }
    }
}
