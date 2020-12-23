use semver::{Version, VersionReq};

use std::fmt;

use crate::{utils, Query};

#[derive(Debug, Clone)]
pub struct VersionWrap(Version);

impl VersionWrap {
    pub fn from(v: Version) -> VersionWrap {
        VersionWrap(v)
    }

    pub fn inner(&self) -> &Version {
        &self.0
    }
}

impl serde::Serialize for VersionWrap {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.0.to_string())
    }
}

pub struct VersionVisitor;

impl<'de> serde::de::Visitor<'de> for VersionVisitor {
    type Value = VersionWrap;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a semver formatted version")
    }

    fn visit_str<E>(self, s: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        match Version::parse(s) {
            Ok(v) => Ok(VersionWrap(v)),
            Err(e) => Err(E::custom(e.to_string())),
        }
    }
}

impl<'de> serde::Deserialize<'de> for VersionWrap {
    fn deserialize<D>(deserializer: D) -> Result<VersionWrap, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_str(VersionVisitor)
    }
}

#[derive(Debug, Clone)]
pub struct DependencyWrap(String, VersionReq);

impl DependencyWrap {
    pub fn from(q: Query) -> DependencyWrap {
        DependencyWrap(q.0, q.1)
    }

    pub fn inner(&self) -> (&String, &VersionReq) {
        (&self.0, &self.1)
    }
}

impl serde::Serialize for DependencyWrap {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(format!("{}{}", self.0.to_string(), self.1.to_string()).as_str())
    }
}

struct DependencyVisitor;

impl<'de> serde::de::Visitor<'de> for DependencyVisitor {
    type Value = DependencyWrap;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a semver formatted version requirement")
    }

    fn visit_str<E>(self, s: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        match utils::parse_pkg_str_info(s) {
            Ok((name, vreq)) => Ok(DependencyWrap(name, vreq)),
            Err(e) => Err(E::custom(e.to_string())),
        }
    }
}

impl<'de> serde::Deserialize<'de> for DependencyWrap {
    fn deserialize<D>(deserializer: D) -> Result<DependencyWrap, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_str(DependencyVisitor)
    }
}
