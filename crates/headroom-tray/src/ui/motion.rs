use adw::prelude::*;

const STANDARD_MS: u32 = 200;

pub fn grow(widget: &impl IsA<gtk::Widget>, set_progress: impl Fn(f64) + 'static) {
    let target_widget = widget.clone().upcast::<gtk::Widget>();
    let redraw = target_widget.downgrade();
    let target = adw::CallbackAnimationTarget::new(move |progress| {
        set_progress(progress);
        if let Some(widget) = redraw.upgrade() {
            widget.queue_draw();
        }
    });
    let animation = adw::TimedAnimation::builder()
        .widget(&target_widget)
        .value_from(0.0)
        .value_to(1.0)
        .duration(STANDARD_MS)
        .easing(adw::Easing::EaseOutCubic)
        .target(&target)
        .build();
    widget.connect_map(move |_| animation.play());
}

#[must_use]
pub fn reduced(setting: bool) -> bool {
    setting || !gtk::Settings::default().is_some_and(|settings| settings.is_gtk_enable_animations())
}
