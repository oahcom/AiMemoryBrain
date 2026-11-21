//! D dual-high-water proof gate for F-43-01/F1.
//!
//! The six-condition evaluator is a pure predicate over supplied evidence.
//! It does not authenticate that evidence or establish an operational D source.
//! `require_operational_source` and `require_evidence_gate` remain the separate,
//! fail-closed operational boundary. Evaluate one namespace per evidence value.

use crate::authority::DAuthoritativeSource;
use crate::receipt::{ReceiptSample, validate_operational_receipt};
use std::collections::{BTreeMap, BTreeSet};

/// Legacy source snapshot shape retained for the authority adapter boundary.
/// It is not accepted as a substitute for `SixConditionEvidence`.
#[derive(Debug, Clone)]
pub struct DHighWaterSnapshot {
    pub authoritative_high_water: u64,
    pub observed_high_water: u64,
    pub source_sequence: u64,
    pub linearization_token: String,
    pub provenance: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HighWaterParseError;

impl std::fmt::Display for HighWaterParseError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(
            "high-water/generation must be an unsigned ASCII decimal integer in 0..=2^63-1",
        )
    }
}

impl std::error::Error for HighWaterParseError {}

macro_rules! define_high_water_type {
    ($name:ident) => {
        /// An independent nominal decimal high-water/generation domain.
        ///
        /// No ordering or cross-type conversion traits are implemented.
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub struct $name(u64);

        impl $name {
            pub const MAX: u64 = i64::MAX as u64;

            pub const fn new(value: u64) -> Option<Self> {
                if value <= Self::MAX {
                    Some(Self(value))
                } else {
                    None
                }
            }

            pub const fn get(self) -> u64 {
                self.0
            }
        }

        impl TryFrom<&str> for $name {
            type Error = HighWaterParseError;

            fn try_from(value: &str) -> Result<Self, Self::Error> {
                if value.is_empty()
                    || !value.bytes().all(|byte| byte.is_ascii_digit())
                {
                    return Err(HighWaterParseError);
                }

                let parsed = value.parse::<u64>().map_err(|_| HighWaterParseError)?;
                Self::new(parsed).ok_or(HighWaterParseError)
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(formatter, "{}", self.0)
            }
        }
    };
}

define_high_water_type!(ProducerRegistrationHighWater);
define_high_water_type!(ForgetFenceGeneration);
define_high_water_type!(ProducerWorkItemHighWater);
define_high_water_type!(ProducerWorkAttemptHighWater);
define_high_water_type!(BarrierGeneration);

/// A lowercase SHA-256 digest bound to the barrier root field.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct BarrierRootDigest(String);

/// A lowercase SHA-256 digest bound to the dependency-closure root field.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DependencyClosureRootDigest(String);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DigestParseError;

impl std::fmt::Display for DigestParseError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("digest must be exactly 64 lowercase hexadecimal characters")
    }
}

impl std::error::Error for DigestParseError {}

fn validate_sha256_hex(value: &str) -> Result<(), DigestParseError> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(DigestParseError);
    }
    Ok(())
}

macro_rules! define_digest_type {
    ($name:ident) => {
        impl TryFrom<&str> for $name {
            type Error = DigestParseError;

            fn try_from(value: &str) -> Result<Self, Self::Error> {
                validate_sha256_hex(value)?;
                Ok(Self(value.to_owned()))
            }
        }

        impl $name {
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str(&self.0)
            }
        }
    };
}

define_digest_type!(BarrierRootDigest);
define_digest_type!(DependencyClosureRootDigest);

/// Opaque UTF-8 cursor bytes. This type deliberately has no numeric parser.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DependencyClosureCursor(Vec<u8>);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CursorParseError;

impl std::fmt::Display for CursorParseError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("dependency closure cursor must be non-empty opaque UTF-8")
    }
}

impl std::error::Error for CursorParseError {}

impl TryFrom<&str> for DependencyClosureCursor {
    type Error = CursorParseError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        if value.is_empty()
            || value.bytes().all(|byte| byte.is_ascii_digit())
        {
            return Err(CursorParseError);
        }
        Ok(Self(value.as_bytes().to_vec()))
    }
}

impl DependencyClosureCursor {
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    pub fn as_str(&self) -> &str {
        // Construction accepts only Rust &str, so these bytes are valid UTF-8.
        std::str::from_utf8(&self.0).expect("cursor bytes originate from valid UTF-8")
    }
}

impl std::fmt::Display for DependencyClosureCursor {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// Exact immutable receipt binding for one H field.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DFieldBinding {
    pub field_name: String,
    pub nominal_type_id: String,
    pub authority_id: String,
    pub namespace_id: String,
    pub lineage_id: Option<String>,
    pub canonical_value: String,
    pub source_checkpoint_ref: String,
    pub source_checkpoint_digest: String,
    pub snapshot_receipt_digest: String,
    pub binding_version: String,
}

/// Caller-supplied authoritative identity expectations for a single receipt.
///
/// Namespace and lineage expectations are keyed by the exact H field spelling.
/// `None` lineage means that the field is not lineage-scoped.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DFieldBindingRequirements {
    pub authority_id: String,
    pub namespaces: BTreeMap<String, String>,
    pub lineages: BTreeMap<String, Option<String>>,
    pub snapshot_receipt_digest: String,
    pub accepted_binding_versions: BTreeSet<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DFieldBindingError {
    WrongFieldCount { expected: usize, actual: usize },
    MissingField(String),
    DuplicateField(String),
    UnknownField(String),
    MissingExpectation(String),
    WrongNominalType { field: String, expected: String, actual: String },
    WrongAuthority(String),
    WrongNamespace(String),
    WrongLineage(String),
    WrongSnapshotDigest(String),
    InvalidCanonicalValue(String),
    MissingCheckpointReference(String),
    InvalidCheckpointDigest(String),
    UnsupportedBindingVersion(String),
    InvalidRequirements(String),
}

