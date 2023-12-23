use std::{collections::HashMap, fmt::Display, hash::Hash};

use serde::{Serialize, Serializer};

/// Type to allow serializing HashMap<Option<K>, V> with `Option::None` as `"Unknown"``
#[derive(Serialize)]
pub struct HashMapWithUnknown<K: Eq + Display + Hash, V>(HashMap<KeyOrUnknown<K>, V>);

impl<K: Eq + Display + Hash, V> From<HashMapWithUnknown<K, V>> for HashMap<Option<K>, V> {
    fn from(value: HashMapWithUnknown<K, V>) -> Self {
        value
            .0
            .into_iter()
            .map(|(k, v)| (Option::from(k), v))
            .collect::<HashMap<_, _>>()
    }
}

impl<K: Eq + Display + Hash, V> From<HashMap<Option<K>, V>> for HashMapWithUnknown<K, V> {
    fn from(value: HashMap<Option<K>, V>) -> Self {
        Self(
            value
                .into_iter()
                .map(|(k, v)| (KeyOrUnknown::from(k), v))
                .collect::<HashMap<_, _>>(),
        )
    }
}

#[derive(Eq, PartialEq, Hash)]
enum KeyOrUnknown<K: Eq + Hash> {
    Unknown,
    Key(K),
}

impl<K: Eq + Display + Hash> Display for KeyOrUnknown<K> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            KeyOrUnknown::Unknown => "Unknown".fmt(f),
            KeyOrUnknown::Key(k) => k.fmt(f),
        }
    }
}

impl<K: Eq + Display + Hash> Serialize for KeyOrUnknown<K> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<K: Eq + PartialEq + Hash> From<Option<K>> for KeyOrUnknown<K> {
    fn from(value: Option<K>) -> Self {
        match value {
            None => Self::Unknown,
            Some(k) => Self::Key(k),
        }
    }
}

impl<K: Eq + PartialEq + Hash> From<KeyOrUnknown<K>> for Option<K> {
    fn from(value: KeyOrUnknown<K>) -> Self {
        match value {
            KeyOrUnknown::Unknown => Self::None,
            KeyOrUnknown::Key(k) => Self::Some(k),
        }
    }
}
