use core::fmt;
use std::{ops::Range, path::PathBuf};

use ::arrow::array::RecordBatch;
use itertools::Itertools;
use thiserror::Error;

use crate::metadata::DataType;

pub mod arrow;
pub mod attributes;
pub mod manifest;
pub mod structure;

pub type Path = PathBuf;

/// The id of a file in object store
/// FIXME: should this be passed by ref everywhere?
#[derive(Hash, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ObjectId(pub [u8; 16]); // FIXME: this doesn't need to be this big

impl ObjectId {
    const SIZE: usize = 16;

    pub fn random() -> ObjectId {
        ObjectId(rand::random())
    }

    pub const FAKE: ObjectId = ObjectId([0; 16]);
}

impl fmt::Debug for ObjectId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:02x}", self.0.iter().format(""))
    }
}

impl TryFrom<&[u8]> for ObjectId {
    type Error = &'static str;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        let buf = value.try_into();
        buf.map(ObjectId).map_err(|_| "Invalid ObjectId buffer length")
    }
}

/// The internal id of an array or group, unique only to a single store version
pub type NodeId = u32;

#[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
/// An ND index to an element in a chunk grid.
pub struct ChunkIndices(pub Vec<u64>);

pub type ChunkOffset = u64;
pub type ChunkLength = u64;

pub type TableOffset = u32;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TableRegion(TableOffset, TableOffset);

impl TableRegion {
    pub fn new(start: TableOffset, end: TableOffset) -> Option<Self> {
        if start < end {
            Some(Self(start, end))
        } else {
            None
        }
    }

    /// Create a TableRegion of size 1
    pub fn singleton(start: TableOffset) -> Self {
        #[allow(clippy::expect_used)] // this implementation is used exclusively for tests
        Self::new(start, start + 1).expect("bug in TableRegion::singleton")
    }

    pub fn start(&self) -> TableOffset {
        self.0
    }

    pub fn end(&self) -> TableOffset {
        self.1
    }

    pub fn extend_right(&mut self, shift_by: TableOffset) {
        self.1 += shift_by
    }
}

impl TryFrom<Range<TableOffset>> for TableRegion {
    type Error = &'static str;

    fn try_from(value: Range<TableOffset>) -> Result<Self, Self::Error> {
        Self::new(value.start, value.end).ok_or("invalid range")
    }
}

impl From<TableRegion> for Range<TableOffset> {
    fn from(value: TableRegion) -> Self {
        Range { start: value.start(), end: value.end() }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Flags(); // FIXME: implement

#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum IcechunkFormatError {
    #[error("error decoding fill_value from array")]
    FillValueDecodeError { found_size: usize, target_size: usize, target_type: DataType },
    #[error("error decoding fill_value from json")]
    FillValueParse { data_type: DataType, value: serde_json::Value },
    #[error("column not found: `{column}`")]
    ColumnNotFound { column: String },
    #[error("invalid column type: `{expected_column_type}` for column `{column_name}`")]
    InvalidColumnType { column_name: String, expected_column_type: String },
    #[error("invalid path: `{path:?}`")]
    InvalidPath { path: Path },
    #[error("node not found at `{path:?}`")]
    NodeNotFound { path: Path },
    #[error("unexpected null element at column `{column_name}` index `{index}`")]
    NullElement { index: usize, column_name: String },
    #[error("invalid node type `{node_type}` at index `{index}`")]
    InvalidNodeType { index: usize, node_type: String },
    #[error("invalid array metadata field `{field}` at index `{index}`: {message}")]
    InvalidArrayMetadata { index: usize, field: String, message: String },
    #[error("invalid array manifest field `{field}` at index `{index}`: {message}")]
    InvalidArrayManifest { index: usize, field: String, message: String },
    #[error("invalid manifest index `{index}` > `{max_index}`")]
    InvalidManifestIndex { index: usize, max_index: usize },
    #[error("chunk coordinates not found `{coords:?}`")]
    ChunkCoordinatesNotFound { coords: ChunkIndices },
}

pub type IcechunkResult<T> = Result<T, IcechunkFormatError>;

pub trait BatchLike {
    fn get_batch(&self) -> &RecordBatch;
}
