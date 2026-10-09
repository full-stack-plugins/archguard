//! Optional Git candidate binding and fixture-profile SpecGuard trace integration.
pub mod domain {
    pub mod baseline;
}
pub mod integration {
    pub use archguard::integration::{evidence, projection};
    pub mod binding;
    pub mod trace;
}
