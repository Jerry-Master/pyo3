// Copyright (c) 2017-present PyO3 Project and Contributors

//! Python Mapping Interface
//! Trait and support implementation for implementing mapping support

use std::os::raw::c_int;

use ffi;
use err::{PyErr, PyResult};
use python::{Python, PythonObject, PyDrop};
use objects::{exc, PyObject};
use callback::{PyObjectCallbackConverter, LenResultConverter, UnitCallbackConverter};
use conversion::{ToPyObject, FromPyObject};


/// Mapping interface
#[allow(unused_variables)]
pub trait PyMappingProtocol: PythonObject {
    fn __len__(&self, py: Python) -> Self::Result
        where Self: PyMappingLenProtocol
    { unimplemented!() }

    fn __getitem__(&self, py: Python, key: Self::Key) -> Self::Result
        where Self: PyMappingGetItemProtocol
    { unimplemented!() }

    fn __setitem__(&self, py: Python, key: Self::Key, value: Self::Value) -> Self::Result
        where Self: PyMappingSetItemProtocol
    { unimplemented!() }

    fn __delitem__(&self, py: Python, key: Self::Key) -> Self::Result
        where Self: PyMappingDelItemProtocol
    { unimplemented!() }

}


// The following are a bunch of marker traits used to detect
// the existance of a slotted method.

pub trait PyMappingLenProtocol: PyMappingProtocol {
    type Result: Into<PyResult<usize>>;
}

pub trait PyMappingGetItemProtocol: PyMappingProtocol {
    type Key: for<'a> FromPyObject<'a>;
    type Success: ToPyObject;
    type Result: Into<PyResult<Self::Success>>;
}

pub trait PyMappingSetItemProtocol: PyMappingProtocol {
    type Key: for<'a> FromPyObject<'a>;
    type Value: for<'a> FromPyObject<'a>;
    type Success: ToPyObject;
    type Result: Into<PyResult<()>>;
}

pub trait PyMappingDelItemProtocol: PyMappingProtocol {
    type Key: for<'a> FromPyObject<'a>;
    type Success: ToPyObject;
    type Result: Into<PyResult<()>>;
}

#[doc(hidden)]
pub trait PyMappingProtocolImpl {
    fn tp_as_mapping() -> Option<ffi::PyMappingMethods>;
}

impl<T> PyMappingProtocolImpl for T {
    #[inline]
    default fn tp_as_mapping() -> Option<ffi::PyMappingMethods> {
        None
    }
}

impl<T> PyMappingProtocolImpl for T where T: PyMappingProtocol {
    #[inline]
    fn tp_as_mapping() -> Option<ffi::PyMappingMethods> {
        let mut f = Self::mp_ass_subscript();

        if let Some(df) = Self::mp_del_subscript() {
            f = Some(df)
        }

        Some(ffi::PyMappingMethods {
            mp_length: Self::mp_length(),
            mp_subscript: Self::mp_subscript(),
            mp_ass_subscript: f,
        })
    }
}

trait PyMappingLenProtocolImpl {
    fn mp_length() -> Option<ffi::lenfunc>;
}

impl<T> PyMappingLenProtocolImpl for T
    where T: PyMappingProtocol
{
    #[inline]
    default fn mp_length() -> Option<ffi::lenfunc> {
        None
    }
}

impl<T> PyMappingLenProtocolImpl for T
    where T: PyMappingLenProtocol
{
    #[inline]
    fn mp_length() -> Option<ffi::lenfunc> {
        py_len_func_!(PyMappingLenProtocol, T::__len__, LenResultConverter)
    }
}

trait PyMappingGetItemProtocolImpl {
    fn mp_subscript() -> Option<ffi::binaryfunc>;
}

impl<T> PyMappingGetItemProtocolImpl for T
    where T: PyMappingProtocol
{
    #[inline]
    default fn mp_subscript() -> Option<ffi::binaryfunc> {
        None
    }
}

impl<T> PyMappingGetItemProtocolImpl for T
    where T: PyMappingGetItemProtocol
{
    #[inline]
    fn mp_subscript() -> Option<ffi::binaryfunc> {
        py_binary_func_!(PyMappingGetItemProtocol, T::__getitem__, PyObjectCallbackConverter)
    }
}

trait PyMappingSetItemProtocolImpl {
    fn mp_ass_subscript() -> Option<ffi::objobjargproc>;
}

impl<T> PyMappingSetItemProtocolImpl for T
    where T: PyMappingProtocol
{
    #[inline]
    default fn mp_ass_subscript() -> Option<ffi::objobjargproc> {
        None
    }
}

