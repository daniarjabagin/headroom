use std::f64::consts::PI;

pub const SWEEP_MS: u64 = 1400;
pub const REST_MS: u64 = 5000;
pub const START_DELAY_MS: u64 = 600;
pub const STEP_MS: u64 = 40;
pub const BAND_WIDTH: f64 = 44.0;
const MIN_FRACTION: f64 = 0.03;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SheenBand {
    pub clip_x: f64,
    pub clip_width: f64,
    pub x: f64,
    pub strength: f64,
}

#[must_use]
pub fn shows_sheen(fraction: f64) -> bool {
    fraction >= MIN_FRACTION
}

#[must_use]
pub fn sweep_progress(elapsed_ms: u64) -> Option<f64> {
    (elapsed_ms < SWEEP_MS).then(|| {
        #[allow(
            clippy::cast_precision_loss,
            reason = "elapsed is below the sweep length of a few thousand milliseconds"
        )]
        let linear = elapsed_ms as f64 / SWEEP_MS as f64;
        (1.0 - (PI * linear).cos()) / 2.0
    })
}

fn strength(offset: f64, clip_width: f64) -> f64 {
    let overlap = (offset + BAND_WIDTH).min(clip_width) - offset.max(0.0);
    (overlap / BAND_WIDTH).clamp(0.0, 1.0)
}

#[must_use]
pub fn sheen_band(fill_x: f64, fill_width: f64, track: f64, progress: f64) -> Option<SheenBand> {
    let inset = track / 2.0;
    let clip_width = fill_width - 2.0 * inset;
    if clip_width <= 0.0 {
        return None;
    }
    let offset = -BAND_WIDTH + progress * (clip_width + BAND_WIDTH);
    let strength = strength(offset, clip_width);
    (strength > 0.0).then_some(SheenBand {
        clip_x: fill_x + inset,
        clip_width,
        x: fill_x + inset + offset,
        strength,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_sweep_eases_in_and_out_and_then_rests() {
        assert_eq!(sweep_progress(0), Some(0.0));
        assert!((sweep_progress(SWEEP_MS / 2).unwrap() - 0.5).abs() < 1e-9);
        assert!(sweep_progress(SWEEP_MS - 1).unwrap() > 0.99);
        assert_eq!(sweep_progress(SWEEP_MS), None);
    }

    #[test]
    fn the_cycle_matches_the_other_shells() {
        assert_eq!(SWEEP_MS + REST_MS, 6400);
        assert_eq!(START_DELAY_MS, 600);
        assert_eq!(1000 / STEP_MS, 25);
    }

    #[test]
    fn the_band_crosses_the_inset_fill() {
        let start = sheen_band(10.0, 105.0, 5.0, 0.0);
        assert_eq!(start, None);
        let middle = sheen_band(10.0, 105.0, 5.0, 0.5).unwrap();
        assert!((middle.clip_x - 12.5).abs() < 1e-9);
        assert!((middle.clip_width - 100.0).abs() < 1e-9);
        assert!((middle.x - 40.5).abs() < 1e-9);
        assert!((middle.strength - 1.0).abs() < 1e-9);
        assert_eq!(sheen_band(10.0, 105.0, 5.0, 1.0), None);
    }

    #[test]
    fn the_band_fades_at_the_edges_and_skips_slivers() {
        let entering = sheen_band(0.0, 105.0, 5.0, 0.1).unwrap();
        assert!(entering.strength > 0.0 && entering.strength < 1.0);
        assert_eq!(sheen_band(0.0, 5.0, 5.0, 0.5), None);
        assert!(!shows_sheen(0.029));
        assert!(shows_sheen(0.03));
    }
}
