//! Architecture-owned semantic models; no language provider is enabled here.
pub mod model;
pub mod rules;

/// Pure architecture revision lifecycle; no Git/specification trace dependency.
pub mod baseline;

pub mod contracts;

pub mod diff;
