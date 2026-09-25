use gtk::prelude::*;

use crate::account::shown_windows;
use crate::payload::State;
use crate::popup_model::model_card::model_card;
use crate::popup_model::spend_view::period_heading;
use crate::ui::context::{Ctx, ShareTarget};
use crate::ui::header::links_of;
use crate::ui::header_menu::{MenuContext, MenuInput, menu_items};
use crate::ui::model_popover::{CardLook, model_card_view};
use crate::ui::unit_menu::menu;
use crate::ui::widgets::column;

pub struct PreviewPart {
    pub name: &'static str,
    pub widget: gtk::Widget,
}

fn surface(child: &impl IsA<gtk::Widget>, classes: &[&str]) -> gtk::Widget {
    let frame = column(0, &["headroom-preview-surface"]);
    for class in classes {
        frame.add_css_class(class);
    }
    frame.append(child);
    frame.upcast()
}

fn model_part(ctx: &Ctx, state: &State) -> Option<PreviewPart> {
    let choice = ctx.spend?;
    let period = choice.period_of(&state.spend);
    let heading = period_heading(ctx.locale.lang, choice.period);
    period.by_provider.iter().find_map(|spend| {
        let card = model_card(ctx.locale.lang, choice.unit, choice.basis(), heading, spend)?;
        let look = CardLook {
            lang: ctx.locale.lang,
            series: ctx.series_color(&spend.provider),
            track: ctx.color("track"),
        };
        Some(PreviewPart {
            name: "models",
            widget: surface(&model_card_view(look, &card), &[]),
        })
    })
}

fn unit_part(ctx: &Ctx) -> Option<PreviewPart> {
    let popover = menu(ctx);
    let items = popover.child()?;
    popover.set_child(None::<&gtk::Widget>);
    Some(PreviewPart {
        name: "units",
        widget: surface(&items, &["headroom-menu"]),
    })
}

fn menu_part(ctx: &Ctx, state: &State) -> Option<PreviewPart> {
    let account = state
        .accounts
        .iter()
        .find(|account| !account.hidden && !shown_windows(account).is_empty())?;
    let input = MenuInput {
        provider_name: account.provider_name.clone(),
        account_ids: vec![account.id.clone()],
        starred: ctx.display.is_starred(&account.id),
        target: ShareTarget::Account(account.id.clone()),
        links: links_of(ctx, &account.provider),
    };
    let holder = gtk::Popover::new();
    let items = menu_items(&MenuContext::of(ctx), &input, &holder);
    Some(PreviewPart {
        name: "menu",
        widget: surface(&items, &["headroom-menu"]),
    })
}

#[must_use]
pub fn popover_parts(ctx: &Ctx, state: &State) -> Vec<PreviewPart> {
    [
        model_part(ctx, state),
        unit_part(ctx),
        menu_part(ctx, state),
    ]
    .into_iter()
    .flatten()
    .collect()
}
