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
fn s1_to_s7_scenarios_are_defined_and_clause_mapped() {
    const SCENARIOS: [(&str, &str); 7] = [
        ("S1", "D-contract-1.3.1 §5.3.8; D-002 §4 S1"),
        ("S2", "D-002 §4 S2; D operation contract §§3.3-3.6, 3.8"),
        ("S3", "D-002 §4 S3; D operation contract §§3.3-3.4, 3.8"),
        ("S4", "D-002 §4 S4; D operation contract §§3.3, 3.8-3.9"),
        ("S5", "D-002 §4 S5; D operation contract §§3.3-3.4, 3.8"),
        ("S6", "D-contract-1.3.1 §5.4; D-002 §4 S6; D operation contract §§3.8-3.9"),
        ("S7", "D-002 §4 S7; D operation contract §§3.3-3.4, 3.9"),
    ];
    let yaml = include_str!("scenarios/f1-dual-high-water-scenarios.yaml");
    assert_eq!(SCENARIOS.len(), 7);
    assert_eq!(yaml.matches("  - id: S").count(), 7);
    assert_eq!(yaml.matches("    normative_clause:").count(), 7);
    assert!(!yaml.contains(concat!("PENDING", "_")));
    for (id, clause) in SCENARIOS {
        assert!(yaml.contains(&format!("  - id: {id}\n")));
        assert!(!clause.trim().is_empty());
        assert!(clause.contains("D-002"));
        assert!(yaml.contains(clause));
    }
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
