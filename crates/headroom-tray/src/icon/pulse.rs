use std::f64::consts::TAU;
use std::time::Duration;

use super::canvas::to_byte;
use super::layout::FULL_OPACITY;

pub const FRAME: Duration = Duration::from_millis(125);
pub const FRAMES_PER_PERIOD: u32 = 16;
const DIM_OPACITY: f64 = 140.0;

#[must_use]
pub fn opacity(frame: u32) -> u8 {
    let phase = f64::from(frame % FRAMES_PER_PERIOD) / f64::from(FRAMES_PER_PERIOD);
    let lift = f64::midpoint(1.0, (TAU * phase).cos());
    let full = f64::from(FULL_OPACITY);
    to_byte((DIM_OPACITY + (full - DIM_OPACITY) * lift) / full)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pulses_between_full_and_dim_over_two_seconds() {
        assert_eq!(FRAME * FRAMES_PER_PERIOD, Duration::from_secs(2));
        assert_eq!(opacity(0), 255);
        assert_eq!(opacity(FRAMES_PER_PERIOD / 2), 140);
        assert_eq!(opacity(FRAMES_PER_PERIOD), 255);
        assert_eq!(opacity(3), opacity(FRAMES_PER_PERIOD - 3));
        assert!(opacity(4) < opacity(2));
    }

    #[test]
    fn a_period_has_few_distinct_frames_to_cache() {
        let mut levels: Vec<u8> = (0..FRAMES_PER_PERIOD).map(opacity).collect();
        levels.sort_unstable();
        levels.dedup();
        assert_eq!(levels.len(), 9);
    }
}
