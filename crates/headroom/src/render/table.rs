use super::printable::printable;

pub fn pad(text: &str, width: usize) -> String {
    let text = printable(text);
    let padding = width.saturating_sub(text.chars().count());
    format!("{text}{}", " ".repeat(padding))
}

pub fn table(header: &[&str], rows: &[Vec<String>]) -> String {
    let rows: Vec<Vec<String>> = rows.iter().map(|row| printable_row(row)).collect();
    let widths: Vec<usize> = (0..header.len())
        .map(|column| {
            let cells = rows.iter().filter_map(|row| row.get(column));
            let widest = cells.map(|cell| cell.chars().count()).max().unwrap_or(0);
            widest.max(header[column].chars().count())
        })
        .collect();
    let header: Vec<String> = header.iter().map(|title| (*title).to_owned()).collect();
    std::iter::once(&header)
        .chain(&rows)
        .map(|row| line(row, &widths))
        .collect::<Vec<_>>()
        .join("\n")
}

fn printable_row(row: &[String]) -> Vec<String> {
    row.iter()
        .map(|cell| printable(cell).into_owned())
        .collect()
}

fn line(row: &[String], widths: &[usize]) -> String {
    let cells: Vec<String> = row
        .iter()
        .zip(widths)
        .map(|(cell, width)| pad(cell, *width))
        .collect();
    cells.join("  ").trim_end().to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pads_by_characters() {
        assert_eq!(pad("ab", 4), "ab  ");
        assert_eq!(pad("né", 3), "né ");
        assert_eq!(pad("long", 2), "long");
    }

    #[test]
    fn control_characters_are_dropped_before_measuring() {
        let rows = vec![vec!["a\x1b[2Jb".to_owned(), "x".to_owned()]];
        assert_eq!(table(&["ID", "N"], &rows), "ID     N\na[2Jb  x");
        assert_eq!(pad("\x1b]0;t\x07", 5), "]0;t ");
    }

    #[test]
    fn aligns_columns_and_trims_line_ends() {
        let rows = vec![
            vec!["codex:1".to_owned(), "codex".to_owned(), "-".to_owned()],
            vec![
                "claude:22".to_owned(),
                "claude".to_owned(),
                "Work".to_owned(),
            ],
        ];
        let expected = "ID         PROVIDER  LABEL\n\
                        codex:1    codex     -\n\
                        claude:22  claude    Work";
        assert_eq!(table(&["ID", "PROVIDER", "LABEL"], &rows), expected);
    }
}
