pub const POPUP_WIDTH: i32 = 320;
pub const SCREEN_MARGIN: i32 = 8;
pub const PANEL_GAP: i32 = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

impl Rect {
    fn contains(self, (x, y): (i32, i32)) -> bool {
        x >= self.x && x < self.x + self.width && y >= self.y && y < self.y + self.height
    }

    fn upper_half(self, y: i32) -> bool {
        y < self.y + self.height / 2
    }

    #[must_use]
    pub fn scaled(self, factor: i32) -> Self {
        Self {
            x: self.x * factor,
            y: self.y * factor,
            width: self.width * factor,
            height: self.height * factor,
        }
    }
}

#[must_use]
pub fn usable_click(click: Option<(i32, i32)>) -> Option<(i32, i32)> {
    click.filter(|point| *point != (0, 0))
}

#[must_use]
pub fn monitor_for(monitors: &[Rect], click: Option<(i32, i32)>) -> Option<usize> {
    let click = usable_click(click)?;
    monitors.iter().position(|monitor| monitor.contains(click))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Edge {
    Top,
    Bottom,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LayerPlacement {
    pub edge: Edge,
    pub margin_left: i32,
    pub margin_edge: i32,
}

fn clamp_left(monitor: Rect, left: i32, width: i32) -> i32 {
    let max = (monitor.width - width - SCREEN_MARGIN).max(SCREEN_MARGIN);
    left.clamp(SCREEN_MARGIN, max)
}

#[must_use]
pub fn layer_placement(monitor: Rect, click: Option<(i32, i32)>, width: i32) -> LayerPlacement {
    match usable_click(click) {
        Some((x, y)) => LayerPlacement {
            edge: if monitor.upper_half(y) {
                Edge::Top
            } else {
                Edge::Bottom
            },
            margin_left: clamp_left(monitor, x - monitor.x - width / 2, width),
            margin_edge: PANEL_GAP,
        },
        None => LayerPlacement {
            edge: Edge::Top,
            margin_left: clamp_left(monitor, monitor.width, width),
            margin_edge: PANEL_GAP,
        },
    }
}

#[must_use]
pub fn window_position(area: Rect, click: (i32, i32), size: (i32, i32)) -> (i32, i32) {
    let (x, y) = click;
    let (width, height) = size;
    let max_x = (area.x + area.width - width - SCREEN_MARGIN).max(area.x + SCREEN_MARGIN);
    let left = (x - width / 2).clamp(area.x + SCREEN_MARGIN, max_x);
    let max_y = (area.y + area.height - height - SCREEN_MARGIN).max(area.y + SCREEN_MARGIN);
    let top = if area.upper_half(y) {
        y + PANEL_GAP
    } else {
        y - PANEL_GAP - height
    };
    (left, top.clamp(area.y + SCREEN_MARGIN, max_y))
}

#[must_use]
pub fn max_popup_height(monitor_height: i32) -> i32 {
    (monitor_height - 2 * SCREEN_MARGIN - 2 * PANEL_GAP - 32).max(200)
}

#[cfg(test)]
mod tests {
    use super::*;

    const LEFT: Rect = Rect {
        x: 0,
        y: 0,
        width: 1920,
        height: 1080,
    };
    const RIGHT: Rect = Rect {
        x: 1920,
        y: 0,
        width: 2560,
        height: 1440,
    };

    #[test]
    fn picks_the_monitor_under_the_click() {
        let monitors = [LEFT, RIGHT];
        assert_eq!(monitor_for(&monitors, Some((100, 10))), Some(0));
        assert_eq!(monitor_for(&monitors, Some((2000, 1400))), Some(1));
        assert_eq!(monitor_for(&monitors, Some((0, 0))), None);
        assert_eq!(monitor_for(&monitors, None), None);
        assert_eq!(monitor_for(&monitors, Some((9000, 10))), None);
    }

    #[test]
    fn layer_placement_centres_on_the_click() {
        let top = layer_placement(RIGHT, Some((1920 + 1000, 12)), POPUP_WIDTH);
        assert_eq!(
            top,
            LayerPlacement {
                edge: Edge::Top,
                margin_left: 840,
                margin_edge: 4
            }
        );
        let bottom = layer_placement(LEFT, Some((1910, 1070)), POPUP_WIDTH);
        assert_eq!(bottom.edge, Edge::Bottom);
        assert_eq!(bottom.margin_left, 1920 - 320 - 8);
        let edge = layer_placement(LEFT, Some((3, 5)), POPUP_WIDTH);
        assert_eq!(edge.margin_left, 8);
    }

    #[test]
    fn layer_placement_without_a_click_uses_the_top_right() {
        let placement = layer_placement(LEFT, None, POPUP_WIDTH);
        assert_eq!(placement.edge, Edge::Top);
        assert_eq!(placement.margin_left, 1920 - 320 - 8);
    }

    #[test]
    fn window_position_hangs_below_or_above_the_click() {
        assert_eq!(window_position(LEFT, (1800, 20), (320, 600)), (1592, 24));
        assert_eq!(window_position(LEFT, (500, 1070), (320, 600)), (340, 466));
        assert_eq!(window_position(LEFT, (500, 1070), (320, 2000)), (340, 8));
    }

    #[test]
    fn scales_rects_and_limits_height() {
        assert_eq!(LEFT.scaled(2).width, 3840);
        assert_eq!(max_popup_height(1080), 1080 - 16 - 8 - 32);
        assert_eq!(max_popup_height(100), 200);
    }
}
