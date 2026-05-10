//! Convenience functions for formatting values for output.
//!
//! Currently only contains a `format_num` function for formatting numbers, which ensures that
//! integers are displayed with a ".0" suffix when `./rlox parse` is run. Note that in `evaluate`
//! and `run` modes, integer values will be printed without the ".0" suffix.

/// When emitting number tokens, format integers with a ".0" suffix to match
/// the expected output format.
pub fn format_num(value: f64) -> String {
    if value % 1.0 == 0.0 {
        format!("{:.1}", value)
    } else {
        value.to_string()
    }
}
