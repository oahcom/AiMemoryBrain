// AiMemoryBrain/src/ucd.rs

use crate::lockfile::LockedFile;
use sha2::{Digest, Sha256};
use std::fmt;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnicodeDataRecord {
    /// Zero-based Unicode scalar value parsed from field 0.
    pub code_point: u32,
    /// Original one-based source line number.
    pub source_line: usize,
    /// One-based data-record number, excluding no source lines.
    pub source_record: usize,
    /// All 15 original fields, preserved without normalization.
    pub fields: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NormalizationTestRecord {
    /// Original one-based source line number.
    pub source_line: usize,
    /// One-based data-record number; comments and blank lines are excluded.
    pub source_record: usize,
    /// The five normative columns c1 through c5.
    pub columns: [Vec<u32>; 5],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifiedBytes {
    pub size_bytes: u64,
    pub sha256: String,
}

#[derive(Debug)]
pub enum UcdError {
    Io(std::io::Error),
    Invalid(String),
}

impl fmt::Display for UcdError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(f, "UCD I/O error: {error}"),
            Self::Invalid(message) => write!(f, "invalid UCD data: {message}"),
        }
    }
}

impl std::error::Error for UcdError {}

impl From<std::io::Error> for UcdError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

/// Read a locked file and verify its exact byte length and SHA-256.
pub fn read_verified_file(
    root: impl AsRef<Path>,
    locked: &LockedFile,
) -> Result<Vec<u8>, UcdError> {
    validate_locked_path(&locked.path)?;

    let path = root.as_ref().join(&locked.path);
    let bytes = fs::read(&path)?;

    verify_bytes(&bytes, locked)?;

    Ok(bytes)
}

/// Verify exact bytes. Hashing is performed on the original byte sequence;
/// no newline, encoding, or text normalization is applied.
pub fn verify_bytes(
    bytes: &[u8],
    locked: &LockedFile,
) -> Result<VerifiedBytes, UcdError> {
    let actual_size = u64::try_from(bytes.len()).map_err(|_| {
        UcdError::Invalid("input byte length cannot be represented as u64".to_owned())
    })?;

    if actual_size != locked.size_bytes {
        return Err(UcdError::Invalid(format!(
            "byte length mismatch for {}: expected {}, found {}",
            locked.path, locked.size_bytes, actual_size
        )));
    }

    let actual_sha256 = sha256_hex(bytes);
    if actual_sha256 != locked.sha256 {
        return Err(UcdError::Invalid(format!(
            "SHA-256 mismatch for {}: expected {}, found {}",
            locked.path, locked.sha256, actual_sha256
        )));
    }

    Ok(VerifiedBytes {
        size_bytes: actual_size,
        sha256: actual_sha256,
    })
}

/// Parse UnicodeData.txt strictly as UTF-8 and exactly 15 fields per record.
///
/// The original field strings, source line, and source record number are
/// retained. This parser does not expand First/Last ranges; those records
/// remain distinct source records for downstream semantic processing.
pub fn parse_unicode_data(
    bytes: &[u8],
) -> Result<Vec<UnicodeDataRecord>, UcdError> {
    let text = std::str::from_utf8(bytes).map_err(|error| {
        UcdError::Invalid(format!(
            "UnicodeData.txt is not valid UTF-8: {error}"
        ))
    })?;

    let mut records = Vec::new();
    let mut data_record = 0usize;

    for (line_index, raw_line) in text.lines().enumerate() {
        let source_line = line_index + 1;
        let line = raw_line.strip_suffix('\r').unwrap_or(raw_line);

        if line.is_empty() {
            return Err(UcdError::Invalid(format!(
                "unexpected empty UnicodeData record at source line {source_line}"
            )));
        }

        if line.contains('\0') {
            return Err(UcdError::Invalid(format!(
                "NUL byte in UnicodeData record at source line {source_line}"
            )));
        }

        let fields: Vec<String> = line.split(';').map(str::to_owned).collect();

        if fields.len() != 15 {
            return Err(UcdError::Invalid(format!(
                "UnicodeData record at line {source_line} has {} fields; expected 15",
                fields.len()
            )));
        }

        let code_point = parse_code_point(&fields[0]).map_err(|message| {
            UcdError::Invalid(format!(
                "UnicodeData line {source_line}: {message}"
            ))
        })?;

        if fields[1].is_empty() {
            return Err(UcdError::Invalid(format!(
                "UnicodeData line {source_line}: character name is empty"
            )));
        }

        validate_unicode_scalar(code_point).map_err(|message| {
            UcdError::Invalid(format!(
                "UnicodeData line {source_line}: {message}"
            ))
        })?;

        data_record = data_record.checked_add(1).ok_or_else(|| {
            UcdError::Invalid("UnicodeData record counter overflow".to_owned())
        })?;

        records.push(UnicodeDataRecord {
            code_point,
            source_line,
            source_record: data_record,
            fields,
        });
    }

    if records.is_empty() {
        return Err(UcdError::Invalid(
            "UnicodeData.txt contains no records".to_owned(),
        ));
    }

    Ok(records)
}

