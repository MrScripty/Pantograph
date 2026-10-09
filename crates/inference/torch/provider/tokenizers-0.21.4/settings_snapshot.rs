// Pantograph modification, Apache-2.0. CPython 3.12.3 primitive settings only.
// Continuous native GIL custody; no Python evaluation, opaque getters, serializer,
// user comparison/hash, arbitrary decref, allow_threads or mutable input copies.
use super::{sorted, Meter, Sink, COPY, ENTRIES, STRING, TEXT};
use pyo3::prelude::*;
use pyo3::types::{PyBool, PyBytes, PyDict, PyFloat, PyInt, PyList, PyString, PyTuple};
use std::{borrow::Cow, ffi::CStr};

const PREFIX: &[u8] = b"pantograph-cpython-3.12.3-tokenizer-settings.v1\0";
const STORAGE: usize = 256 * 1024;
const DEPTH: usize = 8;

struct State<'a> {
    sink: Sink<'a>,
    nodes: usize,
}

impl State<'_> {
    fn node(&mut self, depth: usize) -> Result<(), ()> {
        self.sink.meter.work(1)?;
        self.nodes = self.nodes.checked_add(1).ok_or(())?;
        if self.nodes > ENTRIES || depth > DEPTH {
            return Err(());
        }
        Ok(())
    }

    fn storage(&mut self, value: &Bound<'_, PyAny>, len: usize) -> Result<(), ()> {
        // Only exact immutable built-in type implementations reach this call.
        // CPython3.12.3 __sizeof__ is constant-work for these container types.
        // Their descriptor performs C-only work; no caller override can run.
        if len > ENTRIES {
            return Err(());
        }
        let bytes = value
            .call_method0("__sizeof__")
            .map_err(|_| ())?
            .extract::<usize>()
            .map_err(|_| ())?;
        if bytes > STORAGE {
            return Err(());
        }
        // CPython split dictionaries omit shared keys from __sizeof__.
        // max30 shared keys + indices/header fit inside this 1024-byte allowance.
        // Charge both the occupied traversal and conservative storage scan.
        self.sink.meter.work(2 * (bytes + 1024))
    }

    fn text<'a>(&mut self, value: &'a Bound<'_, PyString>) -> Result<Cow<'a, str>, ()> {
        // GetLength reads existing Unicode metadata before UTF8 cache creation.
        let length = unsafe { pyo3::ffi::PyUnicode_GetLength(value.as_ptr()) };
        let length = usize::try_from(length).map_err(|_| ())?;
        if length > STRING {
            return Err(());
        }
        let maximum = length.checked_mul(4).ok_or(())?;
        self.sink.meter.text = self.sink.meter.text.checked_add(maximum).ok_or(())?;
        if self.sink.meter.text > TEXT {
            return Err(());
        }
        // Reserve worst-case UTF8 cache writes before any conversion. This
        // deliberately narrows aggregate admission even for ASCII strings.
        // The abi3-py39 binding's to_cow path creates UTF8 PyBytes and a Rust
        // owned string. Reserve BOTH writes, even on a cached/borrowed path.
        self.sink.meter.copy(2 * maximum)?;
        let text = value.to_cow().map_err(|_| ())?;
        if text.len() > STRING {
            return Err(());
        }
        Ok(text)
    }

    fn value(&mut self, value: &Bound<'_, PyAny>, depth: usize, filter: u8) -> Result<(), ()> {
        self.node(depth)?;
        let py = value.py();
        let ty = value.get_type();
        if value.is_none() {
            return self.sink.raw(b"n");
        }
        if ty.is(&py.get_type::<PyBool>()) {
            return self.sink.raw(if value.extract::<bool>().map_err(|_| ())? {
                b"t"
            } else {
                b"f"
            });
        }
        if ty.is(&py.get_type::<PyInt>()) {
            // Exact ints only; constant-size preflight before integer conversion.
            self.storage(value, 1)?;
            let bytes = value
                .call_method0("__sizeof__")
                .map_err(|_| ())?
                .extract::<usize>()
                .map_err(|_| ())?;
            if bytes > 64 {
                return Err(());
            }
            let number = value.extract::<i128>().map_err(|_| ())?;
            if !(-(1_i128 << 64)..(1_i128 << 64)).contains(&number) {
                return Err(());
            }
            self.sink.raw(b"i")?;
            return self.sink.raw(&number.to_le_bytes());
        }
        if ty.is(&py.get_type::<PyFloat>()) {
            let number = value.extract::<f64>().map_err(|_| ())?;
            if !number.is_finite() {
                return Err(());
            }
            self.sink.raw(b"r")?;
            return self.sink.raw(&number.to_bits().to_le_bytes());
        }
        if ty.is(&py.get_type::<PyString>()) {
            let text = self.text(value.downcast::<PyString>().map_err(|_| ())?)?;
            self.sink.raw(b"s")?;
            return self.sink.text(&text);
        }
        if ty.is(&py.get_type::<PyList>()) || ty.is(&py.get_type::<PyTuple>()) {
            let len = value.len().map_err(|_| ())?;
            self.storage(value, len)?;
            // List/tuple intentionally preserve the earlier settings equivalence.
            self.sink.raw(b"a")?;
            self.sink.number(len)?;
            for index in 0..len {
                // Exact built-in indexed borrowed access: no iterator callback.
                let child = if ty.is(&py.get_type::<PyList>()) {
                    value.downcast::<PyList>().map_err(|_| ())?.get_item(index)
                } else {
                    value.downcast::<PyTuple>().map_err(|_| ())?.get_item(index)
                }
                .map_err(|_| ())?;
                self.value(&child, depth + 1, 0)?;
            }
            return Ok(());
        }
        if !ty.is(&py.get_type::<PyDict>()) {
            // Including native AddedToken, configs, dtype/device, sets, subclasses.
            return Err(());
        }
        let dict = value.downcast::<PyDict>().map_err(|_| ())?;
        self.storage(value, dict.len())?;
        // Keep only borrowed references; the original dictionary remains rooted
        // by the argument graph under GIL. Cloning a Bound increments a refcount
        // without allocating or invoking an arbitrary object's finalizer.
        self.sink.meter.copy(
            dict.len() * std::mem::size_of::<(Bound<'_, PyAny>, Bound<'_, PyAny>, Vec<u8>, u8)>(),
        )?;
        let mut entries = Vec::new();
        entries.try_reserve_exact(dict.len()).map_err(|_| ())?;
        for (key, child) in dict.iter() {
            let start = self.sink.bytes.len();
            let key_type = key.get_type();
            if !(key_type.is(&py.get_type::<PyString>()) || key_type.is(&py.get_type::<PyInt>())) {
                return Err(());
            }
            // Qualify every key before locator filtering or hashing/equality.
            self.value(&key, depth + 1, 0)?;
            let key_size = self.sink.bytes.len() - start;
            self.sink.meter.copy(key_size)?;
            let mut encoded = Vec::new();
            encoded.try_reserve_exact(key_size).map_err(|_| ())?;
            encoded.extend_from_slice(&self.sink.bytes[start..]);
            self.sink.bytes.truncate(start);
            let name = if key_type.is(&py.get_type::<PyString>()) {
                Some(std::str::from_utf8(&encoded[9..]).map_err(|_| ())?)
            } else {
                None
            };
            let excluded = match (filter, name) {
                (1, Some("_tokenizer" | "name_or_path" | "deprecation_warnings")) => true,
                (2, Some("name_or_path" | "tokenizer_file" | "_commit_hash")) => true,
                _ => false,
            };
            if !excluded {
                let nested = if filter == 1 && name == Some("init_kwargs") {
                    2
                } else {
                    0
                };
                if nested == 2 && !child.get_type().is(&py.get_type::<PyDict>()) {
                    return Err(());
                }
                entries.push((key, child, encoded, nested));
            }
        }
        // Sort indices, never Python objects/comparisons or caller hash methods.
        self.sink
            .meter
            .copy(entries.len() * std::mem::size_of::<usize>())?;
        let mut order = Vec::new();
        order.try_reserve_exact(entries.len()).map_err(|_| ())?;
        order.extend(0..entries.len());
        sorted(&mut order, self.sink.meter, |a, b, meter| {
            let left = &entries[a].2;
            let right = &entries[b].2;
            meter.work(left.len().min(right.len()) + 1)?;
            Ok(left.cmp(right))
        })?;
        self.sink.raw(b"d")?;
        self.sink.number(order.len())?;
        for index in order {
            self.sink.raw(&entries[index].2)?;
            self.value(&entries[index].1, depth + 1, entries[index].3)?;
        }
        Ok(())
    }
}

