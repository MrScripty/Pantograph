// Pantograph modification to Tokenizers 0.21.4, Apache-2.0.
// This module is installed by prepare.py into the pinned provider binding.
// No Serde, vocabulary clone, native getter, Python callback, or blocking lock.
use crate::pre_tokenizers::{PyPreTokenizerTypeWrapper, PyPreTokenizerWrapper};
use crate::tokenizer::PyTokenizer;
use pyo3::prelude::*;
use pyo3::types::PyBytes;
use std::cmp::Ordering;
use tk::models::ModelWrapper;
use tk::pre_tokenizers::PreTokenizerWrapper;
use tk::tokenizer::{
    AddedToken, PaddingDirection, PaddingStrategy, TruncationDirection, TruncationStrategy,
};

const ENTRIES: usize = 4096;
const STRING: usize = 16 * 1024;
const TEXT: usize = 64 * 1024;
const COPY: usize = 4 * 1024 * 1024;
// Each of five maps/sets can scan <= 2*(capacity+1) buckets. Capacity is
// checked before iteration. Heapsort compares at most 4*n*(log2(n)+1)
// times, each conservatively charged for the full shorter string.
const WORK: u64 = 5 * 4 * 4096 * 14 * (16384 + 1) + 32 * (4 * 1024 * 1024);
const PREFIX: &[u8] = b"pantograph-tokenizers-0.21.4-wordlevel-snapshot.v1\0";

#[derive(Default)]
struct Meter {
    work: usize,
    copied: usize,
    text: usize,
}
impl Meter {
    fn work(&mut self, n: usize) -> Result<(), ()> {
        let next = self.work.checked_add(n).ok_or(())?;
        if next as u64 > WORK {
            return Err(());
        }
        self.work = next;
        Ok(())
    }
    fn copy(&mut self, n: usize) -> Result<(), ()> {
        let next = self.copied.checked_add(n).ok_or(())?;
        if next > COPY {
            return Err(());
        }
        self.copied = next;
        self.work(n)
    }
    fn string(&mut self, value: &str) -> Result<(), ()> {
        self.work(1)?;
        if value.len() > STRING {
            return Err(());
        }
        self.text = self.text.checked_add(value.len()).ok_or(())?;
        if self.text > TEXT {
            return Err(());
        }
        Ok(())
    }
    fn extent(&mut self, len: usize, capacity: usize) -> Result<(), ()> {
        self.work(1)?;
        if len > ENTRIES || capacity > 2 * ENTRIES {
            return Err(());
        }
        // AHashMap delegates to std HashMap; capacity excludes its spare
        // buckets. This envelope also covers empty tables and control groups.
        // Both preflight and reference collection/output traverse storage.
        self.work(2 * (2 * (capacity + 1) + 16))
    }
}

fn added(m: &mut Meter, token: &AddedToken) -> Result<(), ()> {
    m.string(&token.content)?;
    // Historic normalizer output is not exposed. Refuse instead of treating
    // current normalizer=None as proof about previously compiled patterns.
    if token.normalized {
        return Err(());
    }
    Ok(())
}

// Deterministic, allocation-free heapsort after a single reserved reference
// vector. Charge comparisons before reading strings, and all reference swaps.
fn sorted<T: Copy>(
    items: &mut [T],
    m: &mut Meter,
    compare: impl Fn(T, T, &mut Meter) -> Result<Ordering, ()>,
) -> Result<(), ()> {
    fn sift<T: Copy>(
        a: &mut [T],
        mut root: usize,
        end: usize,
        m: &mut Meter,
        cmp: &impl Fn(T, T, &mut Meter) -> Result<Ordering, ()>,
    ) -> Result<(), ()> {
        while 2 * root + 1 < end {
            let mut child = 2 * root + 1;
            if child + 1 < end && cmp(a[child], a[child + 1], m)? == Ordering::Less {
                child += 1;
            }
            if cmp(a[root], a[child], m)? != Ordering::Less {
                break;
            }
            m.copy(3 * std::mem::size_of::<T>())?;
            a.swap(root, child);
            root = child;
        }
        Ok(())
    }
    for root in (0..items.len() / 2).rev() {
        sift(items, root, items.len(), m, &compare)?;
    }
    for end in (1..items.len()).rev() {
        m.copy(3 * std::mem::size_of::<T>())?;
        items.swap(0, end);
        sift(items, 0, end, m, &compare)?;
    }
    Ok(())
}

