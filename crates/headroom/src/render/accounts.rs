use headroom_daemon::state::payload::{AccountStatus, AccountView};

use super::table::table;

const HEADER: [&str; 7] = [
    "ID", "PROVIDER", "LABEL", "EMAIL", "PLAN", "STATUS", "HIDDEN",
];
const NONE: &str = "-";

pub fn render_accounts(accounts: &[AccountView]) -> String {
    if accounts.is_empty() {
        return "No accounts found.\n".to_owned();
    }
    let rows: Vec<Vec<String>> = accounts.iter().map(row).collect();
    format!("{}\n", table(&HEADER, &rows))
}

fn row(account: &AccountView) -> Vec<String> {
    let text = |value: &Option<String>| value.clone().unwrap_or_else(|| NONE.to_owned());
    vec![
        account.id.clone(),
        account.provider.as_str().to_owned(),
        text(&account.label),
        text(&account.email),
        text(&account.plan),
        status_name(account.status).to_owned(),
        if account.hidden { "yes" } else { "no" }.to_owned(),
    ]
}

fn status_name(status: AccountStatus) -> &'static str {
    match status {
        AccountStatus::Fresh => "fresh",
        AccountStatus::Stale => "stale",
        AccountStatus::Refreshing => "refreshing",
        AccountStatus::Error => "error",
        AccountStatus::SignedOut => "signed out",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::fixtures::full_state;

    #[test]
    fn lists_every_account_including_hidden_ones() {
        let expected = "\
ID            PROVIDER  LABEL  EMAIL               PLAN  STATUS      HIDDEN
codex:work    codex     Work   ada@example.com     Pro   fresh       no
claude:main   claude    -      ada@claude.example  Pro   signed out  no
codex:hidden  codex     -      -                   -     stale       yes
";
        assert_eq!(render_accounts(&full_state().accounts), expected);
    }

    #[test]
    fn empty_list_says_so() {
        assert_eq!(render_accounts(&[]), "No accounts found.\n");
    }
}
