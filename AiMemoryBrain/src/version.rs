// AiMemoryBrain/src/version.rs

use crate::lockfile::{
    payload_sha256, DerivedArtifact, LockError, LockedFile, UcdLockEnvelope,
};
use sha2::{Digest, Sha256};
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct UnicodeVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VersionBindingReport {
    pub version: UnicodeVersion,
    pub verified_source_files: usize,
    pub verified_derived_artifacts: usize,
    pub cargo_lock_verified: bool,
}

#[derive(Debug)]
pub enum VersionError {
    Lock(LockError),
    Io(std::io::Error),
    Invalid(String),
}

impl fmt::Display for VersionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Lock(error) => write!(f, "{error}"),
            Self::Io(error) => write!(f, "version binding I/O error: {error}"),
            Self::Invalid(message) => write!(f, "version binding failed: {message}"),
        }
    }
}

impl std::error::Error for VersionError {}

impl From<LockError> for VersionError {
    fn from(value: LockError) -> Self {
        Self::Lock(value)
    }
}

impl From<std::io::Error> for VersionError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

impl UnicodeVersion {
    pub fn parse(value: &str) -> Result<Self, VersionError> {
        let parts: Vec<&str> = value.split('.').collect();

        if parts.len() != 3 {
            return Err(VersionError::Invalid(format!(
                "Unicode version must have exactly three numeric components: {value}"
            )));
        }

        let major = parse_component(parts[0], value)?;
        let minor = parse_component(parts[1], value)?;
        let patch = parse_component(parts[2], value)?;

        Ok(Self {
            major,
            minor,
            patch,
        })
    }

    pub fn as_string(&self) -> String {
        format!("{}.{}.{}", self.major, self.minor, self.patch)
    }
}

impl fmt::Display for UnicodeVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

fn parse_component(component: &str, full: &str) -> Result<u32, VersionError> {
    if component.is_empty()
        || !component.bytes().all(|byte| byte.is_ascii_digit())
        || (component.len() > 1 && component.starts_with('0'))
    {
        return Err(VersionError::Invalid(format!(
            "invalid numeric component in Unicode version: {full}"
        )));
    }

    component.parse::<u32>().map_err(|error| {
        VersionError::Invalid(format!(
            "Unicode version component out of range in {full}: {error}"
        ))
    })
}

/// Verify the lock's Unicode version, source files, derived artifacts,
/// and Cargo.lock binding against the checked-out repository root.
///
/// `root` is the crate/repository root. Locked source and artifact paths are
/// interpreted relative to this root.
pub fn verify_version_binding(
    root: impl AsRef<Path>,
    envelope: &UcdLockEnvelope,
    expected: &str,
) -> Result<VersionBindingReport, VersionError> {
    let root = root.as_ref();
    let expected_version = UnicodeVersion::parse(expected)?;
    let locked_version =
        UnicodeVersion::parse(&envelope.payload.unicode_version)?;

    if locked_version != expected_version {
        return Err(VersionError::Invalid(format!(
            "lock Unicode version {} does not match expected {}",
            locked_version, expected_version
        )));
    }

    if expected_version != (UnicodeVersion {
        major: 15,
        minor: 1,
        patch: 0,
    }) {
        return Err(VersionError::Invalid(format!(
            "this validator build is bound to Unicode 15.1.0, not {}",
            expected_version
        )));
    }

    let expected_url = format!(
        "https://www.unicode.org/Public/{}/ucd/",
        expected_version
    );

    if envelope.payload.unicode_release_url != expected_url {
        return Err(VersionError::Invalid(format!(
            "Unicode release URL mismatch: expected {expected_url}, found {}",
            envelope.payload.unicode_release_url
        )));
    }

    let computed_payload_digest = payload_sha256(&envelope.payload)?;
    if computed_payload_digest != envelope.payload_sha256 {
        return Err(VersionError::Invalid(
            "lock payload digest changed after envelope validation".to_owned(),
        ));
    }

    let mut verified_source_files = 0usize;

    for file in &envelope.payload.files {
        verify_locked_file(root, file)?;
        verified_source_files = verified_source_files
            .checked_add(1)
            .ok_or_else(|| {
                VersionError::Invalid("source file count overflow".to_owned())
            })?;
    }

    let mut verified_derived_artifacts = 0usize;

    for artifact in &envelope.payload.derived_artifacts {
        verify_derived_artifact(root, artifact, &envelope.payload_sha256)?;
        verified_derived_artifacts = verified_derived_artifacts
            .checked_add(1)
            .ok_or_else(|| {
                VersionError::Invalid("artifact count overflow".to_owned())
            })?;
    }

    let cargo_lock_path = root.join("Cargo.lock");
    let cargo_lock_bytes = fs::read(&cargo_lock_path).map_err(|error| {
        VersionError::Invalid(format!(
            "cannot read required {}: {error}",
            cargo_lock_path.display()
        ))
    })?;

    let cargo_lock_digest = sha256_hex(&cargo_lock_bytes);
    if cargo_lock_digest != envelope.payload.normalizer.cargo_lock_sha256 {
        return Err(VersionError::Invalid(format!(
            "Cargo.lock SHA-256 mismatch: expected {}, found {}",
            envelope.payload.normalizer.cargo_lock_sha256,
            cargo_lock_digest
        )));
    }

    Ok(VersionBindingReport {
        version: expected_version,
        verified_source_files,
        verified_derived_artifacts,
        cargo_lock_verified: true,
    })
}

