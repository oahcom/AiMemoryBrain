//! D-contract Unicode validator.
//!
//! Canonical authority for this crate's D validator/build artifacts:
//! D Owner, limited to D-contract-1.3.1.
//!
//! This crate does not confer production acceptance, production signing,
//! main-merge authority, or distributed-system conformance.

pub mod authority;
pub mod d_predicates;
pub mod lockfile;
pub mod normalization;
pub mod receipt;
pub mod ucd;
pub mod version;

pub use lockfile::{UcdLockEnvelope, UcdLockPayload};
pub use version::{UnicodeVersion, VersionBindingReport};
