use gtk::prelude::*;

use crate::account::{CardBody, NoticeView, card_body, is_retrying};
use crate::notices::NoticeKind;
use crate::payload::{Account, Window};
use crate::recovery::{ButtonState, NoticeButton};
use crate::ui::context::{Action, Ctx};
use crate::ui::notice::{MountedNotice, notice_line, notice_row};
use crate::ui::status_views::skeleton_rows;
use crate::ui::widgets::column;

#[derive(Debug, PartialEq)]
pub enum RestKey {
    Skeleton(usize),
    Notices(Vec<NoticeView>, ButtonState),
}

pub struct Card<'a> {
    pub alert: Option<NoticeView>,
    pub rest: Option<RestKey>,
    pub windows: Option<Vec<&'a Window>>,
}

fn press(ctx: &Ctx, account: &Account, button: &NoticeButton) -> Box<dyn Fn()> {
    let action = match button {
        NoticeButton::Retry => Action::Retry(account.id.clone()),
        NoticeButton::SignIn => Action::SignIn(account.provider.clone()),
        NoticeButton::SignInAgain { account_id } => Action::SignInAgain(account_id.clone()),
        NoticeButton::CliSignIn { account_id } => Action::CliSignIn(account_id.clone()),
        NoticeButton::CopyCommand { command, .. } => Action::CopyCommand {
            account_id: account.id.clone(),
            command: command.clone(),
        },
    };
    Box::new(ctx.action(action))
}

pub fn button_state(ctx: &Ctx, account: &Account) -> ButtonState {
    ButtonState {
        busy: is_retrying(account) || ctx.ui.retrying.contains(&account.id),
        copied: ctx.ui.copied_command.as_deref() == Some(account.id.as_str()),
    }
}

pub fn mount_notice(ctx: &Ctx, account: &Account, notice: &NoticeView) -> MountedNotice {
    let mounted = notice_row(ctx.locale.lang, ctx.motion, notice, |button| {
        press(ctx, account, button)
    });
    mounted.apply(ctx.locale.lang, |_| button_state(ctx, account));
    mounted
}

fn notice_widget(ctx: &Ctx, account: &Account, notice: &NoticeView) -> gtk::Widget {
    if notice.kind == NoticeKind::Info {
        return notice_line(&notice.title).upcast();
    }
    mount_notice(ctx, account, notice).widget.upcast()
}

pub fn rest_widget(ctx: &Ctx, account: &Account, rest: &RestKey) -> gtk::Widget {
    match rest {
        RestKey::Skeleton(rows) => skeleton_rows(*rows).upcast(),
        RestKey::Notices(notices, _) => {
            let body = column(0, &[]);
            for notice in notices {
                body.append(&notice_widget(ctx, account, notice));
            }
            body.upcast()
        }
    }
}

pub fn card<'a>(ctx: &Ctx, account: &'a Account) -> Card<'a> {
    let terminal_sign_in = ctx.sign_in.contains(&account.provider);
    match card_body(ctx.locale.lang, account, ctx.offline, terminal_sign_in) {
        CardBody::Blocked(notice) => Card {
            alert: Some(notice),
            rest: None,
            windows: None,
        },
        CardBody::Skeleton(rows) => Card {
            alert: None,
            rest: Some(RestKey::Skeleton(rows)),
            windows: None,
        },
        CardBody::Limits {
            alert,
            notices,
            windows,
        } => Card {
            alert,
            rest: (!notices.is_empty())
                .then(|| RestKey::Notices(notices, button_state(ctx, account))),
            windows: Some(windows),
        },
    }
}