/// Verify one source file by exact byte length and SHA-256.
pub fn verify_locked_file(
    root: impl AsRef<Path>,
    file: &LockedFile,
) -> Result<(), VersionError> {
    verify_path_bytes(
        root.as_ref(),
        &file.path,
        file.size_bytes,
        &file.sha256,
        "locked source file",
    )
}

fn verify_derived_artifact(
    root: &Path,
    artifact: &DerivedArtifact,
    expected_payload_digest: &str,
) -> Result<(), VersionError> {
    if artifact.input_payload_sha256 != expected_payload_digest {
        return Err(VersionError::Invalid(format!(
            "derived artifact {} is bound to a different lock payload",
            artifact.path
        )));
    }

    verify_path_bytes(
        root,
        &artifact.path,
        artifact.size_bytes,
        &artifact.sha256,
        "derived artifact",
    )
}

fn verify_path_bytes(
    root: &Path,
    relative_path: &str,
    expected_size: u64,
    expected_sha256: &str,
    description: &str,
) -> Result<(), VersionError> {
    let path = safe_join(root, relative_path)?;
    let bytes = fs::read(&path).map_err(|error| {
        VersionError::Invalid(format!(
            "cannot read {description} {}: {error}",
            path.display()
        ))
    })?;

    let actual_size = u64::try_from(bytes.len()).map_err(|_| {
        VersionError::Invalid(format!(
            "{description} size cannot be represented: {}",
            path.display()
        ))
    })?;

    if actual_size != expected_size {
        return Err(VersionError::Invalid(format!(
            "{description} size mismatch for {}: expected {}, found {}",
            path.display(),
            expected_size,
            actual_size
        )));
    }

    let actual_sha256 = sha256_hex(&bytes);
    if actual_sha256 != expected_sha256 {
        return Err(VersionError::Invalid(format!(
            "{description} SHA-256 mismatch for {}: expected {}, found {}",
            path.display(),
            expected_sha256,
            actual_sha256
        )));
    }

    Ok(())
}

fn safe_join(root: &Path, relative: &str) -> Result<PathBuf, VersionError> {
    let relative_path = Path::new(relative);

    if relative.is_empty()
        || relative.starts_with('/')
        || relative.contains('\\')
        || relative_path
            .components()
            .any(|component| !matches!(component, std::path::Component::Normal(_)))
    {
        return Err(VersionError::Invalid(format!(
            "unsafe relative path in lock: {relative}"
        )));
    }

    Ok(root.join(relative_path))
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut output = String::with_capacity(64);

    for byte in digest {
        use std::fmt::Write;
        let _ = write!(&mut output, "{byte:02x}");
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_exact_unicode_version() {
        let parsed =
            UnicodeVersion::parse("15.1.0").expect("valid test version");

        assert_eq!(parsed.major, 15);
        assert_eq!(parsed.minor, 1);
        assert_eq!(parsed.patch, 0);
        assert_eq!(parsed.to_string(), "15.1.0");
    }

    #[test]
    fn rejects_incomplete_version() {
        assert!(UnicodeVersion::parse("15.1").is_err());
    }

    #[test]
    fn rejects_leading_zero_component() {
        assert!(UnicodeVersion::parse("015.1.0").is_err());
    }

    #[test]
    fn rejects_non_numeric_component() {
        assert!(UnicodeVersion::parse("15.x.0").is_err());
    }

    #[test]
    fn rejects_path_traversal() {
        let result = safe_join(Path::new("/tmp/root"), "../outside.txt");
        assert!(result.is_err());
    }

    #[test]
    fn verifies_exact_bytes_and_hash() {
        let root = std::env::temp_dir().join(format!(
            "d-ucd-version-test-{}",
            std::process::id()
        ));

        std::fs::create_dir_all(&root)
            .expect("test directory should be creatable");

        let content = b"tiny fixture\n";
        std::fs::write(root.join("fixture.txt"), content)
            .expect("test fixture should be writable");

        let file = LockedFile {
            path: "fixture.txt".to_owned(),
            sha256: sha256_hex(content),
            size_bytes: content.len() as u64,
            role: "test-fixture".to_owned(),
        };

        assert!(verify_locked_file(&root, &file).is_ok());

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn rejects_byte_length_mismatch() {
        let root = std::env::temp_dir().join(format!(
            "d-ucd-version-size-test-{}",
            std::process::id()
        ));

        std::fs::create_dir_all(&root)
            .expect("test directory should be creatable");

        std::fs::write(root.join("fixture.txt"), b"abc")
            .expect("test fixture should be writable");

        let file = LockedFile {
            path: "fixture.txt".to_owned(),
            sha256: sha256_hex(b"abc"),
            size_bytes: 4,
            role: "test-fixture".to_owned(),
        };

        assert!(verify_locked_file(&root, &file).is_err());

        let _ = std::fs::remove_dir_all(&root);
    }
}
