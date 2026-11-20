use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Component, Path, PathBuf};

const LOCK_PATH: &str = "ucd/15.1.0/ucd-lock.json";
const REQUIRED_FILES: [&str; 4] = [
    "ucd/15.1.0/UnicodeData.txt",
    "ucd/15.1.0/NormalizationTest.txt",
    "ucd/15.1.0/DerivedNormalizationProps.txt",
    "ucd/15.1.0/CompositionExclusions.txt",
];

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct LockEnvelope {
    payload: LockPayload,
    payload_sha256: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct LockPayload {
    schema_version: u32,
    unicode_version: String,
    unicode_release_url: String,
    files: Vec<LockedFile>,
    derived_artifacts: Vec<DerivedArtifact>,
    normalizer: NormalizerBinding,
    generator: GeneratorIdentity,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct LockedFile {
    path: String,
    sha256: String,
    size_bytes: u64,
    role: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct DerivedArtifact {
    path: String,
    sha256: String,
    size_bytes: u64,
    generator_version: String,
    input_payload_sha256: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct NormalizerBinding {
    crate_name: String,
    crate_version: String,
    cargo_lock_sha256: String,
    role: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct GeneratorIdentity {
    name: String,
    version: String,
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut output = String::with_capacity(64);
    for byte in digest {
        use std::fmt::Write;
        write!(&mut output, "{byte:02x}").expect("writing to String cannot fail");
    }
    output
}

fn require_sha256(element: &str, value: &str) {
    let valid = value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte));
    if !valid {
        panic!(
            "{element}: expected a 64-character lowercase hexadecimal SHA-256, actual {value:?}"
        );
    }
}

fn resolve_under_root(root: &Path, relative: &str, element: &str) -> PathBuf {
    let path = Path::new(relative);
    if path.is_absolute()
        || path
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        panic!(
            "{element}: expected a relative path containing only normal components, actual {relative:?}"
        );
    }
    root.join(path)
}

fn read_file(path: &Path, element: &str) -> Vec<u8> {
    fs::read(path).unwrap_or_else(|error| {
        panic!(
            "{element}: expected readable file at {}, actual I/O error: {error}",
            path.display()
        )
    })
}

fn verify_file(
    root: &Path,
    relative_path: &str,
    expected_size: u64,
    expected_sha256: &str,
    element: &str,
) {
    require_sha256(&format!("{element}.sha256"), expected_sha256);
    let path = resolve_under_root(root, relative_path, element);
    let bytes = read_file(&path, element);
    let actual_size = bytes.len() as u64;
    if actual_size != expected_size {
        panic!(
            "{element}.size_bytes: expected {expected_size}, actual {actual_size}"
        );
    }
    let actual_sha256 = sha256_hex(&bytes);
    if actual_sha256 != expected_sha256 {
        panic!(
            "{element}.sha256: expected {expected_sha256}, actual {actual_sha256}"
        );
    }
}

fn main() {
    println!("cargo:rerun-if-changed={LOCK_PATH}");
    for path in REQUIRED_FILES {
        println!("cargo:rerun-if-changed={path}");
    }

    let manifest_dir = std::env::var_os("CARGO_MANIFEST_DIR")
        .unwrap_or_else(|| panic!("CARGO_MANIFEST_DIR: expected crate root, actual environment variable is missing"));
    let root = PathBuf::from(manifest_dir);

    let lock_file = resolve_under_root(&root, LOCK_PATH, "lock");
    let lock_bytes = read_file(&lock_file, "lock");
    let lock_text = std::str::from_utf8(&lock_bytes)
        .unwrap_or_else(|error| panic!("lock: expected UTF-8 JSON text, actual UTF-8 error: {error}"));
    if lock_text.contains("REPLACE_WITH_") {
        panic!("lock: expected no REPLACE_WITH_ placeholders, actual lock text contains REPLACE_WITH_");
    }

    let mut envelope: LockEnvelope = serde_json::from_str(lock_text)
        .unwrap_or_else(|error| panic!("lock: expected valid lock envelope JSON, actual parse error: {error}"));

    if envelope.payload.schema_version != 1 {
        panic!(
            "payload.schema_version: expected 1, actual {}",
            envelope.payload.schema_version
        );
    }
    if envelope.payload.unicode_version != "15.1.0" {
        panic!(
            "payload.unicode_version: expected 15.1.0, actual {}",
            envelope.payload.unicode_version
        );
    }

    require_sha256("envelope.payload_sha256", &envelope.payload_sha256);
    require_sha256(
        "payload.normalizer.cargo_lock_sha256",
        &envelope.payload.normalizer.cargo_lock_sha256,
    );

    let payload_bytes = serde_json::to_vec(&envelope.payload)
        .unwrap_or_else(|error| panic!("payload serialization: expected serializable typed payload, actual error: {error}"));
    let actual_payload_sha256 = sha256_hex(&payload_bytes);
    if actual_payload_sha256 != envelope.payload_sha256 {
        panic!(
            "envelope.payload_sha256: expected {}, actual {}",
            envelope.payload_sha256, actual_payload_sha256
        );
    }

    let expected_input_payload_sha256 = envelope
        .payload
        .derived_artifacts
        .first()
        .map(|artifact| artifact.input_payload_sha256.clone());
    envelope.payload.derived_artifacts.clear();
    let empty_payload_bytes = serde_json::to_vec(&envelope.payload)
        .unwrap_or_else(|error| panic!("empty-derived payload serialization: expected serializable typed payload, actual error: {error}"));
    let actual_empty_payload_sha256 = sha256_hex(&empty_payload_bytes);

    for (index, artifact) in envelope
        .payload
        .derived_artifacts
        .iter()
        .enumerate()
    {
        let element = format!("payload.derived_artifacts[{index}]");
        require_sha256(
            &format!("{element}.input_payload_sha256"),
            &artifact.input_payload_sha256,
        );
        if artifact.input_payload_sha256 != actual_empty_payload_sha256 {
            panic!(
                "{element}.input_payload_sha256: expected {}, actual {}",
                artifact.input_payload_sha256, actual_empty_payload_sha256
            );
        }
    }

    if let Some(expected) = expected_input_payload_sha256 {
        if expected != actual_empty_payload_sha256 {
            panic!(
                "payload.derived_artifacts[0].input_payload_sha256: expected {expected}, actual {actual_empty_payload_sha256}"
            );
        }
    }

    let derived_artifacts = {
        let lock_bytes = read_file(&lock_file, "lock re-read");
        let lock_text = std::str::from_utf8(&lock_bytes)
            .unwrap_or_else(|error| panic!("lock re-read: expected UTF-8, actual error: {error}"));
        let original: LockEnvelope = serde_json::from_str(lock_text)
            .unwrap_or_else(|error| panic!("lock re-read: expected valid JSON, actual error: {error}"));
        original.payload.derived_artifacts
    };

    for (index, file) in envelope.payload.files.iter().enumerate() {
        let element = format!("payload.files[{index}] ({})", file.path);
        verify_file(
            &root,
            &file.path,
            file.size_bytes,
            &file.sha256,
            &element,
        );
    }

    for (index, artifact) in derived_artifacts.iter().enumerate() {
        println!("cargo:rerun-if-changed={}", artifact.path);
        let element = format!(
            "payload.derived_artifacts[{index}] ({})",
            artifact.path
        );
        verify_file(
            &root,
            &artifact.path,
            artifact.size_bytes,
            &artifact.sha256,
            &element,
        );
    }

    println!(
        "cargo:warning=UCD 15.1.0 lock verified: {} files, payload SHA-256 {}",
        envelope.payload.files.len(),
        &actual_payload_sha256[..12]
    );
}
