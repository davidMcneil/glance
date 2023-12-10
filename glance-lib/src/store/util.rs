use blake3::Hash;
use derive_more::{From, Into};
use file_format::FileFormat;
use std::{ffi::OsStr, path::PathBuf};

use rusqlite::{types::ToSqlOutput, Error, ToSql};

#[derive(Debug, From, Into)]
pub(crate) struct FileFormatSql(FileFormat);

impl ToSql for FileFormatSql {
    fn to_sql(&self) -> Result<ToSqlOutput<'_>, Error> {
        Ok(self.0.to_string().into())
    }
}

#[derive(Debug, From, Into)]
pub(crate) struct HashSql(Hash);

impl ToSql for HashSql {
    fn to_sql(&self) -> Result<ToSqlOutput<'_>, Error> {
        Ok(Vec::from(self.0.as_bytes()).into())
    }
}

#[derive(Debug, From, Into)]
pub(crate) struct PathBufSql(PathBuf);

impl ToSql for PathBufSql {
    fn to_sql(&self) -> Result<ToSqlOutput<'_>, Error> {
        let v: &OsStr = self.0.as_ref();
        <&str>::try_from(v)
            .map(|v| v.into())
            .map_err(|e| Error::ToSqlConversionFailure(e.into()))
    }
}
