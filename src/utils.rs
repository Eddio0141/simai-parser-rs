/// Calculates number of seconds for a length divider
pub fn calc_length_divider(bpm: f64, divider: f64) -> f64 {
    240. / bpm / divider
}