impl std::fmt::Display for DFieldBindingError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for DFieldBindingError {}

const REQUIRED_D_FIELD_TYPES: [(&str, &str); 8] = [
    ("producer_registration_high_water", "ProducerRegistrationHighWater"),
    ("forget_fence_generation_high_water", "ForgetFenceGeneration"),
    ("work_item_high_water", "ProducerWorkItemHighWater"),
    ("attempt_high_water", "ProducerWorkAttemptHighWater"),
    ("barrier_generation", "BarrierGeneration"),
    ("barrier_root", "BarrierRootDigest"),
    ("dependency_closure_cursor", "DependencyClosureCursor"),
    ("dependency_closure_root", "DependencyClosureRootDigest"),
];

fn validate_bound_value(
    field_name: &str,
    nominal_type_id: &str,
    value: &str,
) -> Result<(), DFieldBindingError> {
    let valid = match (field_name, nominal_type_id) {
        ("producer_registration_high_water", "ProducerRegistrationHighWater") => {
            ProducerRegistrationHighWater::try_from(value)
                .is_ok_and(|parsed| parsed.to_string() == value)
        }
        ("forget_fence_generation_high_water", "ForgetFenceGeneration") => {
            ForgetFenceGeneration::try_from(value)
                .is_ok_and(|parsed| parsed.to_string() == value)
        }
        ("work_item_high_water", "ProducerWorkItemHighWater") => {
            ProducerWorkItemHighWater::try_from(value)
                .is_ok_and(|parsed| parsed.to_string() == value)
        }
        ("attempt_high_water", "ProducerWorkAttemptHighWater") => {
            ProducerWorkAttemptHighWater::try_from(value)
                .is_ok_and(|parsed| parsed.to_string() == value)
        }
        ("barrier_generation", "BarrierGeneration") => {
            BarrierGeneration::try_from(value)
                .is_ok_and(|parsed| parsed.to_string() == value)
        }
        ("barrier_root", "BarrierRootDigest") => {
            BarrierRootDigest::try_from(value).is_ok()
        }
        ("dependency_closure_cursor", "DependencyClosureCursor") => {
            DependencyClosureCursor::try_from(value).is_ok()
        }
        ("dependency_closure_root", "DependencyClosureRootDigest") => {
            DependencyClosureRootDigest::try_from(value).is_ok()
        }
        _ => {
            return Err(DFieldBindingError::WrongNominalType {
                field: field_name.to_owned(),
                expected: REQUIRED_D_FIELD_TYPES
                    .iter()
                    .find(|(field, _)| *field == field_name)
                    .map(|(_, nominal)| (*nominal).to_owned())
                    .unwrap_or_else(|| "unknown field".to_owned()),
                actual: nominal_type_id.to_owned(),
            });
        }
    };

    if valid {
        Ok(())
    } else {
        Err(DFieldBindingError::InvalidCanonicalValue(
            field_name.to_owned(),
        ))
    }
}

/// Validate the closed-world eight-field binding collection.
///
/// Both the exact field name and its nominal type identifier select the value
/// parser. A syntactically valid value for another nominal domain is rejected.
pub fn validate_field_bindings(
    bindings: &[DFieldBinding],
    requirements: &DFieldBindingRequirements,
) -> Result<(), DFieldBindingError> {
    if bindings.len() != REQUIRED_D_FIELD_TYPES.len() {
        return Err(DFieldBindingError::WrongFieldCount {
            expected: REQUIRED_D_FIELD_TYPES.len(),
            actual: bindings.len(),
        });
    }

    if requirements.authority_id.trim().is_empty()
        || requirements.snapshot_receipt_digest.trim().is_empty()
        || requirements.accepted_binding_versions.is_empty()
    {
        return Err(DFieldBindingError::InvalidRequirements(
            "authority, snapshot digest, and accepted versions are required".to_owned(),
        ));
    }

    validate_sha256_hex(&requirements.snapshot_receipt_digest)
        .map_err(|_| DFieldBindingError::InvalidRequirements(
            "snapshot receipt digest is not lowercase SHA-256".to_owned(),
        ))?;

    let mut seen = BTreeSet::new();

    for binding in bindings {
        if !REQUIRED_D_FIELD_TYPES
            .iter()
            .any(|(field, _)| *field == binding.field_name)
        {
            return Err(DFieldBindingError::UnknownField(
                binding.field_name.clone(),
            ));
        }

        if !seen.insert(binding.field_name.as_str()) {
            return Err(DFieldBindingError::DuplicateField(
                binding.field_name.clone(),
            ));
        }

        let expected_type = REQUIRED_D_FIELD_TYPES
            .iter()
            .find(|(field, _)| *field == binding.field_name)
            .map(|(_, nominal)| *nominal)
            .ok_or_else(|| DFieldBindingError::UnknownField(binding.field_name.clone()))?;

        if binding.nominal_type_id != expected_type {
            return Err(DFieldBindingError::WrongNominalType {
                field: binding.field_name.clone(),
                expected: expected_type.to_owned(),
                actual: binding.nominal_type_id.clone(),
            });
        }

        if binding.authority_id != requirements.authority_id {
            return Err(DFieldBindingError::WrongAuthority(
                binding.field_name.clone(),
            ));
        }

        let expected_namespace = requirements
            .namespaces
            .get(&binding.field_name)
            .ok_or_else(|| DFieldBindingError::MissingExpectation(
                format!("namespace for {}", binding.field_name),
            ))?;
        if binding.namespace_id != *expected_namespace {
            return Err(DFieldBindingError::WrongNamespace(
                binding.field_name.clone(),
            ));
        }

        let expected_lineage = requirements
            .lineages
            .get(&binding.field_name)
            .ok_or_else(|| DFieldBindingError::MissingExpectation(
                format!("lineage for {}", binding.field_name),
            ))?;
        if &binding.lineage_id != expected_lineage {
            return Err(DFieldBindingError::WrongLineage(
                binding.field_name.clone(),
            ));
        }

        if binding.snapshot_receipt_digest != requirements.snapshot_receipt_digest {
            return Err(DFieldBindingError::WrongSnapshotDigest(
                binding.field_name.clone(),
            ));
        }

        if !requirements
            .accepted_binding_versions
            .contains(&binding.binding_version)
        {
            return Err(DFieldBindingError::UnsupportedBindingVersion(
                binding.binding_version.clone(),
            ));
        }

        if binding.source_checkpoint_ref.trim().is_empty() {
            return Err(DFieldBindingError::MissingCheckpointReference(
                binding.field_name.clone(),
            ));
        }

        validate_sha256_hex(&binding.source_checkpoint_digest)
            .map_err(|_| DFieldBindingError::InvalidCheckpointDigest(
                binding.field_name.clone(),
            ))?;

        validate_bound_value(
            &binding.field_name,
            &binding.nominal_type_id,
            &binding.canonical_value,
        )?;
    }

    for (field_name, _) in REQUIRED_D_FIELD_TYPES {
        if !seen.contains(field_name) {
            return Err(DFieldBindingError::MissingField(
                field_name.to_owned(),
            ));
        }
    }

    Ok(())
}

