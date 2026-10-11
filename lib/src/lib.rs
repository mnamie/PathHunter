//! PathHunter core: PATH scanning, source attribution, and clean planning.
//! Shared by the `ph` CLI and the `ph-gui` front-end. Zero dependencies.

pub mod audit;
pub mod cleanplan;
pub mod expandvars;
pub mod source;
pub mod winenv;