/// Parse the complete NormalizationTest.txt data section.
///
/// Blank lines and comment-only lines are ignored. Every non-comment data
/// line must contain exactly five semicolon-delimited columns. Inline
/// comments are removed only after the data portion is isolated. Each
/// returned record retains its original source line and data-record index.
pub fn parse_normalization_test(
    bytes: &[u8],
) -> Result<Vec<NormalizationTestRecord>, UcdError> {
    let text = std::str::from_utf8(bytes).map_err(|error| {
        UcdError::Invalid(format!(
            "NormalizationTest.txt is not valid UTF-8: {error}"
        ))
    })?;

    let mut records = Vec::new();
    let mut data_record = 0usize;

    for (line_index, raw_line) in text.lines().enumerate() {
        let source_line = line_index + 1;
        let line = raw_line.strip_suffix('\r').unwrap_or(raw_line);
        let data = line.split('#').next().unwrap_or("").trim();

        if data.is_empty() || data.starts_with("@Part") {
            continue;
        }

        let mut columns: Vec<&str> = data.split(';').collect();
        if columns.last().is_some_and(|column| column.trim().is_empty()) {
            columns.pop();
        }

        if columns.len() != 5 {
            return Err(UcdError::Invalid(format!(
                "NormalizationTest line {source_line} has {} columns; expected 5",
                columns.len()
            )));
        }

        let parsed = [
            parse_code_point_sequence(columns[0], source_line, 1)?,
            parse_code_point_sequence(columns[1], source_line, 2)?,
            parse_code_point_sequence(columns[2], source_line, 3)?,
            parse_code_point_sequence(columns[3], source_line, 4)?,
            parse_code_point_sequence(columns[4], source_line, 5)?,
        ];

        data_record = data_record.checked_add(1).ok_or_else(|| {
            UcdError::Invalid(
                "NormalizationTest record counter overflow".to_owned(),
            )
        })?;

        records.push(NormalizationTestRecord {
            source_line,
            source_record: data_record,
            columns: parsed,
        });
    }

    if records.is_empty() {
        return Err(UcdError::Invalid(
            "NormalizationTest.txt contains no data records".to_owned(),
        ));
    }

    Ok(records)
}

fn parse_code_point_sequence(
    value: &str,
    source_line: usize,
    column: usize,
) -> Result<Vec<u32>, UcdError> {
    let trimmed = value.trim();

    if trimmed.is_empty() {
        // Empty sequences are representable in the file format.
        return Ok(Vec::new());
    }

    let mut result = Vec::new();

    for token in trimmed.split_ascii_whitespace() {
        let code_point = parse_code_point(token).map_err(|message| {
            UcdError::Invalid(format!(
                "NormalizationTest line {source_line}, column {column}: {message}"
            ))
        })?;

        validate_unicode_scalar(code_point).map_err(|message| {
            UcdError::Invalid(format!(
                "NormalizationTest line {source_line}, column {column}: {message}"
            ))
        })?;

        result.push(code_point);
    }

    Ok(result)
}

fn parse_code_point(value: &str) -> Result<u32, String> {
    if value.is_empty()
        || !value.bytes().all(|byte| byte.is_ascii_hexdigit())
    {
        return Err(format!("invalid hexadecimal code point: {value:?}"));
    }

    u32::from_str_radix(value, 16)
        .map_err(|error| format!("code point out of range {value:?}: {error}"))
}

