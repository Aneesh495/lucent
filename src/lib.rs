#![doc = "Lucent's mathematical core and request interface."]

pub mod mp;
mod wire;

pub use wire::request;

pub mod interval;
pub mod lang;
