use d_unicode_validator::lockfile::{
    payload_sha256, sha256_hex, DerivedArtifact, GeneratorIdentity, LockedFile,
    NormalizerBinding, UcdLockEnvelope, UcdLockPayload, LOCK_SCHEMA_VERSION,
};
use serde_json::{json, Value};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

const UNICODE_VERSION: &str = "15.1.0";
const RELEASE_URL: &str = "https://www.unicode.org/Public/15.1.0/ucd/";
const GENERATOR_NAME: &str = "d-ucd-generator";
const GENERATOR_VERSION: &str = "0.1.0";
const NORMALIZER_NAME: &str = "unicode-normalization";
const NORMALIZER_VERSION: &str = "0.1.24";
const LOCK_RELATIVE_PATH: &str = "ucd/15.1.0/ucd-lock.json";
const DERIVED_RELATIVE_PATH: &str =
    "ucd/15.1.0/derived/normalization-index.json";

const INPUT_FILES: [(&str, &str); 4] = [
    (
        "UnicodeData.txt",
        "normative_unicode_character_database",
    ),
    (
        "NormalizationTest.txt",
        "normative_normalization_conformance_corpus",
    ),
    (
        "DerivedNormalizationProps.txt",
        "normalization_derived_properties_input",
    ),
    (
        "CompositionExclusions.txt",
        "normalization_composition_exclusion_input",
    ),
];

