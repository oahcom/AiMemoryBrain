use d_unicode_validator::authority::{AuthorityError, AuthorityIdentity, DAuthoritativeSource};
use d_unicode_validator::d_predicates::{DHighWaterSnapshot, DProofError, require_evidence_gate};
use d_unicode_validator::receipt::ReceiptSample;

struct Fixture;
impl DAuthoritativeSource for Fixture {
    fn identity(&self) -> AuthorityIdentity {
        AuthorityIdentity {
            system: "fixture".into(),
            endpoint_or_adapter: "memory".into(),
            version: "test".into(),
            provenance: "synthetic".into(),
        }
    }
    fn read_authoritative_snapshot(&self) -> Result<DHighWaterSnapshot, AuthorityError> {
        Ok(DHighWaterSnapshot {
            authoritative_high_water: 7,
            observed_high_water: 7,
            source_sequence: 7,
            linearization_token: "fixture".into(),
            provenance: "synthetic".into(),
        })
    }
}
fn receipt(fixture: bool) -> ReceiptSample {
    ReceiptSample {
        receipt_id: "r".into(),
        source_system: "s".into(),
        source_sequence: 7,
        observed_high_water: 7,
        linearization_reference: "l".into(),
        provenance_reference: "p".into(),
        is_fixture: fixture,
    }
}

#[test]
#[ignore = "BLOCKED: S01-S10 exact normative-clause mapping, source evidence, and expected results are not present in the accepted proof pack"]
fn all_ten_scenarios_are_defined_and_clause_mapped() {
    // The ten YAML entries are placeholders, not mapped scenarios.
}

#[test]
fn test_double_cannot_satisfy_operational_proof() {
    let source = Fixture;
    assert!(matches!(
        require_evidence_gate(Some(&source), Some(&receipt(false))),
        Err(DProofError::SourceUnavailable(_))
    ));
}

#[test]
fn missing_authority_and_receipt_fail_closed() {
    let source = Fixture;
    let valid_shape = receipt(false);
    assert!(matches!(
        require_evidence_gate(None, Some(&valid_shape)),
        Err(DProofError::SourceUnavailable(_))
    ));
    assert!(matches!(
        require_evidence_gate(Some(&source), None),
        Err(DProofError::MissingReceipt)
    ));
    assert!(matches!(
        require_evidence_gate(Some(&source), Some(&valid_shape)),
        Err(DProofError::SourceUnavailable(_))
    ));
}
