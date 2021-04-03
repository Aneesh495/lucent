#![doc = "Lucent's mathematical core and request interface."]

pub mod mp;
mod wire;

pub use wire::{request, verify_packet};

pub mod calculus;
pub mod coloring;
pub mod differentiate;
pub mod dual;
pub mod eval;
pub mod exact;
pub mod floating;
pub mod graph;
pub mod improper;
pub mod interval;
mod job;
pub mod lang;
pub mod linear;
pub mod ode;
pub mod polynomial;
pub mod program;
pub mod quadrature;
pub mod simplify;
pub mod spatial;
pub mod statistics;
pub mod value;