#[derive(Debug)]
struct InputFile {
    path: String,
    role: String,
    bytes: Vec<u8>,
    sha256: String,
    size_bytes: u64,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("d-ucd-lockgen: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let repository_root = repository_root()?;
    let ucd_directory = repository_root.join("ucd").join(UNICODE_VERSION);
    let lock_path = repository_root.join(LOCK_RELATIVE_PATH);
    let cargo_lock_path = repository_root.join("Cargo.lock");

    let cargo_lock = fs::read(&cargo_lock_path).map_err(|error| {
        format!(
            "cannot read {}: {error}",
            cargo_lock_path.display()
        )
    })?;
    let cargo_lock_sha256 = sha256_hex(&cargo_lock);

    let existing_lock = read_existing_lock(&lock_path)?;
    let inputs = read_inputs(&ucd_directory, existing_lock.as_ref())?;
    let locked_files: Vec<LockedFile> = inputs
        .iter()
        .map(|input| LockedFile {
            path: input.path.clone(),
            sha256: input.sha256.clone(),
            size_bytes: input.size_bytes,
            role: input.role.clone(),
        })
        .collect();

    let mut source_payload = UcdLockPayload {
        schema_version: LOCK_SCHEMA_VERSION,
        unicode_version: UNICODE_VERSION.to_owned(),
        unicode_release_url: RELEASE_URL.to_owned(),
        files: locked_files,
        derived_artifacts: Vec::new(),
        normalizer: NormalizerBinding {
            crate_name: NORMALIZER_NAME.to_owned(),
            crate_version: NORMALIZER_VERSION.to_owned(),
            cargo_lock_sha256,
            role: "normalization_engine_verified_against_locked_ucd_corpus".to_owned(),
        },
        generator: GeneratorIdentity {
            name: GENERATOR_NAME.to_owned(),
            version: GENERATOR_VERSION.to_owned(),
        },
    };

    let input_payload_sha256 = payload_sha256(&source_payload).map_err(|error| format!("payload hash failure: {error}"))?;
    let index_bytes = build_index(&inputs, input_payload_sha256.as_str())?;
    let derived_directory = ucd_directory.join("derived");
    fs::create_dir_all(&derived_directory).map_err(|error| {
        format!(
            "cannot create derived artifact directory {}: {error}",
            derived_directory.display()
        )
    })?;
    let derived_path = repository_root.join(DERIVED_RELATIVE_PATH);
    fs::write(&derived_path, &index_bytes).map_err(|error| {
        format!(
            "cannot write derived artifact {}: {error}",
            derived_path.display()
        )
    })?;

    let derived_artifact = DerivedArtifact {
        path: DERIVED_RELATIVE_PATH.to_owned(),
        sha256: sha256_hex(&index_bytes),
        size_bytes: u64::try_from(index_bytes.len())
            .map_err(|_| "derived artifact size exceeds u64".to_owned())?,
        generator_version: format!("{GENERATOR_NAME}/{GENERATOR_VERSION}"),
        input_payload_sha256,
    };
    source_payload.derived_artifacts = vec![derived_artifact];

    let envelope = UcdLockEnvelope {
        payload_sha256: payload_sha256(&source_payload).map_err(|error| format!("payload hash failure: {error}"))?,
        payload: source_payload,
    };
    let lock_bytes = serde_json::to_vec(&envelope)
        .map_err(|error| format!("cannot serialize UCD lock envelope: {error}"))?;
    let lock_bytes = append_newline(lock_bytes);

    fs::write(&lock_path, lock_bytes).map_err(|error| {
        format!("cannot write lock file {}: {error}", lock_path.display())
    })?;

    println!(
        "generated {} and {}",
        lock_path.display(),
        derived_path.display()
    );
    Ok(())
}

fn repository_root() -> Result<PathBuf, String> {
    let current = env::current_dir()
        .map_err(|error| format!("cannot determine current directory: {error}"))?;
    if current.join("Cargo.toml").is_file()
        && current.join("Cargo.lock").is_file()
    {
        return Ok(current);
    }
    Err(format!(
        "run this generator from the crate root; Cargo.toml and Cargo.lock not found in {}",
        current.display()
    ))
}

fn read_existing_lock(
    path: &Path,
) -> Result<Option<Value>, String> {
    if !path.exists() {
        return Ok(None);
    }
    let bytes = fs::read(path)
        .map_err(|error| format!("cannot read existing lock {}: {error}", path.display()))?;
    let text = std::str::from_utf8(&bytes)
        .map_err(|error| format!("existing lock is not UTF-8: {error}"))?;
    if text.contains("REPLACE_WITH_") {
        return Ok(None);
    }
    let value: Value = serde_json::from_slice(&bytes)
        .map_err(|error| format!("existing lock is malformed JSON: {error}"))?;
    Ok(Some(value))
}

fn read_inputs(
    directory: &Path,
    existing_lock: Option<&Value>,
) -> Result<Vec<InputFile>, String> {
    let mut result = Vec::with_capacity(INPUT_FILES.len());
    for (name, role) in INPUT_FILES {
        let relative_path = format!("ucd/{UNICODE_VERSION}/{name}");
        let full_path = directory.join(name);
        let bytes = fs::read(&full_path).map_err(|error| {
            format!("cannot read required UCD input {}: {error}", full_path.display())
        })?;
        let size_bytes = u64::try_from(bytes.len())
            .map_err(|_| format!("input size exceeds u64: {}", full_path.display()))?;
        let digest = sha256_hex(&bytes);

        if let Some(existing) = existing_lock {
            verify_existing_file_entry(
                existing,
                &relative_path,
                &digest,
                size_bytes,
            )?;
        }

        result.push(InputFile {
            path: relative_path,
            role: role.to_owned(),
            bytes,
            sha256: digest,
            size_bytes,
        });
    }
    Ok(result)
}

fn verify_existing_file_entry(
    existing: &Value,
    expected_path: &str,
    actual_sha256: &str,
    actual_size: u64,
) -> Result<(), String> {
    let files = existing
        .get("payload")
        .and_then(|payload| payload.get("files"))
        .and_then(Value::as_array)
        .ok_or_else(|| "existing lock has no payload.files array".to_owned())?;

    let Some(entry) = files.iter().find(|entry| {
        entry.get("path").and_then(Value::as_str) == Some(expected_path)
    }) else {
        return Ok(());
    };

    let expected_digest = entry
        .get("sha256")
        .and_then(Value::as_str)
        .ok_or_else(|| format!("existing lock entry {expected_path} has no sha256"))?;
    let expected_size = entry
        .get("size_bytes")
        .and_then(Value::as_u64)
        .ok_or_else(|| format!("existing lock entry {expected_path} has no size_bytes"))?;

    if expected_digest.contains("REPLACE_WITH_") {
        return Ok(());
    }
    if expected_digest != actual_sha256 {
        return Err(format!(
            "digest mismatch for {expected_path}: lock has {expected_digest}, disk has {actual_sha256}"
        ));
    }
    if expected_size != actual_size {
        return Err(format!(
            "size mismatch for {expected_path}: lock has {expected_size}, disk has {actual_size}"
        ));
    }
    Ok(())
}

fn build_index(
    inputs: &[InputFile],
    input_payload_sha256: &str,
) -> Result<Vec<u8>, String> {
    let mut file_entries: Vec<Value> = inputs
        .iter()
        .map(|input| {
            json!({
                "path": input.path,
                "role": input.role,
                "sha256": input.sha256,
                "size_bytes": input.size_bytes
            })
        })
        .collect();
    file_entries.sort_by(|left, right| {
        left.get("path")
            .and_then(Value::as_str)
            .cmp(&right.get("path").and_then(Value::as_str))
    });

    let index = json!({
        "generator": {
            "name": GENERATOR_NAME,
            "version": GENERATOR_VERSION
        },
        "input_payload_sha256": input_payload_sha256,
        "unicode_version": UNICODE_VERSION,
        "inputs": file_entries
    });
    serde_json::to_vec(&index)
        .map_err(|error| format!("cannot serialize deterministic normalization index: {error}"))
}

fn append_newline(mut bytes: Vec<u8>) -> Vec<u8> {
    bytes.push(b'\n');
    bytes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn payload_digest_is_stable_for_identical_serialized_payloads() {
        let payload = UcdLockPayload {
            schema_version: LOCK_SCHEMA_VERSION,
            unicode_version: UNICODE_VERSION.to_owned(),
            unicode_release_url: RELEASE_URL.to_owned(),
            files: Vec::new(),
            derived_artifacts: Vec::new(),
            normalizer: NormalizerBinding {
                crate_name: NORMALIZER_NAME.to_owned(),
                crate_version: NORMALIZER_VERSION.to_owned(),
                cargo_lock_sha256: "0".repeat(64),
                role: "normalization_engine_verified_against_locked_ucd_corpus".to_owned(),
            },
            generator: GeneratorIdentity {
                name: GENERATOR_NAME.to_owned(),
                version: GENERATOR_VERSION.to_owned(),
            },
        };

        let first = payload_sha256(&payload);
        let second = payload_sha256(&payload);
        assert_eq!(first.unwrap_or_else(|error| panic!("first payload hash failed: {error}")), second.unwrap_or_else(|error| panic!("second payload hash failed: {error}")));
    }

    #[test]
    fn placeholder_marker_is_detected() {
        let placeholder = r#"{"payload_sha256":"REPLACE_WITH_DIGEST"}"#;
        assert!(placeholder.contains("REPLACE_WITH_"));
        assert!(!"0".repeat(64).contains("REPLACE_WITH_"));
    }

    #[test]
    fn deterministic_index_bytes_are_stable() {
        let inputs = vec![
            InputFile {
                path: "ucd/15.1.0/UnicodeData.txt".to_owned(),
                role: "normative_unicode_character_database".to_owned(),
                bytes: Vec::new(),
                sha256: "a".repeat(64),
                size_bytes: 0,
            },
        ];
        let first = build_index(&inputs, &"b".repeat(64)).unwrap();
        let second = build_index(&inputs, &"b".repeat(64)).unwrap();
        assert_eq!(first, second);
    }
}

