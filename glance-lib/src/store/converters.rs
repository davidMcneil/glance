//! Wrapper types for converting from higher level types to sql data types

use blake3::Hash;
use derive_more::{From, Into};
use file_format::FileFormat;
use std::{ffi::OsStr, path::PathBuf};

use rusqlite::{
    types::{FromSql, FromSqlError, FromSqlResult, ToSqlOutput, ValueRef},
    Error, ToSql,
};

#[derive(Debug, From, Into)]
pub(crate) struct FileFormatSql(pub FileFormat);

impl ToSql for FileFormatSql {
    fn to_sql(&self) -> Result<ToSqlOutput<'_>, Error> {
        Ok(self.0.to_string().into())
    }
}

impl FromSql for FileFormatSql {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        Ok(FileFormatSql(FileFormat::from_bytes(value.as_bytes()?)))
    }
}

#[derive(Debug, From, Into)]
pub(crate) struct HashSql(pub Hash);

impl ToSql for HashSql {
    fn to_sql(&self) -> Result<ToSqlOutput<'_>, Error> {
        Ok(self.0.as_bytes().to_vec().into())
    }
}

impl FromSql for HashSql {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        Ok(HashSql(Hash::from_bytes(
            value
                .as_bytes()?
                .try_into()
                .map_err(|e| FromSqlError::Other(Box::new(e)))?,
        )))
    }
}

#[derive(Debug, From, Into)]
pub(crate) struct PathBufSql(pub PathBuf);

impl ToSql for PathBufSql {
    fn to_sql(&self) -> Result<ToSqlOutput<'_>, Error> {
        let v: &OsStr = self.0.as_ref();
        <&str>::try_from(v)
            .map(|v| v.into())
            .map_err(|e| Error::ToSqlConversionFailure(e.into()))
    }
}

impl FromSql for PathBufSql {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        Ok(PathBufSql(PathBuf::from(
            value
                .as_str()
                .map_err(|e| FromSqlError::Other(Box::new(e)))?,
        )))
    }
}
