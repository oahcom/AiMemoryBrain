use d_unicode_validator::lockfile::read_lock;
use d_unicode_validator::normalization::{verify_full_corpus, CorpusError};
use d_unicode_validator::ucd::{parse_normalization_test, read_verified_file};
use std::path::PathBuf;

const LOCK_RELATIVE_PATH: &str = "ucd/15.1.0/ucd-lock.json";
const CORPUS_ROLE: &str = "normative_normalization_conformance_corpus";

fn format_codepoints(value: &str) -> String {
    value
        .chars()
        .map(|character| format!("U+{:04X}", character as u32))
        .collect::<Vec<_>>()
        .join(" ")
}

#[test]
fn validates_complete_locked_unicode_15_1_0_normalization_corpus() {
    let crate_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let lock_path = crate_root.join(LOCK_RELATIVE_PATH);

    let lock = read_lock(&lock_path).unwrap_or_else(|error| {
        panic!(
            "failed to read Unicode 15.1.0 lock at {}: {error}",
            lock_path.display()
        )
    });

    assert_eq!(
        lock.payload.unicode_version,
        "15.1.0",
        "lock unicode_version mismatch: expected 15.1.0, actual {}",
        lock.payload.unicode_version
    );

    let corpus_entry = lock
        .payload
        .files
        .iter()
        .find(|entry| entry.role == CORPUS_ROLE)
        .unwrap_or_else(|| {
            panic!(
                "lock is missing required files[] entry with role {:?}",
                CORPUS_ROLE
            )
        });

    let corpus_bytes = read_verified_file(&crate_root, corpus_entry)
        .unwrap_or_else(|error| {
            panic!(
                "locked normalization corpus verification failed for {} \
                 (expected size_bytes={}, sha256={}): {error}",
                corpus_entry.path,
                corpus_entry.size_bytes,
                corpus_entry.sha256
            )
        });

    let records = parse_normalization_test(&corpus_bytes).unwrap_or_else(|error| {
        panic!(
            "failed to parse locked normalization corpus {}: {error}",
            corpus_entry.path
        )
    });

    let report = verify_full_corpus(&records).unwrap_or_else(|error| {
        match error {
            CorpusError::Incomplete {
                expected_minimum,
                actual,
            } => panic!(
                "normalization corpus incomplete: expected at least \
                 {expected_minimum} records, actual {actual}"
            ),
            CorpusError::Mismatch(failure) => panic!(
                "normalization mismatch at source_line={} form={:?}: \
                 input=[{}], expected=[{}], actual=[{}]",
                failure.source_line,
                failure.form,
                format_codepoints(&failure.input),
                format_codepoints(&failure.expected),
                format_codepoints(&failure.actual)
            ),
            CorpusError::InvalidCorpus(message) => {
                panic!("normalization corpus invalid: {message}")
            }
        }
    });

    assert!(
        report.records_consumed >= 15_000,
        "normalization report is not substantive: expected records_consumed >= 15000, actual {}",
        report.records_consumed
    );

    let minimum_assertions = report.records_consumed.saturating_mul(5);
    assert!(
        report.nfc_assertions >= minimum_assertions,
        "NFC assertion count insufficient: expected at least {}, actual {}",
        minimum_assertions,
        report.nfc_assertions
    );
    assert!(
        report.nfd_assertions >= minimum_assertions,
        "NFD assertion count insufficient: expected at least {}, actual {}",
        minimum_assertions,
        report.nfd_assertions
    );

    println!(
        "Unicode 15.1.0 normalization conformance PASS: \
         records_consumed={}, nfc_assertions={}, nfd_assertions={}, \
         corpus_path={}, corpus_size_bytes={}, corpus_sha256={}",
        report.records_consumed,
        report.nfc_assertions,
        report.nfd_assertions,
        corpus_entry.path,
        corpus_entry.size_bytes,
        corpus_entry.sha256
    );
}
