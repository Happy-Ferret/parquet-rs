// Licensed to the Apache Software Foundation (ASF) under one
// or more contributor license agreements.  See the NOTICE file
// distributed with this work for additional information
// regarding copyright ownership.  The ASF licenses this file
// to you under the Apache License, Version 2.0 (the
// "License"); you may not use this file except in compliance
// with the License.  You may obtain a copy of the License at
//
//   http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing,
// software distributed under the License is distributed on an
// "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY
// KIND, either express or implied.  See the License for the
// specific language governing permissions and limitations
// under the License.

use std::rc::Rc;

use basic::{Compression, Encoding, Type};
use errors::{ParquetError, Result};
use schema::types::{ColumnDescriptor, ColumnDescPtr, ColumnPath};
use schema::types::{SchemaDescriptor, SchemaDescPtr, Type as SchemaType, TypePtr};
use parquet_thrift::parquet::{ColumnChunk, ColumnMetaData, RowGroup};

pub type ParquetMetaDataPtr = Rc<ParquetMetaData>;

pub struct ParquetMetaData {
  file_metadata: FileMetaDataPtr,
  row_groups: Vec<RowGroupMetaDataPtr>
}

impl ParquetMetaData {
  pub fn new(file_metadata: FileMetaData, row_groups: Vec<RowGroupMetaData>) -> Self {
    ParquetMetaData {
      file_metadata: Rc::new(file_metadata),
      row_groups: row_groups.into_iter().map(|r| Rc::new(r)).collect()
    }
  }

  pub fn file_metadata(&self) -> FileMetaDataPtr {
    self.file_metadata.clone()
  }

  pub fn num_row_groups(&self) -> usize {
    self.row_groups.len()
  }

  pub fn row_group(&self, i: usize) -> RowGroupMetaDataPtr {
    self.row_groups[i].clone()
  }

  pub fn row_groups(&self) -> &[RowGroupMetaDataPtr] {
    &self.row_groups.as_slice()
  }
}

pub type FileMetaDataPtr = Rc<FileMetaData>;

/// Metadata for a Parquet file
pub struct FileMetaData {
  version: i32,
  num_rows: i64,
  created_by: Option<String>,
  schema: TypePtr,
  schema_descr: SchemaDescPtr
}

impl FileMetaData {
  pub fn new(
    version: i32,
    num_rows: i64,
    created_by: Option<String>,
    schema: TypePtr,
    schema_descr: SchemaDescPtr
  ) -> Self {
    FileMetaData {
      version,
      num_rows,
      created_by,
      schema,
      schema_descr
    }
  }

  pub fn version(&self) -> i32 {
    self.version
  }

  pub fn num_rows(&self) -> i64 {
    self.num_rows
  }

  pub fn created_by(&self) -> &Option<String> {
    &self.created_by
  }

  pub fn schema(&self) -> &SchemaType {
    self.schema.as_ref()
  }

  pub fn schema_descr(&self) -> &SchemaDescriptor {
    &self.schema_descr
  }

  pub fn schema_descr_ptr(&self) -> SchemaDescPtr {
    self.schema_descr.clone()
  }
}

pub type RowGroupMetaDataPtr = Rc<RowGroupMetaData>;

/// Metadata for a row group
pub struct RowGroupMetaData {
  columns: Vec<ColumnChunkMetaDataPtr>,
  num_rows: i64,
  total_byte_size: i64,
  schema_descr: SchemaDescPtr
}

impl RowGroupMetaData {
  pub fn num_columns(&self) -> usize {
    self.columns.len()
  }

  pub fn column(&self, i: usize) -> &ColumnChunkMetaData {
    &self.columns[i]
  }

  pub fn columns(&self) -> &[ColumnChunkMetaDataPtr] {
    &self.columns
  }

  pub fn num_rows(&self) -> i64 {
    self.num_rows
  }

  pub fn total_byte_size(&self) -> i64 {
    self.total_byte_size
  }

  pub fn schema_descr(&self) -> &SchemaDescriptor {
    self.schema_descr.as_ref()
  }

  pub fn schema_descr_ptr(&self) -> SchemaDescPtr {
    self.schema_descr.clone()
  }

  pub fn from_thrift(
    schema_descr: SchemaDescPtr,
    mut rg: RowGroup
  ) -> Result<RowGroupMetaData> {
    assert_eq!(schema_descr.num_columns(), rg.columns.len());
    let total_byte_size = rg.total_byte_size;
    let num_rows = rg.num_rows;
    let mut columns = vec![];
    for (c, d) in rg.columns.drain(0..).zip(schema_descr.columns()) {
      let cc = ColumnChunkMetaData::from_thrift(d.clone(), c)?;
      columns.push(Rc::new(cc));
    }
    Ok(RowGroupMetaData {
      columns,
      num_rows,
      total_byte_size,
      schema_descr
    })
  }
}

