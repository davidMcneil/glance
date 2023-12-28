use std::{io::Error, path::PathBuf};

use derive_more::{From, FromStr, Into};

#[derive(Clone, Debug, Into, From)]
pub struct CanonicalizedPathBuf(PathBuf);

impl FromStr for CanonicalizedPathBuf {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(PathBuf::from(s).canonicalize()?))
    }
}
