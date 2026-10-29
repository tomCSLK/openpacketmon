// Modified for openpacketmon (2026), see NOTICE. Based on `src/mmdb/types/mmdb_reader.rs` of Sniffnet
// (https://github.com/GyulyVGC/sniffnet), Copyright (c) 2022 Giuliano Bellini, MIT OR Apache-2.0.

use std::net::IpAddr;
use std::path::{Path, PathBuf};
use std::sync::{Arc, PoisonError, RwLock};

use maxminddb::{Metadata, Reader};
use serde::Deserialize;

/// The country and ASN databases; a capture keeps a clone, so databases opened later
/// (a download, another path in the settings) are used for the addresses found from then on
#[derive(Clone, Default)]
pub struct MmdbReaders {
    pub country: SharedReader,
    pub asn: SharedReader,
}

/// A database that can be replaced while captures use it
#[derive(Clone, Default)]
pub struct SharedReader(Arc<RwLock<Arc<MmdbReader>>>);

impl SharedReader {
    pub fn new(reader: MmdbReader) -> Self {
        Self(Arc::new(RwLock::new(Arc::new(reader))))
    }

    pub fn get(&self) -> Arc<MmdbReader> {
        self.0
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    pub fn set(&self, reader: MmdbReader) {
        *self.0.write().unwrap_or_else(PoisonError::into_inner) = Arc::new(reader);
    }
}

#[derive(Default)]
pub enum MmdbReader {
    /// A database file and where it was read from
    File(PathBuf, Box<Reader<Vec<u8>>>),
    /// No database: countries and AS stay unknown
    #[default]
    Empty,
}

impl MmdbReader {
    /// Reads a database file; `None` if it is missing or no MMDB file
    pub fn open(path: &Path) -> Option<MmdbReader> {
        Reader::open_readfile(path)
            .ok()
            .map(|reader| MmdbReader::File(path.to_path_buf(), Box::new(reader)))
    }

    pub fn lookup<'a, T: Deserialize<'a>>(&'a self, ip: IpAddr) -> Option<T> {
        match self {
            MmdbReader::File(_, reader) => reader.lookup(ip).and_then(|lr| lr.decode()).ok()?,
            MmdbReader::Empty => None,
        }
    }

    /// The file in use and its metadata (type, build time)
    pub fn source(&self) -> Option<(&Path, &Metadata)> {
        match self {
            MmdbReader::File(path, reader) => Some((path, reader.metadata())),
            MmdbReader::Empty => None,
        }
    }
}
