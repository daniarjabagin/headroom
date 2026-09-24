use std::rc::Rc;

use adw::prelude::*;

use super::PrefsAction;
use super::add_account::DialogCtx;
use super::flow_page::{flow_body, navigation_page, pill, stack_of, wrapping};
use crate::preferences::registry::{AddMethod, ProviderInfo};

pub struct DetectPage {
    pub page: adw::NavigationPage,
    ctx: DialogCtx,
    button: gtk::Button,
    status: gtk::Label,
}

fn reason(method: &AddMethod) -> Option<&str> {
    match method {
        AddMethod::AutoDetect { reason } => reason.as_deref(),
        _ => None,
    }
}

impl DetectPage {
    pub fn new(ctx: &DialogCtx, provider: &ProviderInfo, method: &AddMethod) -> Rc<Self> {
        let lang = ctx.lang;
        let body = flow_body(&provider.id, &ctx.logo_color, &provider.display_name);
        body.description.set_text(
            reason(method).unwrap_or_else(|| lang.tr("Headroom finds this account on its own.")),
        );
        let status = wrapping("", &["dim-label", "caption"]);
        status.set_visible(false);
        let (act, id) = (Rc::clone(&ctx.act), provider.id.clone());
        let button = pill(lang.tr("Detect Again"), true, move || {
            act(PrefsAction::Restore(id.clone()));
        });
        let note = wrapping(
            lang.tr("Already set up, or removed it earlier? Detect it again."),
            &["caption"],
        );
        body.column.append(&stack_of(&[
            note.upcast_ref(),
            button.upcast_ref(),
            status.upcast_ref(),
        ]));
        let page = Rc::new(Self {
            page: navigation_page(&provider.display_name, &body.column),
            ctx: ctx.clone(),
            button,
            status,
        });
        let weak = Rc::downgrade(&page);
        page.button.connect_clicked(move |_| {
            if let Some(page) = weak.upgrade() {
                page.searching();
            }
        });
        page
    }

    fn searching(&self) {
        self.button.set_sensitive(false);
        self.status.set_visible(true);
        self.status
            .set_text(self.ctx.lang.tr("Looking for accounts…"));
    }

    pub fn restored(&self, result: &Result<(), String>) {
        let lang = self.ctx.lang;
        self.button.set_sensitive(true);
        self.status.set_visible(true);
        match result {
            Ok(()) => self.status.set_text(
                lang.tr("Detection finished. Accounts that were found appear in the list."),
            ),
            Err(message) => self.status.set_text(message),
        }
    }
}
