/// Holds the `string_enum!` macro and the `StringEnum` trait it generates.
///
/// This module is public because the generated trait appears in public impls.
#[macro_use]
pub mod macros;

pub mod date;
pub mod db;
pub mod ui;
