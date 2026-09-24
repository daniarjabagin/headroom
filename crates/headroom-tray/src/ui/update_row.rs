use gtk::prelude::*;

use crate::payload::Update;
use crate::ui::context::{Action, Ctx};
use crate::ui::widgets::{button, column, icon, label, row, text_button, wrapping_label};
use crate::update::{
    UpdateAction, UpdateRun, notes_url, update_action, update_title, whats_new_url,
};

const UPDATE_ICON: i32 = 16;
const BADGE_SIZE: i32 = 10;

fn badge(ctx: &Ctx, run: &UpdateRun) -> Option<gtk::Widget> {
    match run {
        UpdateRun::Running(_) => {
            let spinner = gtk::Spinner::new();
            spinner.set_spinning(ctx.motion);
            spinner.set_size_request(BADGE_SIZE, BADGE_SIZE);
            Some(spinner.upcast())
        }
        UpdateRun::Done { .. } => Some(
            icon(
                "object-select-symbolic",
                BADGE_SIZE,
                &["headroom-update-badge", "done"],
            )
            .upcast(),
        ),
        UpdateRun::Failed(_) => Some(
            icon(
                "dialog-error-symbolic",
                BADGE_SIZE,
                &["headroom-update-badge", "failed"],
            )
            .upcast(),
        ),
        UpdateRun::Idle => None,
    }
}

fn detail_line(ctx: &Ctx, update: &Update, run: &UpdateRun) -> gtk::Box {
    let lang = ctx.locale.lang;
    let line = row(4, &[]);
    if let Some(badge) = badge(ctx, run) {
        badge.set_valign(gtk::Align::Start);
        line.append(&badge);
    }
    if let Some(text) = run.line(lang) {
        let detail = wrapping_label(&text, &["headroom-update-detail"]);
        if matches!(run, UpdateRun::Failed(_)) {
            detail.add_css_class("failed");
        }
        line.append(&detail);
    } else if let Some(url) = whats_new_url(update) {
        let whats_new = label(lang.tr("What's new"), &["headroom-update-link-text"]);
        let open = button(
            &whats_new,
            &["headroom-update-link"],
            ctx.action(Action::OpenUrl(url.to_owned())),
        );
        line.append(&open);
    }
    line
}

fn action_button(
    ctx: &Ctx,
    update: &Update,
    action: UpdateAction,
    run: &UpdateRun,
) -> Option<gtk::Button> {
    let lang = ctx.locale.lang;
    if matches!(run, UpdateRun::Running(_) | UpdateRun::Done { .. }) {
        return None;
    }
    let retry = matches!(run, UpdateRun::Failed(_));
    let text = if retry {
        lang.tr("Retry")
    } else {
        action.label(lang)
    };
    let command = match action {
        UpdateAction::Install => Action::InstallUpdate,
        UpdateAction::Command => Action::ToggleUpdateCommand,
        UpdateAction::Notes => Action::OpenUrl(notes_url(update)?.to_owned()),
    };
    let button = text_button(text, &["headroom-small-button"], ctx.action(command));
    button.set_valign(gtk::Align::Center);
    if action == UpdateAction::Install && !retry {
        button.add_css_class("primary");
    }
    if action == UpdateAction::Command && ctx.ui.update_command_open {
        button.add_css_class("checked");
    }
    Some(button)
}

fn command_chip(ctx: &Ctx, update: &Update) -> gtk::Box {
    let lang = ctx.locale.lang;
    let chip = row(6, &["headroom-update-command"]);
    let text = wrapping_label(&update.command, &["headroom-update-command-text"]);
    text.set_selectable(true);
    text.set_hexpand(true);
    chip.append(&text);
    let copy_text = lang.tr(if ctx.ui.copied { "Copied" } else { "Copy" });
    let copy = text_button(
        copy_text,
        &["headroom-small-button"],
        ctx.action(Action::Copy(update.command.clone())),
    );
    copy.set_valign(gtk::Align::Start);
    chip.append(&copy);
    chip
}

pub fn update_row(ctx: &Ctx, update: &Update) -> gtk::Box {
    let action = update_action(update);
    let idle = UpdateRun::Idle;
    let run = if action == Some(UpdateAction::Install) {
        &ctx.ui.update_run
    } else {
        &idle
    };
    let strip = column(8, &["headroom-update"]);
    let main = row(10, &[]);
    main.append(&icon(
        "software-update-available-symbolic",
        UPDATE_ICON,
        &["headroom-update-icon"],
    ));
    let texts = column(1, &[]);
    texts.set_hexpand(true);
    texts.set_valign(gtk::Align::Center);
    texts.append(&wrapping_label(
        &update_title(ctx.locale.lang, update),
        &["headroom-update-title"],
    ));
    texts.append(&detail_line(ctx, update, run));
    main.append(&texts);
    if let Some(button) = action.and_then(|action| action_button(ctx, update, action, run)) {
        main.append(&button);
    }
    strip.append(&main);
    if action == Some(UpdateAction::Command) && ctx.ui.update_command_open {
        strip.append(&command_chip(ctx, update));
    }
    strip
}
