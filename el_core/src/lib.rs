#![no_std]
#![forbid(unsafe_code)]
#![cfg_attr(not(test), deny(clippy::unwrap_used))]
#![cfg_attr(not(test), deny(clippy::expect_used))]
#![cfg_attr(not(test), deny(clippy::panic))]
#![deny(clippy::alloc_instead_of_core)]

pub mod builder;
pub mod parser;
pub mod ump;
pub mod utils;

#[cfg(test)]
mod tests_ump;

#[cfg(test)]
mod tests_utils;

#[cfg(test)]
mod tests_parser;

#[cfg(test)]
mod tests_builder;
