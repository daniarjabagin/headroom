use super::*;
use crate::popup_model::model_card::model_card;
use crate::popup_model::spend_fixture::{period, provider};

#[test]
fn projects_use_the_daemon_share_and_split_bars() {
    let list = project_list(Lang::En, SpendUnit::Cost, Basis::Cost, &period()).unwrap();
    assert_eq!(list.caption, "8 projects");
    let first = &list.rows[0];
    assert_eq!(first.name, "~/work/headroom");
    assert_eq!(first.share, "37.8%");
    assert_eq!(first.bar.len(), 2);
    let other = &list.rows[1];
    assert_eq!(other.mark, RowMark::Folder);
    assert_eq!(other.detail.as_deref(), Some("7 projects"));
    assert_eq!(other.bar[0].provider, None);
    let mut none = period();
    none.projects = None;
    assert!(project_list(Lang::En, SpendUnit::Cost, Basis::Cost, &none).is_none());
}

#[test]
fn project_cost_per_mtok_comes_from_the_payload() {
    let mut period = period();
    let list = project_list(Lang::En, SpendUnit::CostPerMtok, Basis::Cost, &period).unwrap();
    let values: Vec<&str> = list.rows.iter().map(|row| row.value.as_str()).collect();
    assert_eq!(values, ["$0.80", "—"]);
    if let Some(projects) = period.projects.as_mut() {
        projects[0].cost_per_mtok_usd_micros = None;
    }
    if let Some(other) = period.projects_other.as_mut() {
        other.cost_per_mtok_usd_micros = Some(533_580);
    }
    let list = project_list(Lang::En, SpendUnit::CostPerMtok, Basis::Cost, &period).unwrap();
    let values: Vec<&str> = list.rows.iter().map(|row| row.value.as_str()).collect();
    assert_eq!(values, ["—", "$0.53"]);
}

#[test]
fn the_model_card_lists_one_provider() {
    let period = period();
    let card = model_card(
        Lang::En,
        SpendUnit::Cost,
        Basis::Cost,
        "Last 30 days",
        &period.by_provider[0],
    )
    .unwrap();
    assert_eq!(card.title, "Claude · Last 30 days");
    assert_eq!(card.total, "$66.45");
    assert_eq!(card.lines.len(), 4);
    assert_eq!(card.lines[0].share, "62%");
    assert_eq!(card.lines[0].tokens, "38.4M tokens");
    assert_eq!(card.lines[3].name, "Other");
    assert_eq!(card.lines[3].detail.as_deref(), Some("2 models"));
    assert!(card.folded);
    let bare = provider("x", "X", Vec::new(), None);
    assert!(model_card(Lang::En, SpendUnit::Cost, Basis::Cost, "t", &bare).is_none());
}