/// One registration considered by the D serialization proof.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreFenceRegistration {
    pub sequence: u64,
    pub accepted: bool,
    pub included_in_barrier: bool,
}

/// The only permitted post-fence outcomes for an ordinary covered registration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PostFenceDisposition {
    Rejected,
    Quarantined,
    CleanupOnly,
    Published,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PostFenceRegistration {
    pub disposition: PostFenceDisposition,
}

/// Serialization-order evidence for one namespace and one fence linearization.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SerializationProof {
    pub fence_linearization_reference: String,
    /// True only when the source proves both lists are exhaustive for this fence.
    pub ordering_is_exhaustive: bool,
    pub pre_fence_registrations: Vec<PreFenceRegistration>,
    pub post_fence_registrations: Vec<PostFenceRegistration>,
}

/// All fields are optional so absence is distinct from a legitimate zero value.
///
/// This structure represents one namespace only. Multi-namespace operations
/// must construct and evaluate an independent value for each namespace; callers
/// must not aggregate or substitute high-water values across namespaces.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SixConditionEvidence {
    // Condition 1: authority, ledger identity/namespace, and registry version.
    pub brain_authority_id: Option<String>,
    pub producer_registration_ledger_identity: Option<String>,
    pub producer_registration_ledger_namespace: Option<String>,
    pub producer_registry_version: Option<String>,

    // Condition 2: immutable operation identity and exact request/target digests.
    pub forget_operation_id: Option<String>,
    pub idempotency_identity: Option<String>,
    pub canonical_request_digest: Option<String>,
    pub target_set_digest: Option<String>,

    // Condition 3: fence identity and installation commit cursor.
    pub forget_fence_cursor: Option<String>,
    pub forget_fence_generation: Option<ForgetFenceGeneration>,
    pub installation_canonical_commit_cursor: Option<String>,
    pub immutable_snapshot_id: Option<String>,

    // Condition 4: same nominal type, same immutable snapshot, equality only.
    pub captured_producer_registration_high_water: Option<ProducerRegistrationHighWater>,
    pub barrier_snapshot_producer_registration_high_water: Option<ProducerRegistrationHighWater>,

    // Condition 5: barrier receipt binds the complete identity set and proof.
    pub barrier_brain_authority_id: Option<String>,
    pub barrier_producer_registration_ledger_identity: Option<String>,
    pub barrier_producer_registration_ledger_namespace: Option<String>,
    pub barrier_producer_registry_version: Option<String>,
    pub barrier_forget_operation_id: Option<String>,
    pub barrier_idempotency_identity: Option<String>,
    pub barrier_canonical_request_digest: Option<String>,
    pub barrier_target_set_digest: Option<String>,
    pub barrier_forget_fence_cursor: Option<String>,
    pub barrier_forget_fence_generation: Option<ForgetFenceGeneration>,
    pub barrier_installation_canonical_commit_cursor: Option<String>,
    pub barrier_immutable_snapshot_id: Option<String>,
    pub ledger_consistency_proof_reference: Option<String>,

    // Condition 6: authoritative serialization-order proof.
    pub forget_fence_linearization_reference: Option<String>,
    pub serialization_proof: Option<SerializationProof>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SixConditionVerdict {
    Valid,
    Invalid(String),
    Stale(String),
    Unknown(String),
}