fn validate_unicode_scalar(value: u32) -> Result<(), String> {
    if value > 0x10_FFFF {
        return Err(format!("code point U+{value:X} exceeds U+10FFFF"));
    }

    if (0xD800..=0xDFFF).contains(&value) {
        return Err(format!(
            "U+{value:04X} is a surrogate, not a Unicode scalar value"
        ));
    }

    Ok(())
}

fn validate_locked_path(value: &str) -> Result<(), UcdError> {
    let path = Path::new(value);

    if value.is_empty()
        || value.starts_with('/')
        || value.contains('\\')
        || path
            .components()
            .any(|component| !matches!(component, std::path::Component::Normal(_)))
    {
        return Err(UcdError::Invalid(format!(
            "unsafe locked relative path: {value}"
        )));
    }

    Ok(())
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

    fn locked_for(path: &str, bytes: &[u8]) -> LockedFile {
        LockedFile {
            path: path.to_owned(),
            sha256: sha256_hex(bytes),
            size_bytes: bytes.len() as u64,
            role: "test-fixture".to_owned(),
        }
    }

    #[test]
    fn verifies_byte_exact_hash_and_size() {
        let bytes = b"fixture\r\n";
        let locked = locked_for("fixture.txt", bytes);

        assert!(verify_bytes(bytes, &locked).is_ok());
        assert!(verify_bytes(b"fixture\n", &locked).is_err());
    }

    #[test]
    fn parses_unicode_data_and_preserves_source_identity() {
        let fixture = concat!(
            "0041;LATIN CAPITAL LETTER A;Lu;0;L;;;;;N;;;;0061;\n",
            "0061;LATIN SMALL LETTER A;Ll;0;L;;;;;N;;;0041;;0041\n"
        );

        let records = parse_unicode_data(fixture.as_bytes())
            .expect("small UnicodeData fixture should parse");

        assert_eq!(records.len(), 2);
        assert_eq!(records[0].code_point, 0x41);
        assert_eq!(records[0].source_line, 1);
        assert_eq!(records[0].source_record, 1);
        assert_eq!(records[0].fields.len(), 15);
        assert_eq!(records[1].source_line, 2);
    }

    #[test]
    fn rejects_unicode_data_wrong_field_count() {
        let fixture = "0041;LATIN CAPITAL LETTER A;Lu\n";
        assert!(parse_unicode_data(fixture.as_bytes()).is_err());
    }

    #[test]
    fn rejects_unicode_data_surrogate_code_point() {
        let fixture =
            "D800;SURROGATE;Cs;0;L;;;;;N;;;;;\n";
        assert!(parse_unicode_data(fixture.as_bytes()).is_err());
    }

    #[test]
    fn parses_normalization_rows_and_skips_comments() {
        let fixture = concat!(
            "# NormalizationTest fixture\n",
            "\n",
            "0041;0041;0041;0041;0041 # LATIN CAPITAL LETTER A\n",
            "0061;0061;0061;0061;0061\n"
        );

        let records = parse_normalization_test(fixture.as_bytes())
            .expect("small normalization fixture should parse");

        assert_eq!(records.len(), 2);
        assert_eq!(records[0].source_line, 3);
        assert_eq!(records[0].source_record, 1);
        assert_eq!(records[0].columns[0], vec![0x41]);
        assert_eq!(records[1].source_line, 4);
    }

    #[test]
    fn rejects_normalization_row_with_wrong_column_count() {
        let fixture = "0041;0041;0041;0041\n";
        assert!(parse_normalization_test(fixture.as_bytes()).is_err());
    }

    #[test]
    fn rejects_normalization_surrogate_scalar() {
        let fixture = "D800;0041;0041;0041;0041\n";
        assert!(parse_normalization_test(fixture.as_bytes()).is_err());
    }

    #[test]
    fn rejects_invalid_utf8() {
        let fixture = [0xFF, 0xFE, 0xFD];
        assert!(parse_unicode_data(&fixture).is_err());
        assert!(parse_normalization_test(&fixture).is_err());
    }
}
