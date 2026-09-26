use headroom_core::pace::Tone;
use headroom_daemon::state::payload::WindowView;

use super::style::Palette;

pub const BAR_CELLS: u8 = 20;
const CELL_PERCENT: f64 = 100.0 / 20.0;
const FILL: &str = "━";
const TRACK_COLORED: &str = "━";
const TRACK_PLAIN: &str = "─";
const TICK: &str = "┃";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BarShape {
    pub filled: u8,
    pub tick: Option<u8>,
}

pub fn shape(window: &WindowView) -> BarShape {
    let filled = cells_below(window.remaining_percent);
    let filled = if filled == 0 && window.remaining_percent > 0.0 {
        1
    } else {
        filled
    };
    let shows_tick = matches!(window.tone, Tone::Warning | Tone::Critical);
    let tick = window
        .pace
        .even_pace_percent
        .filter(|_| shows_tick)
        .map(|even| cells_below(100.0 - even).min(BAR_CELLS - 1));
    BarShape { filled, tick }
}

fn cells_below(percent: f64) -> u8 {
    let count = (0..BAR_CELLS)
        .filter(|&cell| (f64::from(cell) + 0.5) * CELL_PERCENT < percent)
        .count();
    u8::try_from(count).unwrap_or(BAR_CELLS)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Cell {
    Fill,
    Tick,
    Track,
}

pub fn render(window: &WindowView, palette: Palette) -> String {
    let BarShape { filled, tick } = shape(window);
    let cells = (0..BAR_CELLS).map(|cell| match cell {
        _ if tick == Some(cell) => Cell::Tick,
        _ if cell < filled => Cell::Fill,
        _ => Cell::Track,
    });
    let mut runs: Vec<(Cell, usize)> = Vec::new();
    for cell in cells {
        match runs.last_mut() {
            Some((kind, count)) if *kind == cell => *count += 1,
            _ => runs.push((cell, 1)),
        }
    }
    runs.into_iter()
        .map(|(cell, count)| paint_run(cell, count, window, palette))
        .collect()
}

fn paint_run(cell: Cell, count: usize, window: &WindowView, palette: Palette) -> String {
    match cell {
        Cell::Fill => palette.tone(&FILL.repeat(count), window.tone),
        Cell::Tick => TICK.repeat(count),
        Cell::Track if palette.is_colored() => palette.track(&TRACK_COLORED.repeat(count)),
        Cell::Track => TRACK_PLAIN.repeat(count),
    }
}

#[cfg(test)]
mod tests {
    use headroom_core::pace::Severity;
    use headroom_daemon::state::payload::PaceView;

    use super::*;

    fn window(remaining: f64, tone: Tone, even: Option<f64>) -> WindowView {
        WindowView {
            id: "session".into(),
            label: "Session".into(),
            used_percent: 100.0 - remaining,
            remaining_percent: remaining,
            resets_at: None,
            period_seconds: None,
            tone,
            pace: PaceView {
                severity: Severity::Healthy,
                even_pace_percent: even,
                projected_percent: None,
                spare_percent: None,
                runs_out_at: None,
                basis: None,
                active_left_seconds: None,
            },
            hidden: false,
        }
    }

    #[test]
    fn fill_follows_the_remaining_share() {
        assert_eq!(shape(&window(62.0, Tone::Good, None)).filled, 12);
        assert_eq!(shape(&window(100.0, Tone::Good, None)).filled, 20);
        assert_eq!(shape(&window(0.0, Tone::Critical, None)).filled, 0);
    }

    #[test]
    fn any_remaining_share_shows_at_least_one_cell() {
        assert_eq!(shape(&window(0.4, Tone::Critical, None)).filled, 1);
    }

    #[test]
    fn pace_tick_only_on_warning_and_critical_bars() {
        assert_eq!(shape(&window(40.0, Tone::Good, Some(30.0))).tick, None);
        assert_eq!(
            shape(&window(40.0, Tone::Warning, Some(30.0))).tick,
            Some(14)
        );
        assert_eq!(
            shape(&window(40.0, Tone::Critical, Some(0.0))).tick,
            Some(19)
        );
    }

    #[test]
    fn plain_bar_uses_distinct_glyphs() {
        let bar = render(&window(50.0, Tone::Good, None), Palette::plain());
        assert_eq!(bar, format!("{}{}", "━".repeat(10), "─".repeat(10)));
    }

    #[test]
    fn colored_bar_paints_fill_and_track() {
        let bar = render(
            &window(10.0, Tone::Critical, Some(95.0)),
            Palette::colored(),
        );
        let expected = format!(
            "\x1b[38;2;255;69;58m━\x1b[0m┃\x1b[38;2;85;85;88m{}\x1b[0m",
            "━".repeat(18)
        );
        assert_eq!(bar, expected);
    }
}