pub type ColumnChunkMetaDataPtr = Rc<ColumnChunkMetaData>;

/// Metadata for a column chunk
pub struct ColumnChunkMetaData {
  column_type: Type,
  column_path: ColumnPath,
  column_descr: ColumnDescPtr,
  encodings: Vec<Encoding>,
  file_path: Option<String>,
  file_offset: i64,
  num_values: i64,
  compression: Compression,
  total_compressed_size: i64,
  total_uncompressed_size: i64,
  data_page_offset: i64,
  index_page_offset: Option<i64>,
  dictionary_page_offset: Option<i64>
}

/// Represents common operations for a column chunk
impl ColumnChunkMetaData {
  /// File where the column chunk is stored. If not set, assumed to
  /// be at the same file as the metadata.
  /// This path is relative to the current file.
  pub fn file_path(&self) -> Option<&String> {
    self.file_path.as_ref()
  }

  /// Byte offset in `file_path()`.
  pub fn file_offset(&self) -> i64 {
    self.file_offset
  }

  /// Type of this column. Must be primitive.
  pub fn column_type(&self) -> Type {
    self.column_type
  }

  /// Path (or identifier) of this column
  pub fn column_path(&self) -> &ColumnPath {
    &self.column_path
  }

  /// Descriptor for this column
  pub fn column_descr(&self) -> &ColumnDescriptor {
    self.column_descr.as_ref()
  }

  /// Reference counted clone of descriptor for this column
  pub fn column_descr_ptr(&self) -> ColumnDescPtr {
    self.column_descr.clone()
  }

  /// All encodings used for this column
  pub fn encodings(&self) -> &Vec<Encoding> {
    &self.encodings
  }

  /// Total number of values in this column chunk
  pub fn num_values(&self) -> i64 {
    self.num_values
  }

  pub fn compression(&self) -> Compression {
    self.compression
  }

  /// Get the total compressed data size of this column chunk
  pub fn compressed_size(&self) -> i64 {
    self.total_compressed_size
  }

  /// Get the total uncompressed data size of this column chunk
  pub fn uncompressed_size(&self) -> i64 {
    self.total_uncompressed_size
  }

  /// Get the offset for the column data
  pub fn data_page_offset(&self) -> i64 {
    self.data_page_offset
  }

  /// Whether this column chunk contains a index page
  pub fn has_index_page(&self) -> bool {
    self.index_page_offset.is_some()
  }

  /// Get the offset for the index page
  pub fn index_page_offset(&self) -> Option<i64> {
    self.index_page_offset
  }

  /// Whether this column chunk contains a dictionary page
  pub fn has_dictionary_page(&self) -> bool {
    self.dictionary_page_offset.is_some()
  }

  /// TODO: add statistics

  /// Get the offset for the dictionary page, if any
  pub fn dictionary_page_offset(&self) -> Option<i64> {
    self.dictionary_page_offset
  }

  /// Conversion from Thrift
  pub fn from_thrift(column_descr: ColumnDescPtr, cc: ColumnChunk) -> Result<Self> {
    if cc.meta_data.is_none() {
      return Err(general_err!("Expected to have column metadata"));
    }
    let mut col_metadata: ColumnMetaData = cc.meta_data.unwrap();
    let column_type = Type::from(col_metadata.type_);
    let column_path = ColumnPath::new(col_metadata.path_in_schema);
    let encodings = col_metadata.encodings.drain(0..).map(Encoding::from).collect();
    let compression = Compression::from(col_metadata.codec);
    let file_path = cc.file_path;
    let file_offset = cc.file_offset;
    let num_values = col_metadata.num_values;
    let total_compressed_size = col_metadata.total_compressed_size;
    let total_uncompressed_size = col_metadata.total_uncompressed_size;
    let data_page_offset = col_metadata.data_page_offset;
    let index_page_offset = col_metadata.index_page_offset;
    let dictionary_page_offset = col_metadata.dictionary_page_offset;
    let result = ColumnChunkMetaData {
      column_type,
      column_path,
      column_descr,
      encodings,
      file_path,
      file_offset,
      num_values,
      compression,
      total_compressed_size,
      total_uncompressed_size,
      data_page_offset,
      index_page_offset,
      dictionary_page_offset
    };
    Ok(result)
  }
}