/// Pure six-condition predicate. This does not establish source authenticity,
/// currentness, or revocation status; those remain independent operational gates.
///
/// Explicit contradictions take precedence over missing evidence. A complete
/// result is therefore necessary, but not sufficient, for operational proof.
pub fn evaluate_six_conditions(evidence: &SixConditionEvidence) -> SixConditionVerdict {
    macro_rules! mismatch {
        ($left:expr, $right:expr, $label:literal) => {
            if let (Some(left), Some(right)) = (&$left, &$right) {
                if left != right {
                    return SixConditionVerdict::Invalid(
                        concat!("explicit mismatch: ", $label).to_owned(),
                    );
                }
            }
        };
    }

    // Check all available explicit identity/operation/cursor/snapshot mismatches
    // before classifying any absent evidence as UNKNOWN.
    mismatch!(
        evidence.brain_authority_id,
        evidence.barrier_brain_authority_id,
        "BrainAuthorityId"
    );
    mismatch!(
        evidence.producer_registration_ledger_identity,
        evidence.barrier_producer_registration_ledger_identity,
        "ProducerRegistrationLedger identity"
    );
    mismatch!(
        evidence.producer_registration_ledger_namespace,
        evidence.barrier_producer_registration_ledger_namespace,
        "ProducerRegistrationLedger namespace"
    );
    mismatch!(
        evidence.producer_registry_version,
        evidence.barrier_producer_registry_version,
        "ProducerRegistryVersion"
    );
    mismatch!(
        evidence.forget_operation_id,
        evidence.barrier_forget_operation_id,
        "ForgetOperationId"
    );
    mismatch!(
        evidence.idempotency_identity,
        evidence.barrier_idempotency_identity,
        "idempotency identity"
    );
    mismatch!(
        evidence.canonical_request_digest,
        evidence.barrier_canonical_request_digest,
        "canonical request digest"
    );
    mismatch!(
        evidence.target_set_digest,
        evidence.barrier_target_set_digest,
        "target-set digest"
    );
    mismatch!(
        evidence.forget_fence_cursor,
        evidence.barrier_forget_fence_cursor,
        "ForgetFenceCursor"
    );
    mismatch!(
        evidence.forget_fence_generation,
        evidence.barrier_forget_fence_generation,
        "ForgetFenceGeneration"
    );
    mismatch!(
        evidence.installation_canonical_commit_cursor,
        evidence.barrier_installation_canonical_commit_cursor,
        "installation CanonicalCommitCursor"
    );
    mismatch!(
        evidence.immutable_snapshot_id,
        evidence.barrier_immutable_snapshot_id,
        "immutable snapshot identity"
    );
    mismatch!(
        evidence.forget_fence_linearization_reference,
        evidence.serialization_proof.as_ref().map(|proof| proof.fence_linearization_reference.clone()),
        "fence linearization reference"
    );

    if let (
        Some(captured),
        Some(barrier),
    ) = (
        evidence.captured_producer_registration_high_water,
        evidence.barrier_snapshot_producer_registration_high_water,
    ) {
        if captured != barrier {
            return SixConditionVerdict::Invalid(
                "captured and barrier-snapshot ProducerRegistrationHighWater differ".to_owned(),
            );
        }
    }

    if let (Some(high_water), Some(proof)) = (
        evidence.barrier_snapshot_producer_registration_high_water,
        evidence.serialization_proof.as_ref(),
    ) {
        for registration in &proof.pre_fence_registrations {
            if registration.accepted
                && (registration.sequence > high_water.get()
                    || !registration.included_in_barrier)
            {
                return SixConditionVerdict::Invalid(
                    "pre-fence accepted registration is above H or absent from barrier".to_owned(),
                );
            }
        }
        if proof
            .post_fence_registrations
            .iter()
            .any(|registration| registration.disposition == PostFenceDisposition::Published)
        {
            return SixConditionVerdict::Invalid(
                "ordinary post-fence registration was published".to_owned(),
            );
        }
    }

    // No missing field may be replaced by a default or sentinel.
    let required_strings = [
        evidence.brain_authority_id.as_ref(),
        evidence.producer_registration_ledger_identity.as_ref(),
        evidence.producer_registration_ledger_namespace.as_ref(),
        evidence.producer_registry_version.as_ref(),
        evidence.forget_operation_id.as_ref(),
        evidence.idempotency_identity.as_ref(),
        evidence.canonical_request_digest.as_ref(),
        evidence.target_set_digest.as_ref(),
        evidence.forget_fence_cursor.as_ref(),
        evidence.installation_canonical_commit_cursor.as_ref(),
        evidence.barrier_brain_authority_id.as_ref(),
        evidence.barrier_producer_registration_ledger_identity.as_ref(),
        evidence.barrier_producer_registration_ledger_namespace.as_ref(),
        evidence.barrier_producer_registry_version.as_ref(),
        evidence.barrier_forget_operation_id.as_ref(),
        evidence.barrier_idempotency_identity.as_ref(),
        evidence.barrier_canonical_request_digest.as_ref(),
        evidence.barrier_target_set_digest.as_ref(),
        evidence.barrier_forget_fence_cursor.as_ref(),
        evidence.barrier_installation_canonical_commit_cursor.as_ref(),
        evidence.immutable_snapshot_id.as_ref(),
        evidence.barrier_immutable_snapshot_id.as_ref(),
        evidence.ledger_consistency_proof_reference.as_ref(),
        evidence.forget_fence_linearization_reference.as_ref(),
    ];
    if required_strings
        .iter()
        .any(|field| field.is_none_or(|value| value.trim().is_empty()))
    {
        return SixConditionVerdict::Unknown(
            "one or more identity, digest, cursor, snapshot, or proof fields are missing or blank"
                .to_owned(),
        );
    }

    let Some(generation) = evidence.forget_fence_generation else {
        return SixConditionVerdict::Unknown("ForgetFenceGeneration is missing".to_owned());
    };
    if evidence.barrier_forget_fence_generation.is_none() {
        return SixConditionVerdict::Unknown(
            "barrier-bound ForgetFenceGeneration is missing".to_owned(),
        );
    }
    let _ = generation;

    if evidence.captured_producer_registration_high_water.is_none()
        || evidence
            .barrier_snapshot_producer_registration_high_water
            .is_none()
    {
        return SixConditionVerdict::Unknown(
            "one or both typed ProducerRegistrationHighWater cutoffs are missing".to_owned(),
        );
    }

    let Some(proof_reference) = evidence.ledger_consistency_proof_reference.as_ref() else {
        return SixConditionVerdict::Unknown("ledger consistency proof is missing".to_owned());
    };
    if proof_reference.trim().is_empty() {
        return SixConditionVerdict::Unknown("ledger consistency proof is unavailable".to_owned());
    }

    let Some(serialization_proof) = evidence.serialization_proof.as_ref() else {
        return SixConditionVerdict::Unknown("serialization-order proof is missing".to_owned());
    };
    if serialization_proof.fence_linearization_reference.trim().is_empty() {
        return SixConditionVerdict::Unknown(
            "serialization proof lacks a fence linearization reference".to_owned(),
        );
    }
    if !serialization_proof.ordering_is_exhaustive {
        return SixConditionVerdict::Unknown(
            "serialization proof does not establish exhaustive ordering coverage".to_owned(),
        );
    }

    // Empty registration lists are not proof that ordering was checked.
    if serialization_proof.pre_fence_registrations.is_empty()
        && serialization_proof.post_fence_registrations.is_empty()
    {
        return SixConditionVerdict::Unknown(
            "serialization proof contains no ordering observations".to_owned(),
        );
    }

    SixConditionVerdict::Valid
}

