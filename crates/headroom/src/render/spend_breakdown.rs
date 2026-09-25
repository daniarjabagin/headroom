use headroom_core::account::ProviderId;
use headroom_daemon::catalog::ProviderCatalog;
use headroom_daemon::usage::breakdown::GroupBy;
use headroom_daemon::usage::report::rows::{ReportRow, SpendReport};
use jiff::civil::Date;

use super::format::{compact_tokens, usd};
use super::printable::printable;
use super::style::Palette;

const BAR_WIDTH: u32 = 12;
const BAR: &str = "█";
const MISSING: &str = "—";
const PARTIAL: &str = "partial";
const PARTIAL_NOTE: &str = "partial: some usage has no known price and is left out of the cost";

#[derive(Clone, Copy, PartialEq, Eq)]
enum Align {
    Left,
    Right,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Look {
    Plain,
    Dim,
    Bold,
    Bar,
}

#[derive(Clone, Copy)]
struct Column {
    title: &'static str,
    align: Align,
    look: Look,
}

const fn column(title: &'static str, align: Align, look: Look) -> Column {
    Column { title, align, look }
}

const TOKEN_COLUMNS: [Column; 4] = [
    column("Input", Align::Right, Look::Plain),
    column("Output", Align::Right, Look::Plain),
    column("Cache read", Align::Right, Look::Plain),
    column("Cache write", Align::Right, Look::Plain),
];
const COST: Column = column("Cost", Align::Right, Look::Bold);
const SHARE: Column = column("Share", Align::Right, Look::Dim);
const MARKER: Column = column("", Align::Left, Look::Dim);

pub fn render_breakdown(
    report: &SpendReport,
    catalog: &ProviderCatalog,
    palette: Palette,
) -> String {
    let title = title_line(report, palette);
    if report.rows.is_empty() {
        return format!("{title}\n\nNo local usage in {}.\n", range(report));
    }
    let (columns, rows, total) = grid(report, catalog);
    let mut lines = vec![title, String::new()];
    lines.extend(table(&columns, &rows, &total, palette));
    if report.total.partial {
        lines.push(palette.dim(PARTIAL_NOTE));
    }
    let mut out = lines.join("\n");
    out.push('\n');
    out
}

fn title_line(report: &SpendReport, palette: Palette) -> String {
    let (name, note) = match report.by {
        GroupBy::Model => ("model", "estimated from local logs"),
        GroupBy::Project => ("project", "project = working directory of the session"),
        GroupBy::Provider => ("provider", "estimated from local logs"),
        GroupBy::Day => ("day", "estimated from local logs"),
    };
    format!(
        "{}  {}",
        palette.bold(&format!("Spend by {name}")),
        palette.dim(&format!("{} · {note}", range(report)))
    )
}

fn range(report: &SpendReport) -> String {
    let with_year = report.since.year() != report.until.year();
    let show = |day: Date| {
        if with_year {
            day.strftime("%b %-d, %Y").to_string()
        } else {
            day.strftime("%b %-d").to_string()
        }
    };
    if report.since == report.until {
        return show(report.since);
    }
    format!("{} – {}", show(report.since), show(report.until))
}

fn columns(report: &SpendReport) -> Vec<Column> {
    let key = |title| column(title, Align::Left, Look::Plain);
    match report.by {
        GroupBy::Model => [key("Model"), column("Provider", Align::Left, Look::Dim)]
            .into_iter()
            .chain(TOKEN_COLUMNS)
            .chain([COST])
            .collect(),
        GroupBy::Project => vec![
            key("Project"),
            column("Tokens", Align::Right, Look::Plain),
            COST,
            SHARE,
        ],
        GroupBy::Provider => std::iter::once(key("Provider"))
            .chain(TOKEN_COLUMNS)
            .chain([COST, SHARE])
            .collect(),
        GroupBy::Day => std::iter::once(key("Day"))
            .chain(TOKEN_COLUMNS)
            .chain([COST])
            .collect(),
    }
}

type Grid = (Vec<Column>, Vec<Vec<String>>, Vec<String>);

fn grid(report: &SpendReport, catalog: &ProviderCatalog) -> Grid {
    let mut columns = columns(report);
    let mut rows: Vec<Vec<String>> = report
        .rows
        .iter()
        .map(|row| cells(report, row, catalog))
        .collect();
    let mut total = cells(report, &report.total, catalog);
    if let Some(first) = total.first_mut() {
        "Total".clone_into(first);
    }
    if report.by == GroupBy::Project {
        columns.push(column("", Align::Left, Look::Bar));
        add_bars(&mut rows, &report.rows);
        total.push(String::new());
    }
    if report.rows.iter().any(|row| row.partial) {
        columns.push(MARKER);
        for (cells, row) in rows.iter_mut().zip(&report.rows) {
            cells.push(marker(row));
        }
        total.push(marker(&report.total));
    }
    (columns, rows, total)
}

fn cells(report: &SpendReport, row: &ReportRow, catalog: &ProviderCatalog) -> Vec<String> {
    let tokens = &row.tokens;
    let token_cells = [
        tokens.input,
        tokens.output,
        tokens.cache_read,
        tokens.cache_write,
    ]
    .map(compact_tokens);
    match report.by {
        GroupBy::Model => [key(row), provider_name(row.provider.as_ref(), catalog)]
            .into_iter()
            .chain(token_cells)
            .chain([cost(row)])
            .collect(),
        GroupBy::Project => vec![
            key(row),
            compact_tokens(tokens.total),
            cost(row),
            share(row),
        ],
        GroupBy::Provider => std::iter::once(provider_name(row.provider.as_ref(), catalog))
            .chain(token_cells)
            .chain([cost(row), share(row)])
            .collect(),
        GroupBy::Day => std::iter::once(day(row))
            .chain(token_cells)
            .chain([cost(row)])
            .collect(),
    }
}

fn key(row: &ReportRow) -> String {
    row.key.clone().unwrap_or_else(|| "(no project)".to_owned())
}

fn provider_name(provider: Option<&ProviderId>, catalog: &ProviderCatalog) -> String {
    provider.map_or_else(String::new, |id| catalog.display_name(id).to_owned())
}

fn day(row: &ReportRow) -> String {
    let parsed = row.key.as_deref().and_then(|key| key.parse::<Date>().ok());
    parsed.map_or_else(|| key(row), |date| date.strftime("%a %b %-d").to_string())
}

fn cost(row: &ReportRow) -> String {
    let unpriced = row.unpriced_tokens == row.tokens.total && row.tokens.total > 0;
    if unpriced {
        MISSING.to_owned()
    } else {
        usd(row.cost_usd_micros)
    }
}

fn share(row: &ReportRow) -> String {
    format!("{}.{}%", row.share_permille / 10, row.share_permille % 10)
}

fn marker(row: &ReportRow) -> String {
    if row.partial {
        PARTIAL.to_owned()
    } else {
        String::new()
    }
}

fn add_bars(cells: &mut [Vec<String>], rows: &[ReportRow]) {
    let widest = rows.iter().map(|row| row.share_permille).max().unwrap_or(0);
    for (cells, row) in cells.iter_mut().zip(rows) {
        cells.push(BAR.repeat(bar_length(row.share_permille, widest)));
    }
}

fn bar_length(share: u32, widest: u32) -> usize {
    if share == 0 || widest == 0 {
        return 0;
    }
    let length =
        (u64::from(share) * u64::from(BAR_WIDTH) + u64::from(widest) / 2) / u64::from(widest);
    usize::try_from(length.max(1)).unwrap_or(0)
}

fn table(
    columns: &[Column],
    rows: &[Vec<String>],
    total: &[String],
    palette: Palette,
) -> Vec<String> {
    let clean = |row: &[String]| -> Vec<String> {
        row.iter()
            .map(|cell| printable(cell).into_owned())
            .collect()
    };
    let rows: Vec<Vec<String>> = rows.iter().map(|row| clean(row)).collect();
    let total = clean(total);
    let widths: Vec<usize> = columns
        .iter()
        .enumerate()
        .map(|(index, column)| {
            let cells = rows.iter().chain([&total]).filter_map(|row| row.get(index));
            let widest = cells.map(|cell| cell.chars().count()).max().unwrap_or(0);
            widest.max(column.title.chars().count())
        })
        .collect();
    let titles: Vec<String> = columns
        .iter()
        .map(|column| column.title.to_owned())
        .collect();
    let rule_width = widths.iter().sum::<usize>() + 2 * widths.len().saturating_sub(1);
    let mut lines = vec![line(columns, &widths, &titles, |text, _| palette.dim(text))];
    lines.extend(rows.iter().map(|row| {
        line(columns, &widths, row, |text, look| {
            styled(palette, text, look)
        })
    }));
    lines.push(palette.dim(&"─".repeat(rule_width)));
    lines.push(line(columns, &widths, &total, |text, look| {
        if look == Look::Dim {
            palette.dim(text)
        } else {
            palette.bold(text)
        }
    }));
    lines
}

fn line(
    columns: &[Column],
    widths: &[usize],
    row: &[String],
    style: impl Fn(&str, Look) -> String,
) -> String {
    let last = columns.len().saturating_sub(1);
    let cells: Vec<String> = columns
        .iter()
        .zip(widths)
        .zip(row)
        .enumerate()
        .map(|(index, ((column, width), cell))| {
            let width = if index == last && column.align == Align::Left {
                0
            } else {
                *width
            };
            let text = aligned(cell, width, column.align);
            if cell.is_empty() {
                text
            } else {
                style(&text, column.look)
            }
        })
        .collect();
    cells.join("  ").trim_end().to_owned()
}

fn aligned(cell: &str, width: usize, align: Align) -> String {
    let padding = " ".repeat(width.saturating_sub(cell.chars().count()));
    match align {
        Align::Left => format!("{cell}{padding}"),
        Align::Right => format!("{padding}{cell}"),
    }
}

fn styled(palette: Palette, text: &str, look: Look) -> String {
    match look {
        Look::Plain => text.to_owned(),
        Look::Dim => palette.dim(text),
        Look::Bold => palette.bold(text),
        Look::Bar => palette.tone(text, headroom_core::pace::Tone::Good),
    }
}

#[cfg(test)]
#[path = "spend_breakdown_tests.rs"]
mod tests;
