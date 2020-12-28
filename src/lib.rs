#![doc = "Lucent's mathematical core and request interface."]

pub mod mp;
mod wire;

pub use wire::request;

pub mod calculus;
pub mod differentiate;
pub mod eval;
pub mod exact;
pub mod graph;
pub mod interval;
mod job;
pub mod lang;
pub mod linear;
pub mod polynomial;
pub mod program;
pub mod value;