/// The canonical D endpoint, trust root, transport, and operational key
/// registry are unspecified. A public trait implementation cannot self-attest.
pub fn require_operational_source(_source: &dyn DAuthoritativeSource) -> Result<(), DProofError> {
    Err(DProofError::SourceUnavailable(
        "canonical D-authoritative endpoint and trust configuration are not specified".to_owned(),
    ))
}

pub fn require_evidence_gate(
    source: Option<&dyn DAuthoritativeSource>,
    receipt: Option<&ReceiptSample>,
) -> Result<(), DProofError> {
    let source = source.ok_or_else(|| {
        DProofError::SourceUnavailable("D-authoritative source is not configured".to_owned())
    })?;
    let receipt = receipt.ok_or(DProofError::MissingReceipt)?;
    validate_operational_receipt(receipt).map_err(|error| match error {
        crate::receipt::ReceiptError::MissingProvenance => DProofError::MissingProvenance,
        other => DProofError::PredicateFailed(format!("invalid receipt: {other:?}")),
    })?;
    require_operational_source(source)
}

#[derive(Debug)]
pub enum DProofError {
    SourceUnavailable(String),
    TestDoubleOnly,
    MissingReceipt,
    MissingProvenance,
    PredicateFailed(String),
    MissingScenarioEvidence(String),
    ContractMappingIncomplete(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn complete_evidence() -> SixConditionEvidence {
        let mut evidence = SixConditionEvidence::default();
        evidence.brain_authority_id = Some("authority-A".into());
        evidence.producer_registration_ledger_identity = Some("ledger-A".into());
        evidence.producer_registration_ledger_namespace = Some("namespace-A".into());
        evidence.producer_registry_version = Some("registry-v7".into());
        evidence.forget_operation_id = Some("forget-1".into());
        evidence.idempotency_identity = Some("idem-1".into());
        evidence.canonical_request_digest = Some("request-digest".into());
        evidence.target_set_digest = Some("target-digest".into());
        evidence.forget_fence_cursor = Some("fence-cursor-8".into());
        evidence.forget_fence_generation = ForgetFenceGeneration::new(8);
        evidence.installation_canonical_commit_cursor = Some("commit-cursor-9".into());
        evidence.immutable_snapshot_id = Some("snapshot-1".into());
        evidence.captured_producer_registration_high_water =
            ProducerRegistrationHighWater::new(5);
        evidence.barrier_snapshot_producer_registration_high_water =
            ProducerRegistrationHighWater::new(5);
        evidence.barrier_brain_authority_id = Some("authority-A".into());
        evidence.barrier_producer_registration_ledger_identity = Some("ledger-A".into());
        evidence.barrier_producer_registration_ledger_namespace = Some("namespace-A".into());
        evidence.barrier_producer_registry_version = Some("registry-v7".into());
        evidence.barrier_forget_operation_id = Some("forget-1".into());
        evidence.barrier_idempotency_identity = Some("idem-1".into());
        evidence.barrier_canonical_request_digest = Some("request-digest".into());
        evidence.barrier_target_set_digest = Some("target-digest".into());
        evidence.barrier_forget_fence_cursor = Some("fence-cursor-8".into());
        evidence.barrier_forget_fence_generation = ForgetFenceGeneration::new(8);
        evidence.barrier_installation_canonical_commit_cursor =
            Some("commit-cursor-9".into());
        evidence.barrier_immutable_snapshot_id = Some("snapshot-1".into());
        evidence.ledger_consistency_proof_reference = Some("ledger-proof-1".into());
        evidence.forget_fence_linearization_reference = Some("linearization-8".into());
        evidence.serialization_proof = Some(SerializationProof {
            fence_linearization_reference: "linearization-8".into(),
            ordering_is_exhaustive: true,
            pre_fence_registrations: vec![PreFenceRegistration {
                sequence: 5,
                accepted: true,
                included_in_barrier: true,
            }],
            post_fence_registrations: vec![PostFenceRegistration {
                disposition: PostFenceDisposition::Rejected,
            }],
        });
        evidence
    }

    #[test]
    fn all_six_conditions_complete_return_valid() {
        assert_eq!(
            evaluate_six_conditions(&complete_evidence()),
            SixConditionVerdict::Valid
        );
    }

    #[test]
    fn captured_high_water_greater_than_barrier_is_invalid() {
        let mut evidence = complete_evidence();
        evidence.captured_producer_registration_high_water =
            ProducerRegistrationHighWater::new(6);
        assert!(matches!(
            evaluate_six_conditions(&evidence),
            SixConditionVerdict::Invalid(_)
        ));
    }

    #[test]
    fn captured_high_water_less_than_barrier_is_invalid() {
        let mut evidence = complete_evidence();
        evidence.captured_producer_registration_high_water =
            ProducerRegistrationHighWater::new(4);
        assert!(matches!(
            evaluate_six_conditions(&evidence),
            SixConditionVerdict::Invalid(_)
        ));
    }

    #[test]
    fn missing_captured_high_water_is_unknown() {
        let mut evidence = complete_evidence();
        evidence.captured_producer_registration_high_water = None;
        assert!(matches!(
            evaluate_six_conditions(&evidence),
            SixConditionVerdict::Unknown(_)
        ));
    }

    #[test]
    fn missing_barrier_high_water_is_unknown() {
        let mut evidence = complete_evidence();
        evidence.barrier_snapshot_producer_registration_high_water = None;
        assert!(matches!(
            evaluate_six_conditions(&evidence),
            SixConditionVerdict::Unknown(_)
        ));
    }

    #[test]
    fn target_set_digest_mismatch_is_invalid() {
        let mut evidence = complete_evidence();
        evidence.barrier_target_set_digest = Some("different-targets".into());
        assert!(matches!(
            evaluate_six_conditions(&evidence),
            SixConditionVerdict::Invalid(_)
        ));
    }

    #[test]
    fn fence_generation_mismatch_is_invalid() {
        let mut evidence = complete_evidence();
        evidence.barrier_forget_fence_generation = ForgetFenceGeneration::new(9);
        assert!(matches!(
            evaluate_six_conditions(&evidence),
            SixConditionVerdict::Invalid(_)
        ));
    }

    #[test]
    fn missing_serialization_proof_is_unknown() {
        let mut evidence = complete_evidence();
        evidence.serialization_proof = None;
        assert!(matches!(
            evaluate_six_conditions(&evidence),
            SixConditionVerdict::Unknown(_)
        ));
    }

    #[test]
    fn partial_first_three_conditions_are_unknown() {
        let mut evidence = SixConditionEvidence::default();
        evidence.brain_authority_id = Some("authority-A".into());
        evidence.producer_registration_ledger_identity = Some("ledger-A".into());
        evidence.producer_registration_ledger_namespace = Some("namespace-A".into());
        evidence.producer_registry_version = Some("registry-v7".into());
        evidence.forget_operation_id = Some("forget-1".into());
        evidence.idempotency_identity = Some("idem-1".into());
        evidence.canonical_request_digest = Some("request-digest".into());
        evidence.target_set_digest = Some("target-digest".into());
        evidence.forget_fence_cursor = Some("fence-cursor-8".into());
        evidence.forget_fence_generation = ForgetFenceGeneration::new(8);
        evidence.installation_canonical_commit_cursor = Some("commit-cursor-9".into());
        assert!(matches!(
            evaluate_six_conditions(&evidence),
            SixConditionVerdict::Unknown(_)
        ));
    }

    #[test]
    fn explicit_mismatch_precedes_other_missing_evidence() {
        let mut evidence = SixConditionEvidence::default();
        evidence.target_set_digest = Some("left".into());
        evidence.barrier_target_set_digest = Some("right".into());
        assert!(matches!(
            evaluate_six_conditions(&evidence),
            SixConditionVerdict::Invalid(_)
        ));
    }

    #[test]
    fn accepted_pre_fence_registration_above_h_is_invalid() {
        let mut evidence = complete_evidence();
        evidence.serialization_proof.as_mut().unwrap().pre_fence_registrations[0].sequence = 6;
        assert!(matches!(
            evaluate_six_conditions(&evidence),
            SixConditionVerdict::Invalid(_)
        ));
    }

    #[test]
    fn accepted_pre_fence_registration_missing_from_barrier_is_invalid() {
        let mut evidence = complete_evidence();
        evidence.serialization_proof.as_mut().unwrap().pre_fence_registrations[0]
            .included_in_barrier = false;
        assert!(matches!(
            evaluate_six_conditions(&evidence),
            SixConditionVerdict::Invalid(_)
        ));
    }

    #[test]
    fn published_post_fence_registration_is_invalid() {
        let mut evidence = complete_evidence();
        evidence.serialization_proof.as_mut().unwrap().post_fence_registrations[0].disposition =
            PostFenceDisposition::Published;
        assert!(matches!(
            evaluate_six_conditions(&evidence),
            SixConditionVerdict::Invalid(_)
        ));
    }

    #[test]
    fn empty_serialization_observations_are_unknown() {
        let mut evidence = complete_evidence();
        evidence.serialization_proof.as_mut().unwrap().pre_fence_registrations.clear();
        evidence.serialization_proof.as_mut().unwrap().post_fence_registrations.clear();
        assert!(matches!(
            evaluate_six_conditions(&evidence),
            SixConditionVerdict::Unknown(_)
        ));
    }

    #[test]
    fn plus_sign_is_rejected() {
        assert!(ProducerRegistrationHighWater::try_from("+5").is_err());
    }

    #[test]
    fn fractional_value_is_rejected() {
        assert!(ProducerRegistrationHighWater::try_from("5.0").is_err());
    }

    #[test]
    fn leading_whitespace_is_rejected() {
        assert!(ProducerRegistrationHighWater::try_from(" 5").is_err());
    }

    #[test]
    fn trailing_whitespace_is_rejected() {
        assert!(ProducerRegistrationHighWater::try_from("5 ").is_err());
    }

    #[test]
    fn exponent_notation_is_rejected() {
        assert!(ProducerRegistrationHighWater::try_from("1e3").is_err());
    }

    #[test]
    fn digit_separator_is_rejected() {
        assert!(ProducerRegistrationHighWater::try_from("5,000").is_err());
    }

    #[test]
    fn negative_value_is_rejected() {
        assert!(ProducerRegistrationHighWater::try_from("-5").is_err());
    }

    #[test]
    fn leading_zeroes_are_accepted_and_canonicalized_by_value() {
        let padded = ProducerRegistrationHighWater::try_from("0005").unwrap();
        let canonical = ProducerRegistrationHighWater::try_from("5").unwrap();
        assert_eq!(padded, canonical);
    }

    #[test]
    fn two_to_the_sixty_third_is_rejected() {
        assert!(ProducerRegistrationHighWater::try_from("9223372036854775808").is_err());
    }

    #[test]
    fn maximum_signed_sixty_four_bit_value_is_accepted() {
        assert_eq!(
            ProducerRegistrationHighWater::try_from("9223372036854775807")
                .unwrap()
                .get(),
            i64::MAX as u64
        );
    }

    #[test]
    fn zero_is_a_valid_high_water_value() {
        assert_eq!(
            ProducerRegistrationHighWater::try_from("0").unwrap().get(),
            0
        );
    }

    #[test]
    fn empty_high_water_is_rejected() {
        assert!(ProducerRegistrationHighWater::try_from("").is_err());
    }

    fn binding_requirements() -> DFieldBindingRequirements {
        let mut namespaces = BTreeMap::new();
        let mut lineages = BTreeMap::new();
        for (field, _) in REQUIRED_D_FIELD_TYPES {
            namespaces.insert(field.to_owned(), format!("namespace-{field}"));
            lineages.insert(
                field.to_owned(),
                if field.starts_with("dependency_closure_") {
                    Some("lineage-dependency-v1".to_owned())
                } else {
                    None
                },
            );
        }
        DFieldBindingRequirements {
            authority_id: "authority-D".to_owned(),
            namespaces,
            lineages,
            snapshot_receipt_digest: "a".repeat(64),
            accepted_binding_versions: BTreeSet::from(["D-contract-1.3.1".to_owned()]),
        }
    }

    fn complete_bindings() -> Vec<DFieldBinding> {
        REQUIRED_D_FIELD_TYPES
            .iter()
            .map(|(field, nominal)| {
                let canonical_value = match *field {
                    "producer_registration_high_water"
                    | "forget_fence_generation_high_water"
                    | "work_item_high_water"
                    | "attempt_high_water"
                    | "barrier_generation" => "7".to_owned(),
                    "barrier_root" | "dependency_closure_root" => "b".repeat(64),
                    "dependency_closure_cursor" => "cursor:opaque:v1".to_owned(),
                    _ => String::new(),
                };
                DFieldBinding {
                    field_name: (*field).to_owned(),
                    nominal_type_id: (*nominal).to_owned(),
                    authority_id: "authority-D".to_owned(),
                    namespace_id: format!("namespace-{field}"),
                    lineage_id: if field.starts_with("dependency_closure_") {
                        Some("lineage-dependency-v1".to_owned())
                    } else {
                        None
                    },
                    canonical_value,
                    source_checkpoint_ref: format!("checkpoint-{field}"),
                    source_checkpoint_digest: "c".repeat(64),
                    snapshot_receipt_digest: "a".repeat(64),
                    binding_version: "D-contract-1.3.1".to_owned(),
                }
            })
            .collect()
    }

    #[test]
    fn all_eight_nominal_bindings_are_accepted() {
        assert!(validate_field_bindings(&complete_bindings(), &binding_requirements()).is_ok());
    }

    #[test]
    fn every_high_water_type_accepts_leading_zeroes_and_canonicalizes() {
        assert_eq!(ProducerRegistrationHighWater::try_from("0007").unwrap().to_string(), "7");
        assert_eq!(ForgetFenceGeneration::try_from("0007").unwrap().to_string(), "7");
        assert_eq!(ProducerWorkItemHighWater::try_from("0007").unwrap().to_string(), "7");
        assert_eq!(ProducerWorkAttemptHighWater::try_from("0007").unwrap().to_string(), "7");
        assert_eq!(BarrierGeneration::try_from("0007").unwrap().to_string(), "7");
    }

    #[test]
    fn high_water_maximum_signed_value_is_accepted() {
        assert_eq!(
            BarrierGeneration::try_from("9223372036854775807").unwrap().get(),
            i64::MAX as u64
        );
    }

    #[test]
    fn high_water_above_maximum_is_rejected() {
        assert!(ProducerWorkItemHighWater::try_from("9223372036854775808").is_err());
    }

    #[test]
    fn empty_high_water_is_rejected_for_each_nominal_domain() {
        assert!(ForgetFenceGeneration::try_from("").is_err());
        assert!(ProducerWorkAttemptHighWater::try_from("").is_err());
    }

    #[test]
    fn unicode_decimal_digits_are_rejected() {
        assert!(BarrierGeneration::try_from("１２３").is_err());
    }

    #[test]
    fn negative_high_water_is_rejected_for_new_domains() {
        assert!(ProducerWorkItemHighWater::try_from("-1").is_err());
    }

    #[test]
    fn distinct_nominal_types_do_not_implement_cross_type_equality() {
        // Compile-time contract: ProducerRegistrationHighWater and
        // ForgetFenceGeneration have no cross-type PartialEq implementation.
        // Consequently `registration == generation` does not type-check.
        let registration = ProducerRegistrationHighWater::new(1).unwrap();
        let generation = ForgetFenceGeneration::new(1).unwrap();
        assert_eq!(registration.get(), 1);
        assert_eq!(generation.get(), 1);
    }

    #[test]
    fn missing_one_of_eight_fields_is_rejected() {
        let mut bindings = complete_bindings();
        bindings.pop();
        assert!(matches!(
            validate_field_bindings(&bindings, &binding_requirements()),
            Err(DFieldBindingError::WrongFieldCount { .. })
        ));
    }

    #[test]
    fn nine_fields_are_rejected() {
        let mut bindings = complete_bindings();
        bindings.push(bindings[0].clone());
        assert!(matches!(
            validate_field_bindings(&bindings, &binding_requirements()),
            Err(DFieldBindingError::WrongFieldCount { .. })
        ));
    }

    #[test]
    fn duplicate_field_is_rejected() {
        let mut bindings = complete_bindings();
        bindings[1].field_name = bindings[0].field_name.clone();
        assert!(matches!(
            validate_field_bindings(&bindings, &binding_requirements()),
            Err(DFieldBindingError::DuplicateField(_))
        ));
    }

    #[test]
    fn unknown_field_name_is_rejected() {
        let mut bindings = complete_bindings();
        bindings[0].field_name = "unexpected_high_water".to_owned();
        assert!(matches!(
            validate_field_bindings(&bindings, &binding_requirements()),
            Err(DFieldBindingError::UnknownField(_))
        ));
    }

    #[test]
    fn wrong_nominal_type_identifier_is_rejected() {
        let mut bindings = complete_bindings();
        bindings[0].nominal_type_id = "ForgetFenceGeneration".to_owned();
        assert!(matches!(
            validate_field_bindings(&bindings, &binding_requirements()),
            Err(DFieldBindingError::WrongNominalType { .. })
        ));
    }

    #[test]
    fn hexadecimal_value_in_decimal_field_is_rejected() {
        let mut bindings = complete_bindings();
        bindings[0].canonical_value = "b".repeat(64);
        assert!(matches!(
            validate_field_bindings(&bindings, &binding_requirements()),
            Err(DFieldBindingError::InvalidCanonicalValue(_))
        ));
    }

    #[test]
    fn decimal_value_in_cursor_field_is_rejected() {
        let mut bindings = complete_bindings();
        let cursor = bindings
            .iter_mut()
            .find(|binding| binding.field_name == "dependency_closure_cursor")
            .unwrap();
        cursor.canonical_value = "12345".to_owned();
        assert!(matches!(
            validate_field_bindings(&bindings, &binding_requirements()),
            Err(DFieldBindingError::InvalidCanonicalValue(_))
        ));
    }

    #[test]
    fn cursor_text_in_high_water_field_is_rejected() {
        let mut bindings = complete_bindings();
        bindings[0].canonical_value = "cursor:opaque:v1".to_owned();
        assert!(matches!(
            validate_field_bindings(&bindings, &binding_requirements()),
            Err(DFieldBindingError::InvalidCanonicalValue(_))
        ));
    }

    #[test]
    fn unsupported_binding_version_is_rejected() {
        let mut bindings = complete_bindings();
        bindings[0].binding_version = "D-contract-0.9".to_owned();
        assert!(matches!(
            validate_field_bindings(&bindings, &binding_requirements()),
            Err(DFieldBindingError::UnsupportedBindingVersion(_))
        ));
    }

    #[test]
    fn wrong_authority_is_rejected() {
        let mut bindings = complete_bindings();
        bindings[0].authority_id = "authority-other".to_owned();
        assert!(matches!(
            validate_field_bindings(&bindings, &binding_requirements()),
            Err(DFieldBindingError::WrongAuthority(_))
        ));
    }

    #[test]
    fn wrong_namespace_is_rejected() {
        let mut bindings = complete_bindings();
        bindings[0].namespace_id = "namespace-other".to_owned();
        assert!(matches!(
            validate_field_bindings(&bindings, &binding_requirements()),
            Err(DFieldBindingError::WrongNamespace(_))
        ));
    }

    #[test]
    fn wrong_lineage_is_rejected() {
        let mut bindings = complete_bindings();
        let cursor = bindings
            .iter_mut()
            .find(|binding| binding.field_name == "dependency_closure_cursor")
            .unwrap();
        cursor.lineage_id = Some("lineage-other".to_owned());
        assert!(matches!(
            validate_field_bindings(&bindings, &binding_requirements()),
            Err(DFieldBindingError::WrongLineage(_))
        ));
    }

    #[test]
    fn wrong_snapshot_digest_is_rejected() {
        let mut bindings = complete_bindings();
        bindings[0].snapshot_receipt_digest = "d".repeat(64);
        assert!(matches!(
            validate_field_bindings(&bindings, &binding_requirements()),
            Err(DFieldBindingError::WrongSnapshotDigest(_))
        ));
    }
}
