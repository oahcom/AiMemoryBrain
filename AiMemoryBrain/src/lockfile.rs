// AiMemoryBrain/src/lockfile.rs

use serde::de::{self, MapAccess, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fmt;
use std::fs;
use std::path::{Component, Path};

pub const LOCK_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct UcdLockEnvelope {
    pub payload: UcdLockPayload,
    pub payload_sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct UcdLockPayload {
    pub schema_version: u32,
    pub unicode_version: String,
    pub unicode_release_url: String,
    pub files: Vec<LockedFile>,
    pub derived_artifacts: Vec<DerivedArtifact>,
    pub normalizer: NormalizerBinding,
    pub generator: GeneratorIdentity,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct LockedFile {
    pub path: String,
    pub sha256: String,
    pub size_bytes: u64,
    pub role: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct DerivedArtifact {
    pub path: String,
    pub sha256: String,
    pub size_bytes: u64,
    pub generator_version: String,
    pub input_payload_sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct NormalizerBinding {
    pub crate_name: String,
    pub crate_version: String,
    pub cargo_lock_sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct GeneratorIdentity {
    pub name: String,
    pub version: String,
}

#[derive(Debug)]
pub enum LockError {
    Io(std::io::Error),
    Json(serde_json::Error),
    Invalid(String),
}

impl fmt::Display for LockError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(e) => write!(f, "lock file I/O error: {e}"),
            Self::Json(e) => write!(f, "lock file JSON error: {e}"),
            Self::Invalid(message) => write!(f, "invalid lock file: {message}"),
        }
    }
}

impl std::error::Error for LockError {}

impl From<std::io::Error> for LockError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<serde_json::Error> for LockError {
    fn from(value: serde_json::Error) -> Self {
        Self::Json(value)
    }
}

/// A JSON value deserializer that rejects duplicate object keys.
///
/// serde_json::Value alone does not provide a duplicate-key rejection
/// guarantee. This intermediate representation checks keys while reading.
#[derive(Debug)]
enum StrictJson {
    Null,
    Bool(bool),
    Number(serde_json::Number),
    String(String),
    Array(Vec<StrictJson>),
    Object(BTreeMap<String, StrictJson>),
}

impl<'de> Deserialize<'de> for StrictJson {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct StrictVisitor;

        impl<'de> Visitor<'de> for StrictVisitor {
            type Value = StrictJson;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a JSON value with no duplicate object keys")
            }

            fn visit_unit<E>(self) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(StrictJson::Null)
            }

            fn visit_none<E>(self) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(StrictJson::Null)
            }

            fn visit_bool<E>(self, value: bool) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(StrictJson::Bool(value))
            }

            fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(StrictJson::Number(value.into()))
            }

            fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(StrictJson::Number(value.into()))
            }

            fn visit_f64<E>(self, value: f64) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                let number = serde_json::Number::from_f64(value)
                    .ok_or_else(|| E::custom("non-finite JSON number"))?;
                Ok(StrictJson::Number(number))
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(StrictJson::String(value.to_owned()))
            }

            fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(StrictJson::String(value))
            }

            fn visit_seq<A>(self, mut sequence: A) -> Result<Self::Value, A::Error>
            where
                A: SeqAccess<'de>,
            {
                let mut values = Vec::new();

                while let Some(value) = sequence.next_element::<StrictJson>()? {
                    values.push(value);
                }

                Ok(StrictJson::Array(values))
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: MapAccess<'de>,
            {
                let mut values = BTreeMap::new();

                while let Some((key, value)) =
                    map.next_entry::<String, StrictJson>()?
                {
                    if values.insert(key.clone(), value).is_some() {
                        return Err(de::Error::custom(format!(
                            "duplicate JSON object key: {key}"
                        )));
                    }
                }

                Ok(StrictJson::Object(values))
            }
        }

        deserializer.deserialize_any(StrictVisitor)
    }
}

impl StrictJson {
    fn into_value(self) -> serde_json::Value {
        match self {
            Self::Null => serde_json::Value::Null,
            Self::Bool(value) => serde_json::Value::Bool(value),
            Self::Number(value) => serde_json::Value::Number(value),
            Self::String(value) => serde_json::Value::String(value),
            Self::Array(values) => serde_json::Value::Array(
                values.into_iter().map(Self::into_value).collect(),
            ),
            Self::Object(values) => {
                let mut object = serde_json::Map::new();

                for (key, value) in values {
                    object.insert(key, value.into_value());
                }

                serde_json::Value::Object(object)
            }
        }
    }
}

