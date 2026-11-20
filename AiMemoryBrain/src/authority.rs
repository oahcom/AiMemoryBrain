//! Injection boundary for the real D-authoritative state source.
//!
//! A test double is useful for testing predicate logic, but it is not
//! operational evidence and cannot close F-43-01/F1.

use crate::d_predicates::DHighWaterSnapshot;

/// Stable identity and provenance of the authoritative D source.
#[derive(Debug, Clone)]
pub struct AuthorityIdentity {
    pub system: String,
    pub endpoint_or_adapter: String,
    pub version: String,
    pub provenance: String,
}

/// Adapter implemented by the real D-contract-1.3.1 authoritative source.
///
/// Implementations must obtain a real authoritative observation. A constant
/// response, unconditional true callback, fixture-only adapter, or local
/// synthetic snapshot is not a production/operational D callback.
pub trait DAuthoritativeSource: Send + Sync {
    fn identity(&self) -> AuthorityIdentity;

    /// Fetch a fresh authoritative snapshot and its source provenance.
    ///
    /// The adapter must expose source ordering/linearization metadata needed
    /// by the D six-condition proof. It must not fabricate missing metadata.
    fn read_authoritative_snapshot(
        &self,
    ) -> Result<DHighWaterSnapshot, AuthorityError>;
}

#[derive(Debug)]
pub enum AuthorityError {
    Unavailable(String),
    Unauthorized(String),
    InvalidResponse(String),
    MissingProvenance(String),
    StaleObservation(String),
}
