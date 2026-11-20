//! NFC/NFD validation against the pinned Unicode 15.1.0 conformance corpus.

use crate::ucd::NormalizationTestRecord;
use unicode_normalization::UnicodeNormalization;

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
    let actual: String = match form {
        NormalizationForm::Nfc => input.nfc().collect(),
        NormalizationForm::Nfd => input.nfd().collect(),
    };

    if actual == expected {
        return Ok(());
    }

    Err(NormalizationFailure {
        source_line: 0,
        form,
        input: input.to_owned(),
        expected: expected.to_owned(),
        actual,
    })
}

/// Minimum number of records accepted as a complete Unicode 15.1.0 corpus.
///
/// The official normalization corpus contains substantially more than this
/// conservative floor. The floor detects empty, truncated, or obviously
/// incomplete inputs; it is not a substitute for verifying the locked
/// source-file digest.
const MINIMUM_CORPUS_RECORDS: u64 = 10_000;

/// Execute the complete UCD NormalizationTest corpus.
///
/// Contract: implement the normative c1..c5 normalization relations for
/// NFC and NFD as specified by Unicode 15.1.0. Count every consumed record;
/// any mismatch or skipped malformed row fails the suite.
pub fn verify_full_corpus(
    records: &[NormalizationTestRecord],
) -> Result<CorpusReport, CorpusError> {
    verify_corpus_with_minimum(records, MINIMUM_CORPUS_RECORDS)
}

fn verify_corpus_with_minimum(
    records: &[NormalizationTestRecord],
    minimum_records: u64,
) -> Result<CorpusReport, CorpusError> {
    let actual_records = u64::try_from(records.len()).unwrap_or(u64::MAX);

    if actual_records < minimum_records {
        return Err(CorpusError::Incomplete {
            expected_minimum: minimum_records,
            actual: actual_records,
        });
    }

    let mut report = CorpusReport {
        records_consumed: 0,
        nfc_assertions: 0,
        nfd_assertions: 0,
    };

    for record in records {
        let mut columns: [String; 5] = std::array::from_fn(|_| String::new());

        for (column_index, code_points) in record.columns.iter().enumerate() {
            let mut value = String::new();

            for &code_point in code_points {
                let Some(scalar) = char::from_u32(code_point) else {
                    return Err(CorpusError::InvalidCorpus(format!(
                        "record {} at source line {} contains invalid Unicode scalar U+{:04X} in column c{}",
                        record.source_record,
                        record.source_line,
                        code_point,
                        column_index + 1
                    )));
                };

                value.push(scalar);
            }

            columns[column_index] = value;
        }

        let source_line = u64::try_from(record.source_line).unwrap_or(u64::MAX);

        // Unicode 15.1.0 NormalizationTest.txt NFC relations:
        // c1 -> c2, c2 -> c2, c3 -> c2, c4 -> c4, c5 -> c4.
        let nfc_assertions = [
            (&columns[0], &columns[1]),
            (&columns[1], &columns[1]),
            (&columns[2], &columns[1]),
            (&columns[3], &columns[3]),
            (&columns[4], &columns[3]),
        ];

        for (input, expected) in nfc_assertions {
            verify_record_assertion(
                input,
                NormalizationForm::Nfc,
                expected,
                source_line,
            )?;
            report.nfc_assertions += 1;
        }

        // Unicode 15.1.0 NormalizationTest.txt NFD relations:
        // c1 -> c3, c2 -> c3, c3 -> c3, c4 -> c5, c5 -> c5.
        let nfd_assertions = [
            (&columns[0], &columns[2]),
            (&columns[1], &columns[2]),
            (&columns[2], &columns[2]),
            (&columns[3], &columns[4]),
            (&columns[4], &columns[4]),
        ];

        for (input, expected) in nfd_assertions {
            verify_record_assertion(
                input,
                NormalizationForm::Nfd,
                expected,
                source_line,
            )?;
            report.nfd_assertions += 1;
        }

        report.records_consumed += 1;
    }

    Ok(report)
}

fn verify_record_assertion(
    input: &str,
    form: NormalizationForm,
    expected: &str,
    source_line: u64,
) -> Result<(), CorpusError> {
    verify_one(input, form, expected).map_err(|mut failure| {
        failure.source_line = source_line;
        CorpusError::Mismatch(failure)
    })
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

#[cfg(test)]
mod tests {
    use super::*;

    fn ascii_record() -> NormalizationTestRecord {
        NormalizationTestRecord {
            source_line: 1,
            source_record: 1,
            columns: [
                vec![0x0041],
                vec![0x0041],
                vec![0x0041],
                vec![0x0041],
                vec![0x0041],
            ],
        }
    }

    fn combining_mark_record() -> NormalizationTestRecord {
        // LATIN CAPITAL LETTER A WITH RING ABOVE:
        // NFC is U+00C5; NFD is U+0041 U+030A.
        NormalizationTestRecord {
            source_line: 2,
            source_record: 2,
            columns: [
                vec![0x00C5],
                vec![0x00C5],
                vec![0x0041, 0x030A],
                vec![0x00C5],
                vec![0x0041, 0x030A],
            ],
        }
    }

    #[test]
    fn known_good_ascii_and_combining_mark_relations_pass() {
        let records = [ascii_record(), combining_mark_record()];

        let report = verify_corpus_with_minimum(&records, 2)
            .expect("the valid ASCII and combining-mark fixtures must pass");

        assert_eq!(report.records_consumed, 2);
        assert_eq!(report.nfc_assertions, 10);
        assert_eq!(report.nfd_assertions, 10);
    }

    #[test]
    fn deliberately_wrong_expected_value_returns_mismatch() {
        let mut record = ascii_record();

        // c1 is "A", but c2 is deliberately changed to "B".
        // The first NFC relation must therefore fail.
        record.columns[1] = vec![0x0042];

        let result = verify_corpus_with_minimum(&[record], 1);

        match result {
            Err(CorpusError::Mismatch(failure)) => {
                assert_eq!(failure.source_line, 1);
                assert_eq!(failure.input, "A");
                assert_eq!(failure.expected, "B");
                assert_eq!(failure.actual, "A");
                assert!(matches!(failure.form, NormalizationForm::Nfc));
            }
            _ => panic!("a deliberately incorrect expected value must yield Mismatch"),
        }
    }

    #[test]
    fn empty_corpus_returns_incomplete() {
        match verify_full_corpus(&[]) {
            Err(CorpusError::Incomplete {
                expected_minimum,
                actual,
            }) => {
                assert_eq!(expected_minimum, MINIMUM_CORPUS_RECORDS);
                assert_eq!(actual, 0);
            }
            _ => panic!("an empty corpus must yield Incomplete"),
        }
    }

    #[test]
    fn verify_one_compares_exact_normalized_scalar_sequences() {
        assert!(verify_one(
            "A\u{030A}",
            NormalizationForm::Nfc,
            "\u{00C5}"
        )
        .is_ok());

        assert!(verify_one(
            "\u{00C5}",
            NormalizationForm::Nfd,
            "A\u{030A}"
        )
        .is_ok());
    }
}