pub(crate) fn inspect<'py>(
    py: Python<'py>,
    fields: &Bound<'py, PyAny>,
) -> PyResult<(bool, Bound<'py, PyBytes>, usize, usize)> {
    let mut meter = Meter::default();
    let result = (|| {
        // This storage/GC/Unicode cost model is source-qualified for CPython
        // 3.12.3 with a conventional GIL, plus the existing pinned Rust target.
        let version = unsafe { CStr::from_ptr(pyo3::ffi::Py_GetVersion()) }.to_bytes();
        if !cfg!(pantograph_snapshot_qualified)
            || !version.starts_with(b"3.12.3 ")
            || !fields.get_type().is(&py.get_type::<PyDict>())
        {
            return Err(());
        }
        let mut bytes = Vec::new();
        bytes
            .try_reserve_exact(TEXT + 32 * ENTRIES + 1024)
            .map_err(|_| ())?;
        let mut state = State {
            sink: Sink {
                bytes,
                meter: &mut meter,
            },
            nodes: 0,
        };
        state.sink.raw(PREFIX)?;
        state.value(fields, 0, 1)?;
        Ok(state.sink.bytes)
    })();
    let (accepted, payload) = match result {
        Ok(bytes) if meter.copy(2 * bytes.len()).is_ok() => (true, bytes),
        _ => (false, Vec::new()),
    };
    let bytes = PyBytes::new_with(py, payload.len(), |target| {
        target.copy_from_slice(&payload);
        Ok(())
    })?;
    debug_assert!(meter.copied <= COPY);
    Ok((accepted, bytes, meter.work, meter.copied))
}