/// Read, parse, and validate a lock file. No defaults are applied.
pub fn read_lock(path: impl AsRef<Path>) -> Result<UcdLockEnvelope, LockError> {
    let bytes = fs::read(path)?;
    let strict: StrictJson = serde_json::from_slice(&bytes)?;
    let value = strict.into_value();
    let envelope: UcdLockEnvelope = serde_json::from_value(value)?;

    validate_envelope(&envelope)?;
    Ok(envelope)
}

/// Validate schema, field constraints, paths, and the payload digest.
pub fn validate_envelope(
    envelope: &UcdLockEnvelope,
) -> Result<(), LockError> {
    let payload = &envelope.payload;

    if payload.schema_version != LOCK_SCHEMA_VERSION {
        return Err(LockError::Invalid(format!(
            "unsupported schema_version {}; expected {}",
            payload.schema_version, LOCK_SCHEMA_VERSION
        )));
    }

    validate_nonempty("unicode_version", &payload.unicode_version)?;
    validate_nonempty("unicode_release_url", &payload.unicode_release_url)?;

    if !payload.unicode_release_url.starts_with("https://www.unicode.org/Public/")
        || !payload
            .unicode_release_url
            .ends_with("/ucd/")
    {
        return Err(LockError::Invalid(
            "unicode_release_url must be an HTTPS unicode.org UCD release URL"
                .to_owned(),
        ));
    }

    if payload.files.is_empty() {
        return Err(LockError::Invalid(
            "files must contain at least one locked source file".to_owned(),
        ));
    }

    let mut file_paths = std::collections::BTreeSet::new();

    for file in &payload.files {
        validate_relative_path("files[].path", &file.path)?;
        validate_sha256("files[].sha256", &file.sha256)?;

        if file.size_bytes == 0 {
            return Err(LockError::Invalid(format!(
                "files[].size_bytes must be nonzero for {}",
                file.path
            )));
        }

        validate_nonempty("files[].role", &file.role)?;

        if !file_paths.insert(file.path.as_str()) {
            return Err(LockError::Invalid(format!(
                "duplicate locked file path: {}",
                file.path
            )));
        }
    }

    let mut artifact_paths = std::collections::BTreeSet::new();

    for artifact in &payload.derived_artifacts {
        validate_relative_path("derived_artifacts[].path", &artifact.path)?;
        validate_sha256("derived_artifacts[].sha256", &artifact.sha256)?;
        validate_sha256(
            "derived_artifacts[].input_payload_sha256",
            &artifact.input_payload_sha256,
        )?;
        validate_nonempty(
            "derived_artifacts[].generator_version",
            &artifact.generator_version,
        )?;

        if artifact.size_bytes == 0 {
            return Err(LockError::Invalid(format!(
                "derived artifact size_bytes must be nonzero for {}",
                artifact.path
            )));
        }

        if !artifact_paths.insert(artifact.path.as_str()) {
            return Err(LockError::Invalid(format!(
                "duplicate derived artifact path: {}",
                artifact.path
            )));
        }
    }

    validate_nonempty("normalizer.crate_name", &payload.normalizer.crate_name)?;
    validate_nonempty(
        "normalizer.crate_version",
        &payload.normalizer.crate_version,
    )?;
    validate_sha256(
        "normalizer.cargo_lock_sha256",
        &payload.normalizer.cargo_lock_sha256,
    )?;

    validate_nonempty("generator.name", &payload.generator.name)?;
    validate_nonempty("generator.version", &payload.generator.version)?;

    validate_sha256("payload_sha256", &envelope.payload_sha256)?;

    let actual_digest = payload_sha256(payload)?;
    if actual_digest != envelope.payload_sha256 {
        return Err(LockError::Invalid(format!(
            "payload digest mismatch: declared {}, computed {}",
            envelope.payload_sha256, actual_digest
        )));
    }

    Ok(())
}

/// Deterministic digest of the typed payload.
///
/// The payload is serialized as a struct with fixed field order. The
/// serialization format is compact JSON with no insignificant whitespace.
pub fn payload_sha256(
    payload: &UcdLockPayload,
) -> Result<String, LockError> {
    let bytes = serde_json::to_vec(payload)?;
    Ok(sha256_hex(&bytes))
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut output = String::with_capacity(64);

    for byte in digest {
        use std::fmt::Write;
        let _ = write!(&mut output, "{byte:02x}");
    }

    output
}

