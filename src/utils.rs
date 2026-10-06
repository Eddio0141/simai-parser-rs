/// Calculates number of seconds for a length divider
pub fn calc_length_divider(bpm: f64, divider: u64) -> f64 {
    240. / bpm / divider as f64
}
