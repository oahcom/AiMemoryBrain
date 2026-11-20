//! NFC/NFD validation against the pinned Unicode 15.1.0 conformance corpus.

use crate::ucd::NormalizationTestRecord;

/// Requested normalization form.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NormalizationForm {
    Nfc,
    Nfd,
}

/// One mismatch, retaining enough provenance to reproduce the failure.
#[derive(Debug, Clone)]
pub struct NormalizationFailure {
    pub source_line: u64,
    pub form: NormalizationForm,
    pub input: String,
    pub expected: String,
    pub actual: String,
}

/// Validate one string against the selected normalization form.
///
/// Contract: compare exact Unicode scalar sequence; do not compare rendered
/// glyphs, locale collation, or lossy replacement-decoded strings.
pub fn verify_one(
    input: &str,
    form: NormalizationForm,
    expected: &str,
) -> Result<(), NormalizationFailure> {
    todo!("Step 2: implement exact NFC/NFD comparison")
}

/// Execute the complete UCD NormalizationTest corpus.
///
/// Contract: implement the normative c1..c5 normalization relations for
/// NFC and NFD as specified by Unicode 15.1.0. Count every consumed record;
/// any mismatch or skipped malformed row fails the suite.
pub fn verify_full_corpus(
    records: &[NormalizationTestRecord],
) -> Result<CorpusReport, CorpusError> {
    todo!("Step 2: implement full UCD normalization conformance")
}

#[derive(Debug, Clone)]
pub struct CorpusReport {
    pub records_consumed: u64,
    pub nfc_assertions: u64,
    pub nfd_assertions: u64,
}

#[derive(Debug)]
pub enum CorpusError {
    Mismatch(NormalizationFailure),
    Incomplete { expected_minimum: u64, actual: u64 },
    InvalidCorpus(String),
}