fn validate_nonempty(field: &str, value: &str) -> Result<(), LockError> {
    if value.trim().is_empty() {
        return Err(LockError::Invalid(format!(
            "{field} must be a non-empty string"
        )));
    }

    if value != value.trim() {
        return Err(LockError::Invalid(format!(
            "{field} must not have leading or trailing whitespace"
        )));
    }

    Ok(())
}

fn validate_sha256(field: &str, value: &str) -> Result<(), LockError> {
    let valid = value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte));

    if !valid {
        return Err(LockError::Invalid(format!(
            "{field} must be 64 lowercase hexadecimal characters"
        )));
    }

    Ok(())
}

fn validate_relative_path(field: &str, value: &str) -> Result<(), LockError> {
    validate_nonempty(field, value)?;

    if value.contains('\\') || value.starts_with('/') {
        return Err(LockError::Invalid(format!(
            "{field} must be a normalized relative path: {value}"
        )));
    }

    let path = Path::new(value);

    if path.components().any(|component| {
        !matches!(component, Component::Normal(_))
    }) {
        return Err(LockError::Invalid(format!(
            "{field} contains an unsafe path component: {value}"
        )));
    }

    if path.to_string_lossy() != value {
        return Err(LockError::Invalid(format!(
            "{field} is not in normalized path form: {value}"
        )));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_payload() -> UcdLockPayload {
        UcdLockPayload {
            schema_version: LOCK_SCHEMA_VERSION,
            unicode_version: "15.1.0".to_owned(),
            unicode_release_url:
                "https://www.unicode.org/Public/15.1.0/ucd/".to_owned(),
            files: vec![LockedFile {
                path: "UnicodeData.txt".to_owned(),
                sha256: "a".repeat(64),
                size_bytes: 1,
                role: "unicode-data".to_owned(),
            }],
            derived_artifacts: vec![DerivedArtifact {
                path: "generated/tables.bin".to_owned(),
                sha256: "b".repeat(64),
                size_bytes: 1,
                generator_version: "1.0.0".to_owned(),
                input_payload_sha256: "c".repeat(64),
            }],
            normalizer: NormalizerBinding {
                crate_name: "unicode-normalization".to_owned(),
                crate_version: "0.1.24".to_owned(),
                cargo_lock_sha256: "d".repeat(64),
            },
            generator: GeneratorIdentity {
                name: "d-unicode-validator".to_owned(),
                version: "0.1.0".to_owned(),
            },
        }
    }

    fn sample_envelope() -> UcdLockEnvelope {
        let payload = sample_payload();
        let digest = payload_sha256(&payload)
            .expect("test payload serialization should succeed");

        UcdLockEnvelope {
            payload,
            payload_sha256: digest,
        }
    }

    #[test]
    fn accepts_valid_envelope() {
        let envelope = sample_envelope();
        assert!(validate_envelope(&envelope).is_ok());
    }

    #[test]
    fn rejects_payload_digest_mismatch() {
        let mut envelope = sample_envelope();
        envelope.payload_sha256 = "0".repeat(64);

        assert!(validate_envelope(&envelope).is_err());
    }

    #[test]
    fn rejects_unsafe_paths() {
        let mut envelope = sample_envelope();
        envelope.payload.files[0].path = "../UnicodeData.txt".to_owned();
        envelope.payload_sha256 = payload_sha256(&envelope.payload)
            .expect("test payload serialization should succeed");

        assert!(validate_envelope(&envelope).is_err());
    }

    #[test]
    fn rejects_duplicate_json_keys() {
        let json = br#"{
            "payload": {},
            "payload": {},
            "payload_sha256": "0000000000000000000000000000000000000000000000000000000000000000"
        }"#;

        let parsed = serde_json::from_slice::<StrictJson>(json);
        assert!(parsed.is_err());
    }

    #[test]
    fn rejects_unknown_typed_fields() {
        let json = br#"{
            "payload": {
                "schema_version": 1,
                "unicode_version": "15.1.0",
                "unicode_release_url": "https://www.unicode.org/Public/15.1.0/ucd/",
                "files": [],
                "derived_artifacts": [],
                "normalizer": {
                    "crate_name": "unicode-normalization",
                    "crate_version": "0.1.24",
                    "cargo_lock_sha256": "dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd"
                },
                "generator": {
                    "name": "test",
                    "version": "1"
                },
                "unexpected": true
            },
            "payload_sha256": "0000000000000000000000000000000000000000000000000000000000000000"
        }"#;

        let strict = serde_json::from_slice::<StrictJson>(json)
            .expect("JSON syntax and unique keys should be valid");
        let value = strict.into_value();
        assert!(serde_json::from_value::<UcdLockEnvelope>(value).is_err());
    }
}
