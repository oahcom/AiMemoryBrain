//! Receipt evidence model.
//!
//! A fixture may test parsing and predicate code. It must never be represented
//! as a real D-authoritative Receipt sample.

#[derive(Debug, Clone)]
pub struct ReceiptSample {
    pub receipt_id: String,
    pub source_system: String,
    pub source_sequence: u64,
    pub observed_high_water: u64,
    pub linearization_reference: String,
    pub provenance_reference: String,
    pub is_fixture: bool,
}

/// Validate required fields, provenance, fixture status, and source ordering.
pub fn validate_operational_receipt(receipt: &ReceiptSample) -> Result<(), ReceiptError> {
    if receipt.is_fixture {
        return Err(ReceiptError::FixtureNotOperational);
    }

    if receipt.provenance_reference.trim().is_empty() {
        return Err(ReceiptError::MissingProvenance);
    }
    for (name, value) in [
        ("receipt_id", receipt.receipt_id.as_str()),
        ("source_system", receipt.source_system.as_str()),
        (
            "linearization_reference",
            receipt.linearization_reference.as_str(),
        ),
    ] {
        if value.trim().is_empty() {
            return Err(ReceiptError::MissingField(name.to_owned()));
        }
    }

    if receipt.source_sequence == 0 {
        return Err(ReceiptError::MissingField("source_sequence".to_owned()));
    }
    if receipt.observed_high_water == 0 {
        return Err(ReceiptError::MissingField("observed_high_water".to_owned()));
    }
    if receipt.source_sequence > receipt.observed_high_water {
        return Err(ReceiptError::InvalidProvenance(
            "source_sequence exceeds observed_high_water".to_owned(),
        ));
    }

    Ok(())
}

#[derive(Debug, PartialEq, Eq)]
pub enum ReceiptError {
    MissingField(String),
    MissingProvenance,
    FixtureNotOperational,
    InvalidProvenance(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> ReceiptSample {
        ReceiptSample {
            receipt_id: "receipt-1".to_owned(),
            source_system: "D-ledger".to_owned(),
            source_sequence: 4,
            observed_high_water: 9,
            linearization_reference: "linearization-1".to_owned(),
            provenance_reference: "source-proof-1".to_owned(),
            is_fixture: false,
        }
    }

    #[test]
    fn fixture_receipt_is_rejected() {
        let mut receipt = sample();
        receipt.is_fixture = true;
        assert_eq!(
            validate_operational_receipt(&receipt),
            Err(ReceiptError::FixtureNotOperational)
        );
    }

    #[test]
    fn missing_provenance_is_rejected() {
        let mut receipt = sample();
        receipt.provenance_reference.clear();
        assert_eq!(
            validate_operational_receipt(&receipt),
            Err(ReceiptError::MissingProvenance)
        );
    }

    #[test]
    fn source_sequence_above_observed_high_water_is_rejected() {
        let mut receipt = sample();
        receipt.source_sequence = 10;
        assert!(matches!(
            validate_operational_receipt(&receipt),
            Err(ReceiptError::InvalidProvenance(_))
        ));
    }
}
