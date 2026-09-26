use crate::i18n::{Lang, fill};
use crate::payload::{Account, Recovery, RecoveryField};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NoticeButton {
    Retry,
    SignIn,
    SignInAgain { account_id: String },
    CliSignIn { account_id: String },
    CopyCommand { command: String, primary: bool },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ButtonState {
    pub busy: bool,
    pub copied: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ButtonModel {
    pub label: &'static str,
    pub busy: bool,
    pub primary: bool,
}

fn chosen_id(offered: Option<&String>, account: &Account) -> String {
    offered
        .filter(|id| !id.trim().is_empty())
        .map_or_else(|| account.id.clone(), |id| id.trim().to_owned())
}

fn cli_login_buttons(
    account: &Account,
    command: &str,
    account_id: Option<&String>,
) -> Vec<NoticeButton> {
    let command = command.trim();
    let sign_in = account_id.is_some_and(|id| !id.trim().is_empty());
    let mut buttons = Vec::new();
    if sign_in {
        buttons.push(NoticeButton::CliSignIn {
            account_id: chosen_id(account_id, account),
        });
        buttons.push(NoticeButton::Retry);
    }
    if !command.is_empty() {
        buttons.push(NoticeButton::CopyCommand {
            command: command.to_owned(),
            primary: !sign_in,
        });
    }
    if buttons.is_empty() {
        buttons.push(NoticeButton::Retry);
    }
    buttons
}

fn offered_buttons(account: &Account, recovery: &Recovery) -> Vec<NoticeButton> {
    match recovery {
        Recovery::SignIn { account_id } => vec![NoticeButton::SignInAgain {
            account_id: chosen_id(account_id.as_ref(), account),
        }],
        Recovery::CliLogin {
            command,
            account_id,
        } => cli_login_buttons(account, command, account_id.as_ref()),
        Recovery::Retry | Recovery::Unknown => vec![NoticeButton::Retry],
    }
}

#[must_use]
pub fn button_rows(buttons: &[NoticeButton]) -> Vec<Vec<NoticeButton>> {
    if buttons.len() <= 2 {
        return vec![buttons.to_vec()];
    }
    let (copy, rest): (Vec<NoticeButton>, Vec<NoticeButton>) = buttons
        .iter()
        .cloned()
        .partition(|button| matches!(button, NoticeButton::CopyCommand { .. }));
    [rest, copy]
        .into_iter()
        .filter(|row| !row.is_empty())
        .collect()
}

#[must_use]
pub fn notice_buttons(
    account: &Account,
    signed_out: bool,
    terminal_sign_in: bool,
) -> Vec<NoticeButton> {
    match &account.recovery {
        RecoveryField::Offered(recovery) => offered_buttons(account, recovery),
        RecoveryField::Wait => Vec::new(),
        RecoveryField::Unreported if signed_out && terminal_sign_in => {
            vec![NoticeButton::SignIn, NoticeButton::Retry]
        }
        RecoveryField::Unreported => vec![NoticeButton::Retry],
    }
}

#[must_use]
pub fn cli_login_hint(lang: Lang, account: &Account) -> Option<String> {
    let RecoveryField::Offered(Recovery::CliLogin { command, .. }) = &account.recovery else {
        return None;
    };
    let command = command.trim();
    (!command.is_empty()).then(|| {
        fill(
            lang.tr("Run `{command}` in a terminal — Headroom picks it up automatically."),
            &[("command", command)],
        )
    })
}

#[must_use]
pub fn button_model(lang: Lang, button: &NoticeButton, state: ButtonState) -> ButtonModel {
    let (label, busy, primary) = match button {
        NoticeButton::Retry if state.busy => ("Retrying…", true, false),
        NoticeButton::Retry => ("Retry", false, false),
        NoticeButton::SignIn | NoticeButton::CliSignIn { .. } => ("Sign in…", false, true),
        NoticeButton::SignInAgain { .. } => ("Sign in again…", false, true),
        NoticeButton::CopyCommand { primary, .. } if state.copied => ("Copied", false, *primary),
        NoticeButton::CopyCommand { primary, .. } => ("Copy command", false, *primary),
    };
    ButtonModel {
        label: lang.tr(label),
        busy,
        primary,
    }
}

#[cfg(test)]
#[path = "recovery_tests.rs"]
mod tests;
