use std::rc::Rc;

use super::App;
use crate::ui::onboarding::found::should_open;
use crate::ui::onboarding::{OnboardingSetup, OnboardingWindow};
use crate::ui::prefs::shortcut;

#[derive(Default)]
pub(super) struct OnboardingSlot {
    window: Option<Rc<OnboardingWindow>>,
    dismissed: bool,
}

impl App {
    pub(super) fn sync_onboarding(self: &Rc<Self>) {
        let wanted = {
            let model = self.model.borrow();
            let slot = self.onboarding.borrow();
            slot.window.is_none()
                && should_open(
                    model.view.state(),
                    model.settings.settings(),
                    slot.dismissed,
                )
        };
        if wanted {
            self.open_onboarding();
        }
        let window = self.onboarding.borrow().window.clone();
        if let Some(window) = window {
            let model = self.model.borrow();
            if let (Some(state), Some(settings)) = (model.view.state(), model.settings.settings()) {
                window.update(state, settings, model.providers.as_ref());
            }
        }
    }

    fn open_onboarding(self: &Rc<Self>) {
        let (weak, closed) = (Rc::downgrade(self), Rc::downgrade(self));
        let setup = OnboardingSetup {
            lang: self.lang(),
            logo_color: self.logo_color(),
            act: Rc::new(move |action| {
                if let Some(app) = weak.upgrade() {
                    app.prefs_action(action);
                }
            }),
            shortcut_supported: shortcut::support(),
            on_closed: Rc::new(move || {
                if let Some(app) = closed.upgrade() {
                    let mut slot = app.onboarding.borrow_mut();
                    slot.window = None;
                    slot.dismissed = true;
                }
            }),
        };
        let window = OnboardingWindow::new(Some(&self.application), &setup);
        window.present();
        self.onboarding.borrow_mut().window = Some(window);
    }

    pub(super) fn onboarding_restored(&self, result: &Result<(), String>) {
        if let Some(window) = self.onboarding.borrow().window.as_ref() {
            window.restored(result);
        }
    }
}