fn refs<'a, T: 'a>(
    len: usize,
    iter: impl Iterator<Item = &'a T>,
    m: &mut Meter,
) -> Result<Vec<&'a T>, ()> {
    m.copy(len.checked_mul(std::mem::size_of::<&T>()).ok_or(())?)?;
    let mut out = Vec::new();
    out.try_reserve_exact(len).map_err(|_| ())?;
    out.extend(iter);
    Ok(out)
}

struct Sink<'a> {
    bytes: Vec<u8>,
    meter: &'a mut Meter,
}
impl Sink<'_> {
    fn raw(&mut self, bytes: &[u8]) -> Result<(), ()> {
        self.meter.copy(bytes.len())?;
        if self.bytes.len().checked_add(bytes.len()).ok_or(())? > self.bytes.capacity() {
            return Err(());
        }
        self.bytes.extend_from_slice(bytes);
        Ok(())
    }
    fn number(&mut self, n: usize) -> Result<(), ()> {
        self.raw(&(n as u64).to_le_bytes())
    }
    fn text(&mut self, s: &str) -> Result<(), ()> {
        self.number(s.len())?;
        self.raw(s.as_bytes())
    }
    fn token(&mut self, t: &AddedToken) -> Result<(), ()> {
        self.text(&t.content)?;
        self.raw(&[
            t.single_word as u8,
            t.lstrip as u8,
            t.rstrip as u8,
            t.normalized as u8,
            t.special as u8,
        ])
    }
}

