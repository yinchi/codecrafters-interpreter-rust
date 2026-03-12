/// When emitting number tokens, format integers with a ".0" suffix to match
/// the expected output format.
pub fn format_num(value: f64) -> String {
    if value % 1.0 == 0.0 {
        format!("{:.1}", value)
    } else {
        value.to_string()
    }
}
