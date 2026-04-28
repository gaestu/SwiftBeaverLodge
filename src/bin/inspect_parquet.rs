//! Utility to inspect parquet file schema and data

use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;
use std::fs::File;

fn main() {
    let path = std::env::args()
        .nth(1)
        .expect("Usage: inspect_parquet <path>");
    let file = File::open(&path).unwrap();
    let builder = ParquetRecordBatchReaderBuilder::try_new(file).unwrap();
    let schema = builder.schema();

    println!("Schema for: {}", path);
    println!("{}", "-".repeat(60));
    for field in schema.fields() {
        println!("  {:20} : {:?}", field.name(), field.data_type());
    }

    let mut reader = ParquetRecordBatchReaderBuilder::try_new(File::open(&path).unwrap())
        .unwrap()
        .build()
        .unwrap();

    if let Some(batch) = reader.next() {
        let batch = batch.unwrap();
        println!("\nRows: {}", batch.num_rows());

        if batch.num_rows() > 0 {
            println!("\nFirst row:");
            for (i, field) in batch.schema().fields().iter().enumerate() {
                let col = batch.column(i);
                println!("  {:20} = {:?}", field.name(), col.slice(0, 1));
            }
        }
    }
}