impl<T> PyMappingSetItemProtocolImpl for T
    where T: PyMappingSetItemProtocol
{
    #[inline]
    fn mp_ass_subscript() -> Option<ffi::objobjargproc> {
        unsafe extern "C" fn wrap<T>(slf: *mut ffi::PyObject,
                                     key: *mut ffi::PyObject,
                                     value: *mut ffi::PyObject) -> c_int
            where T: PyMappingSetItemProtocol
        {
            const LOCATION: &'static str = "T.__setitem__()";
            ::callback::handle_callback(LOCATION, UnitCallbackConverter, |py| {
                let slf = PyObject::from_borrowed_ptr(py, slf).unchecked_cast_into::<T>();
                let key = PyObject::from_borrowed_ptr(py, key);

                let ret = match key.extract(py) {
                    Ok(key) =>
                        if value.is_null() {
                            Err(PyErr::new::<exc::NotImplementedError, _>(
                                py, format!("Subscript deletion not supported by {:?}",
                                            stringify!(T))))
                        } else {
                            let value = PyObject::from_borrowed_ptr(py, value);
                            let ret = match value.extract(py) {
                                Ok(value) => slf.__setitem__(py, key, value).into(),
                                Err(e) => Err(e),
                            };
                            PyDrop::release_ref(value, py);
                            ret
                        },
                    Err(e) => Err(e),
                };

                PyDrop::release_ref(key, py);
                PyDrop::release_ref(slf, py);
                ret
            })
        }
        Some(wrap::<T>)
    }
}


trait PyMappingDelItemProtocolImpl {
    fn mp_del_subscript() -> Option<ffi::objobjargproc>;
}

impl<T> PyMappingDelItemProtocolImpl for T
    where T: PyMappingProtocol
{
    #[inline]
    default fn mp_del_subscript() -> Option<ffi::objobjargproc> {
        None
    }
}

impl<T> PyMappingDelItemProtocolImpl for T
    where T: PyMappingDelItemProtocol
{
    #[inline]
    default fn mp_del_subscript() -> Option<ffi::objobjargproc> {
        unsafe extern "C" fn wrap<T>(slf: *mut ffi::PyObject,
                                     key: *mut ffi::PyObject,
                                     value: *mut ffi::PyObject) -> c_int
            where T: PyMappingDelItemProtocol
        {
            const LOCATION: &'static str = "T.__detitem__()";
            ::callback::handle_callback(LOCATION, UnitCallbackConverter, |py| {
                let slf = PyObject::from_borrowed_ptr(py, slf).unchecked_cast_into::<T>();
                let key = PyObject::from_borrowed_ptr(py, key);

                let ret = match key.extract(py) {
                    Ok(key) =>
                        if value.is_null() {
                            slf.__delitem__(py, key).into()
                        } else {
                            Err(PyErr::new::<exc::NotImplementedError, _>(
                                py, format!("Subscript assignment not supported by {:?}",
                                            stringify!(T))))
                        },
                    Err(e) => Err(e),
                };

                PyDrop::release_ref(key, py);
                PyDrop::release_ref(slf, py);
                ret
            })
        }
        Some(wrap::<T>)
    }
}

impl<T> PyMappingDelItemProtocolImpl for T
    where T: PyMappingSetItemProtocol + PyMappingDelItemProtocol
{
    #[inline]
    fn mp_del_subscript() -> Option<ffi::objobjargproc> {
        unsafe extern "C" fn wrap<T>(slf: *mut ffi::PyObject,
                                     key: *mut ffi::PyObject,
                                     value: *mut ffi::PyObject) -> c_int
            where T: PyMappingSetItemProtocol + PyMappingDelItemProtocol
        {
            const LOCATION: &'static str = "T.__set/del_item__()";
            ::callback::handle_callback(LOCATION, UnitCallbackConverter, |py| {
                let slf = PyObject::from_borrowed_ptr(py, slf).unchecked_cast_into::<T>();
                let key = PyObject::from_borrowed_ptr(py, key);

                let ret = if value.is_null() {
                    match key.extract(py) {
                        Ok(key) => slf.__delitem__(py, key).into(),
                        Err(e) => Err(e)
                    }
                } else {
                    match key.extract(py) {
                        Ok(key) => {
                            let value = PyObject::from_borrowed_ptr(py, value);
                            let ret = match value.extract(py) {
                                Ok(value) => slf.__setitem__(py, key, value).into(),
                                Err(e) => Err(e),
                            };
                            PyDrop::release_ref(value, py);
                            ret
                        },
                        Err(e) => Err(e),
                    }
                };

                PyDrop::release_ref(key, py);
                PyDrop::release_ref(slf, py);
                ret
            })
        }
        Some(wrap::<T>)
    }
}
