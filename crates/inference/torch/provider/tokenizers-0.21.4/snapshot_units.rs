// Pantograph native qualification units. Apache-2.0.
use super::*;
use crate::models::PyModel;
use crate::pre_tokenizers::PyPreTokenizer;
use ahash::AHashMap;
use tk::models::wordlevel::WordLevel;
use tk::pre_tokenizers::whitespace::Whitespace;
use tk::tokenizer::TokenizerImpl;

fn native(vocab: AHashMap<String, u32>) -> PyTokenizer {
    let model = WordLevel::builder()
        .vocab(vocab)
        .unk_token("unk".into())
        .build()
        .unwrap();
    let mut tokenizer = TokenizerImpl::new(PyModel::from(model));
    tokenizer.with_pre_tokenizer(Some(PyPreTokenizer::new(
        PyPreTokenizerWrapper::Wrapped(PreTokenizerWrapper::Whitespace(Whitespace)).into(),
    )));
    PyTokenizer { tokenizer }
}

#[test]
fn retained_table_capacity_refuses_before_string_or_scratch_copy() {
    let mut vocab = AHashMap::with_capacity(4 * ENTRIES);
    vocab.insert("unk".into(), 0);
    let tokenizer = native(vocab);
    let mut meter = Meter::default();
    assert!(snapshot(&tokenizer, &mut meter).is_err());
    assert_eq!(meter.copied, 0);
    assert_eq!(meter.text, 0);
}

#[test]
fn model_contention_refuses_without_waiting_or_copying() {
    let tokenizer = native([("unk".into(), 0)].into());
    let _writer = tokenizer.tokenizer.get_model().model.write().unwrap();
    let mut meter = Meter::default();
    assert!(snapshot(&tokenizer, &mut meter).is_err());
    assert_eq!(meter.copied, 0);
}

#[test]
fn pretokenizer_contention_refuses_without_copying() {
    let tokenizer = native([("unk".into(), 0)].into());
    let pretok = tokenizer.tokenizer.get_pre_tokenizer().unwrap();
    let lock = match &pretok.pretok {
        PyPreTokenizerTypeWrapper::Single(lock) => lock,
        _ => unreachable!(),
    };
    let _writer = lock.write().unwrap();
    let mut meter = Meter::default();
    assert!(snapshot(&tokenizer, &mut meter).is_err());
    assert_eq!(meter.copied, 0);
}

#[test]
fn poisoned_model_refuses_with_fixed_outcome() {
    let tokenizer = native([("unk".into(), 0)].into());
    let lock = tokenizer.tokenizer.get_model().model.clone();
    let _ = std::thread::spawn(move || {
        let _writer = lock.write().unwrap();
        panic!("synthetic poison");
    })
    .join();
    let mut meter = Meter::default();
    assert!(snapshot(&tokenizer, &mut meter).is_err());
    assert_eq!(meter.copied, 0);
}

#[test]
fn huge_sparse_id_never_enters_the_legacy_serializer() {
    let tokenizer = native([("unk".into(), 0), ("sparse".into(), u32::MAX)].into());
    let mut meter = Meter::default();
    assert!(snapshot(&tokenizer, &mut meter).is_err());
    assert_eq!(meter.copied, 0);
}

#[test]
fn error_prefix_keeps_admitted_charges_and_never_exceeds_ceilings() {
    let mut meter = Meter::default();
    meter.copy(COPY).unwrap();
    let charged = (meter.work, meter.copied);
    assert!(meter.copy(1).is_err());
    assert_eq!((meter.work, meter.copied), charged);
    assert!(meter.work(usize::MAX).is_err());
    assert_eq!((meter.work, meter.copied), charged);
}