fn snapshot(native: &PyTokenizer, m: &mut Meter) -> Result<Vec<u8>, ()> {
    if !cfg!(pantograph_snapshot_qualified) {
        return Err(());
    }
    let tokenizer = &native.tokenizer;
    if tokenizer.get_normalizer().is_some()
        || tokenizer.get_post_processor().is_some()
        || tokenizer.get_decoder().is_some()
    {
        return Err(());
    }
    let model = tokenizer.get_model().model.try_read().map_err(|_| ())?;
    let wordlevel = match &*model {
        ModelWrapper::WordLevel(w) => w,
        _ => return Err(()),
    };
    let pre = tokenizer.get_pre_tokenizer().ok_or(())?;
    let pre = match &pre.pretok {
        PyPreTokenizerTypeWrapper::Single(p) => p.try_read().map_err(|_| ())?,
        _ => return Err(()),
    };
    let whitespace = match &*pre {
        PyPreTokenizerWrapper::Wrapped(PreTokenizerWrapper::Whitespace(_)) => 0,
        PyPreTokenizerWrapper::Wrapped(PreTokenizerWrapper::WhitespaceSplit(_)) => 1,
        _ => return Err(()),
    };
    let (forward, reverse) = wordlevel.pantograph_borrowed_vocab();
    let vocabulary = tokenizer.get_added_vocabulary();
    let af = vocabulary.get_vocab();
    let ar = vocabulary.get_added_tokens_decoder();
    let (classic, special, set, ids, normalized_ids, patterns, normalized_patterns, trie_bytes) =
        vocabulary.pantograph_borrowed_history();
    for (len, cap) in [
        (forward.len(), forward.capacity()),
        (reverse.len(), reverse.capacity()),
        (af.len(), af.capacity()),
        (ar.len(), ar.capacity()),
        (set.len(), set.capacity()),
        (classic.len(), classic.capacity()),
        (special.len(), special.capacity()),
        (ids.len(), ids.capacity()),
        (normalized_ids.len(), normalized_ids.capacity()),
    ] {
        m.extent(len, cap)?;
    }
    if !normalized_ids.is_empty() || normalized_patterns != 0 || patterns != ids.len() {
        return Err(());
    }
    m.string(&wordlevel.unk_token)?;
    // Validate the complete borrowed prefix before ANY variable scratch or
    // payload copy. This conservative text sum includes duplicate storage.
    for (text, id) in forward.iter().chain(af.iter()) {
        m.string(text)?;
        if *id as usize >= ENTRIES {
            return Err(());
        }
    }
    for text in reverse.values() {
        m.string(text)?;
    }
    for token in ar.values().chain(classic.iter()).chain(special.iter()) {
        added(m, token)?;
    }
    for text in set {
        m.string(text)?;
    }
    for id in ids {
        m.work(1)?;
        if *id as usize >= ENTRIES {
            return Err(());
        }
    }
    if let Some(p) = tokenizer.get_padding() {
        m.string(&p.pad_token)?;
    }

    // Reverse-map ID sorting avoids key comparisons and never scans max ID.
    // Forward keys sort with an explicit conservative byte-read charge.
    let mut f: Vec<(&String, &u32)> = Vec::new();
    m.copy((forward.len() + af.len()) * std::mem::size_of::<(&String, &u32)>())?;
    f.try_reserve_exact(forward.len() + af.len())
        .map_err(|_| ())?;
    f.extend(forward.iter());
    let base_len = f.len();
    f.extend(af.iter());
    let cmp = |a: (&String, &u32), b: (&String, &u32), m: &mut Meter| {
        m.work(1 + a.0.len().min(b.0.len()))?;
        Ok(a.0.cmp(b.0))
    };
    sorted(&mut f[..base_len], m, cmp)?;
    sorted(&mut f[base_len..], m, cmp)?;
    // Compute the effective forward-map union, with added keys overriding
    // model keys. Count overlaps once and refuse conflicting/doubled IDs.
    // The fixed presence array is part of the stated stack metadata envelope.
    let mut present = [false; ENTRIES];
    let (mut i, mut j, mut count) = (0, base_len, 0);
    while i < base_len || j < f.len() {
        m.work(1)?;
        let pair = if i == base_len {
            let pair = f[j];
            j += 1;
            pair
        } else if j == f.len() {
            let pair = f[i];
            i += 1;
            pair
        } else {
            match cmp(f[i], f[j], m)? {
                Ordering::Less => {
                    let pair = f[i];
                    i += 1;
                    pair
                }
                Ordering::Greater => {
                    let pair = f[j];
                    j += 1;
                    pair
                }
                Ordering::Equal => {
                    let pair = f[j];
                    i += 1;
                    j += 1;
                    pair
                }
            }
        };
        let id = *pair.1 as usize;
        if present[id] {
            return Err(());
        }
        present[id] = true;
        count += 1;
    }
    m.work(3 * ENTRIES)?;
    if count == 0
        || count > ENTRIES
        || present[..count].iter().any(|v| !*v)
        || present[count..].iter().any(|v| *v)
    {
        return Err(());
    }
    let mut r: Vec<(&u32, &String)> = Vec::new();
    m.copy(reverse.len() * std::mem::size_of::<(&u32, &String)>())?;
    r.try_reserve_exact(reverse.len()).map_err(|_| ())?;
    r.extend(reverse.iter());
    sorted(&mut r, m, |a, b, m| {
        m.work(1)?;
        Ok(a.0.cmp(b.0))
    })?;
    let mut a: Vec<(&u32, &AddedToken)> = Vec::new();
    m.copy(ar.len() * std::mem::size_of::<(&u32, &AddedToken)>())?;
    a.try_reserve_exact(ar.len()).map_err(|_| ())?;
    a.extend(ar.iter());
    sorted(&mut a, m, |a, b, m| {
        m.work(1)?;
        Ok(a.0.cmp(b.0))
    })?;
    let mut s = refs(set.len(), set.iter(), m)?;
    sorted(&mut s, m, |a, b, m| {
        m.work(1 + a.len().min(b.len()))?;
        Ok(a.cmp(b))
    })?;
    // At most 64KiB raw strings plus fixed per-entry framing (all vectors
    // already <=4096); one allocation, no growth/reallocation copies.
    let envelope = TEXT + 128 * ENTRIES + 1024;
    let mut output = Vec::new();
    output.try_reserve_exact(envelope).map_err(|_| ())?;
    let mut sink = Sink {
        bytes: output,
        meter: m,
    };
    sink.raw(PREFIX)?;
    sink.raw(&[whitespace, vocabulary.get_encode_special_tokens() as u8])?;
    sink.text(&wordlevel.unk_token)?;
    for entries in [&f[..base_len], &f[base_len..]] {
        sink.number(entries.len())?;
        for (text, id) in entries {
            sink.text(text)?;
            sink.number(**id as usize)?;
        }
    }
    sink.number(r.len())?;
    for (id, text) in r {
        sink.number(*id as usize)?;
        sink.text(text)?;
    }
    sink.number(a.len())?;
    for (id, token) in a {
        sink.number(*id as usize)?;
        sink.token(token)?;
    }
    for history in [classic, special] {
        sink.number(history.len())?;
        for token in history {
            sink.token(token)?;
        }
    }
    sink.number(s.len())?;
    for text in s {
        sink.text(text)?;
    }
    sink.number(ids.len())?;
    for id in ids {
        sink.number(*id as usize)?;
    }
    sink.number(trie_bytes)?;
    match tokenizer.get_padding() {
        None => sink.raw(&[0])?,
        Some(p) => {
            sink.raw(&[1, matches!(p.direction, PaddingDirection::Left) as u8])?;
            match p.strategy {
                PaddingStrategy::BatchLongest => sink.raw(&[0])?,
                PaddingStrategy::Fixed(n) => {
                    sink.raw(&[1])?;
                    sink.number(n)?;
                }
            }
            match p.pad_to_multiple_of {
                None => sink.raw(&[0])?,
                Some(n) => {
                    sink.raw(&[1])?;
                    sink.number(n)?;
                }
            }
            sink.number(p.pad_id as usize)?;
            sink.number(p.pad_type_id as usize)?;
            sink.text(&p.pad_token)?;
        }
    }
    match tokenizer.get_truncation() {
        None => sink.raw(&[0])?,
        Some(t) => {
            sink.raw(&[
                1,
                matches!(t.direction, TruncationDirection::Left) as u8,
                match t.strategy {
                    TruncationStrategy::LongestFirst => 0,
                    TruncationStrategy::OnlyFirst => 1,
                    TruncationStrategy::OnlySecond => 2,
                },
            ])?;
            sink.number(t.max_length)?;
            sink.number(t.stride)?;
        }
    }
    // Guards and the outer immutable Python borrow remain held through the
    // complete native traversal and encoding. No GIL release/callback here.
    Ok(sink.bytes)
}

pub(crate) fn inspect<'py>(
    native: &PyTokenizer,
    py: Python<'py>,
) -> PyResult<(bool, Bound<'py, PyBytes>, usize, usize)> {
    // Fixed compiled call/work/copy ceilings are admitted before native work.
    // Meter retains the spent prefix on refusal; no caller-supplied envelope.
    let mut meter = Meter::default();
    let result = snapshot(native, &mut meter);
    let (accepted, payload) = match result {
        Ok(bytes) if meter.copy(2 * bytes.len()).is_ok() => (true, bytes),
        _ => (false, Vec::new()),
    };
    // Fallible allocation preserves ordinary MemoryError/refusal and collector
    // unwind cleanup. new_with zero-initializes before this sealed copy; BOTH
    // variable buffer writes were reserved above. No arbitrary callback runs.
    let bytes = PyBytes::new_with(py, payload.len(), |target| {
        target.copy_from_slice(&payload);
        Ok(())
    })?;
    Ok((accepted, bytes, meter.work, meter.copied))
}

#[cfg(test)]
#[path = "snapshot_units.rs"]
mod snapshot_units;
