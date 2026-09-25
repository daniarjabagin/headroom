use crate::i18n::{Lang, fill};
use crate::payload::{Account, Recovery, RecoveryField};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NoticeButton {
    Retry,
    SignIn,
    SignInAgain { account_id: String },
    CopyCommand { command: String },
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

fn offered_buttons(account: &Account, recovery: &Recovery) -> Vec<NoticeButton> {
    match recovery {
        Recovery::SignIn { account_id } => vec![NoticeButton::SignInAgain {
            account_id: account_id
                .clone()
                .filter(|id| !id.is_empty())
                .unwrap_or_else(|| account.id.clone()),
        }],
        Recovery::CliLogin { command } if !command.trim().is_empty() => {
            vec![NoticeButton::CopyCommand {
                command: command.trim().to_owned(),
            }]
        }
        Recovery::Retry | Recovery::CliLogin { .. } | Recovery::Unknown => {
            vec![NoticeButton::Retry]
        }
    }
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
    let RecoveryField::Offered(Recovery::CliLogin { command }) = &account.recovery else {
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
        NoticeButton::SignIn => ("Sign in…", false, true),
        NoticeButton::SignInAgain { .. } => ("Sign in again…", false, true),
        NoticeButton::CopyCommand { .. } if state.copied => ("Copied", false, true),
        NoticeButton::CopyCommand { .. } => ("Copy command", false, true),
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
