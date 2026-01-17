//! LanceDB schema and record batch utilities for vector storage

use std::sync::Arc;

use arrow_array::types::Float64Type;
use arrow_array::{
    ArrayRef, BooleanArray, FixedSizeListArray, Int32Array, RecordBatch, StringArray,
};
use arrow_schema::{DataType, Field, Schema};
use rig::embeddings::Embedding;

use super::error::RagError;
use crate::agent::types::DocChunk;

/// LanceDB table name for doc chunks
pub const CHUNKS_TABLE_NAME: &str = "doc_chunks";

/// Default embedding dimensions for common models
pub const DEFAULT_EMBEDDING_DIM: i32 = 1024; // Qwen3-Embedding-0.6B uses 1024 dimensions

/// Create Arrow schema for the chunks table
pub fn create_schema(embedding_dim: i32) -> Arc<Schema> {
    Arc::new(Schema::new(vec![
        Field::new("id", DataType::Utf8, false),
        Field::new("doc_id", DataType::Utf8, false),
        Field::new("title", DataType::Utf8, false),
        Field::new("doc_title", DataType::Utf8, false),
        Field::new("section", DataType::Utf8, false),
        Field::new("chunk_index", DataType::Int32, false),
        Field::new("total_chunks", DataType::Int32, false),
        Field::new("header_context", DataType::Utf8, false),
        Field::new("content", DataType::Utf8, false),
        Field::new("has_code", DataType::Boolean, false),
        Field::new(
            "vector",
            DataType::FixedSizeList(
                Arc::new(Field::new("item", DataType::Float64, true)),
                embedding_dim,
            ),
            false,
        ),
    ]))
}

/// Convert embeddings to Arrow RecordBatch
pub fn embeddings_to_record_batch(
    embeddings: &[(DocChunk, Embedding)],
    embedding_dim: i32,
) -> Result<RecordBatch, RagError> {
    let ids: Vec<&str> = embeddings.iter().map(|(c, _)| c.id.as_str()).collect();
    let doc_ids: Vec<&str> = embeddings.iter().map(|(c, _)| c.doc_id.as_str()).collect();
    let titles: Vec<&str> = embeddings.iter().map(|(c, _)| c.title.as_str()).collect();
    let doc_titles: Vec<&str> = embeddings
        .iter()
        .map(|(c, _)| c.doc_title.as_str())
        .collect();
    let sections: Vec<&str> = embeddings.iter().map(|(c, _)| c.section.as_str()).collect();
    let chunk_indices: Vec<i32> = embeddings
        .iter()
        .map(|(c, _)| c.chunk_index as i32)
        .collect();
    let total_chunks: Vec<i32> = embeddings
        .iter()
        .map(|(c, _)| c.total_chunks as i32)
        .collect();
    let header_contexts: Vec<&str> = embeddings
        .iter()
        .map(|(c, _)| c.header_context.as_str())
        .collect();
    let contents: Vec<&str> = embeddings.iter().map(|(c, _)| c.content.as_str()).collect();
    let has_codes: Vec<bool> = embeddings.iter().map(|(c, _)| c.has_code).collect();

    // Build vector array - flatten all embeddings into a single Vec<f64>
    let vectors_flat: Vec<f64> = embeddings
        .iter()
        .flat_map(|(_, emb)| emb.vec.iter().copied())
        .collect();

    let vectors_array = FixedSizeListArray::from_iter_primitive::<Float64Type, _, _>(
        vectors_flat
            .chunks(embedding_dim as usize)
            .map(|chunk| Some(chunk.iter().copied().map(Some).collect::<Vec<_>>())),
        embedding_dim,
    );

    let schema = create_schema(embedding_dim);

    let batch = RecordBatch::try_new(
        schema,
        vec![
            Arc::new(StringArray::from(ids)) as ArrayRef,
            Arc::new(StringArray::from(doc_ids)) as ArrayRef,
            Arc::new(StringArray::from(titles)) as ArrayRef,
            Arc::new(StringArray::from(doc_titles)) as ArrayRef,
            Arc::new(StringArray::from(sections)) as ArrayRef,
            Arc::new(Int32Array::from(chunk_indices)) as ArrayRef,
            Arc::new(Int32Array::from(total_chunks)) as ArrayRef,
            Arc::new(StringArray::from(header_contexts)) as ArrayRef,
            Arc::new(StringArray::from(contents)) as ArrayRef,
            Arc::new(BooleanArray::from(has_codes)) as ArrayRef,
            Arc::new(vectors_array) as ArrayRef,
        ],
    )
    .map_err(|e| RagError::LanceDb(format!("Failed to create RecordBatch: {}", e)))?;

    Ok(batch)
}

/// Helper to extract string column from a RecordBatch
pub fn get_string_col<'a>(
    batch: &'a RecordBatch,
    name: &str,
) -> Result<&'a StringArray, RagError> {
    batch
        .column_by_name(name)
        .ok_or_else(|| RagError::LanceDb(format!("Missing {} column", name)))?
        .as_any()
        .downcast_ref::<StringArray>()
        .ok_or_else(|| RagError::LanceDb(format!("{} column has wrong type", name)))
}

/// Helper to extract i32 column from a RecordBatch
pub fn get_i32_col<'a>(
    batch: &'a RecordBatch,
    name: &str,
) -> Result<&'a Int32Array, RagError> {
    batch
        .column_by_name(name)
        .ok_or_else(|| RagError::LanceDb(format!("Missing {} column", name)))?
        .as_any()
        .downcast_ref::<Int32Array>()
        .ok_or_else(|| RagError::LanceDb(format!("{} column has wrong type", name)))
}

/// Helper to extract bool column from a RecordBatch
pub fn get_bool_col<'a>(
    batch: &'a RecordBatch,
    name: &str,
) -> Result<&'a BooleanArray, RagError> {
    batch
        .column_by_name(name)
        .ok_or_else(|| RagError::LanceDb(format!("Missing {} column", name)))?
        .as_any()
        .downcast_ref::<BooleanArray>()
        .ok_or_else(|| RagError::LanceDb(format!("{} column has wrong type", name)))
}
