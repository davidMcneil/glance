use std::path::Path;
use std::str::FromStr;
use std::{io::Error, path::PathBuf};

use derive_more::{From, Into};

#[derive(Clone, Debug, Into, From)]
pub struct CanonicalizedPathBuf(PathBuf);

impl FromStr for CanonicalizedPathBuf {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(PathBuf::from(s).canonicalize()?))
    }
}

impl AsRef<Path> for CanonicalizedPathBuf {
    fn as_ref(&self) -> &Path {
        self.0.as_ref()
    }
}
