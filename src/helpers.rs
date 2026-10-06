use std::sync::Arc;

use arrow_array::builder::{Float32Builder, Int64Builder, ListBuilder, StringBuilder};
use arrow_array::{ArrayRef, RecordBatch};
use arrow_schema::{DataType, Field, Schema};

pub fn table_schema() -> Arc<Schema> {
    Arc::new(Schema::new(vec![
        Field::new("id", DataType::Utf8, false),
        Field::new("content", DataType::Utf8, false),
        Field::new(
            "vector",
            DataType::List(Arc::new(Field::new("item", DataType::Float32, true))),
            false,
        ),
        Field::new("category", DataType::Utf8, false),
        Field::new("file_hash", DataType::Utf8, false),
        Field::new("timestamp", DataType::Int64, false),
    ]))
}

pub fn build_arrow_record(
    id: &str,
    content: &str,
    category: &str,
    hash: &str,
    ts: i64,
    vector: Vec<f32>,
    dimension: usize,
) -> Result<RecordBatch, String> {
    let schema = table_schema();

    let mut id_builder = StringBuilder::with_capacity(1, id.len());
    let mut content_builder = StringBuilder::with_capacity(1, content.len());
    let mut category_builder = StringBuilder::with_capacity(1, category.len());
    let mut hash_builder = StringBuilder::with_capacity(1, hash.len());
    let mut ts_builder = Int64Builder::with_capacity(1);

    let values_builder = Float32Builder::with_capacity(dimension);
    let mut vector_builder = ListBuilder::with_capacity(values_builder, 1);

    id_builder.append_value(id);
    content_builder.append_value(content);
    category_builder.append_value(category);
    hash_builder.append_value(hash);
    ts_builder.append_value(ts);

    if vector.len() != dimension {
        return Err(format!(
            "Critical error: Length of vector from Model ({}) does not match VECTOR_DIMENSION ({})",
            vector.len(),
            dimension
        ));
    }

    let values_item_builder = vector_builder.values();
    for &val in &vector {
        values_item_builder.append_value(val);
    }
    vector_builder.append(true);

    let id_array: ArrayRef = Arc::new(id_builder.finish());
    let content_array: ArrayRef = Arc::new(content_builder.finish());
    let vector_array: ArrayRef = Arc::new(vector_builder.finish());
    let category_array: ArrayRef = Arc::new(category_builder.finish());
    let hash_array: ArrayRef = Arc::new(hash_builder.finish());
    let ts_array: ArrayRef = Arc::new(ts_builder.finish());

    let columns = vec![
        id_array,
        content_array,
        vector_array,
        category_array,
        hash_array,
        ts_array,
    ];

    RecordBatch::try_new(schema, columns)
        .map_err(|e| format!("Critical error building Arrow RecordBatch: {}", e))
}
