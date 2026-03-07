#![allow(unused)]

#[cfg(feature = "defmt")]
pub use defmt::{debug, error, info, trace, warn};
#[cfg(feature = "log")]
pub use log::{debug, error, info, trace, warn};

#[cfg(all(feature = "log", feature = "defmt"))]
compile_error!("feature \"log\" and feature \"defmt\" cannot be enabled at the same time");
