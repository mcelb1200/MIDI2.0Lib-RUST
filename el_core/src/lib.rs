#![no_std]
#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![deny(clippy::alloc_instead_of_core)]

pub mod ump;
pub mod utils;

#[cfg(test)]
mod tests_ump;

#[cfg(test)]
mod tests_utils;
