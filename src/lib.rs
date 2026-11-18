//! What does `temperature` do while a model is thinking?
//!
//! The answer differs per provider and per model, so it is a table, not a rule. Each model or
//! endpoint is classed as one of four values ([`ThinkingTemperature`]). [`temperature_plan`] turns
//! that class into a decision (send the parameter or leave it out) and records why, so a stored
//! response can show that the setting did nothing.
//!
//! The matrix is data written from provider documentation on 2026-11-15 and re-checked in part on
//! 2026-11-18. Providers change. Re-check before you rely on it.

mod capability;
mod plan;

pub use capability::{
    builtin_capability, endpoint_capability, Capability, ThinkingTemperature, UNKNOWN_SOURCE,
};
pub use plan::{temperature_plan, Target, TemperaturePlan};
