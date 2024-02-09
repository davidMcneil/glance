use std::path::PathBuf;

use derive_more::{From, Into};

use crate::store::label_sql::LabelSql;

#[derive(Debug)]
pub struct Label {
    pub filepath: PathBuf,
    pub label: String,
}

impl From<LabelSql> for Label {
    fn from(value: LabelSql) -> Self {
        Self {
            filepath: value.filepath.into(),
            label: value.label,
        }
    }
}

impl From<Label> for LabelSql {
    fn from(value: Label) -> Self {
        Self {
            filepath: value.filepath.into(),
            label: value.label,
        }
    }
}
