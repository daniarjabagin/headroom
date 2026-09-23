use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom};
use std::path::Path;

const CHUNK_BYTES: u64 = 64 * 1024;

pub(super) fn find_last_line<T>(
    path: &Path,
    mut parse: impl FnMut(&str) -> Option<T>,
) -> io::Result<Option<T>> {
    let mut file = File::open(path)?;
    let mut end = file.metadata()?.len();
    let mut carry: Vec<u8> = Vec::new();
    while end > 0 {
        let start = end.saturating_sub(CHUNK_BYTES);
        let mut block = read_block(&mut file, start, end)?;
        block.append(&mut carry);
        let complete_from = if start == 0 {
            0
        } else {
            block
                .iter()
                .position(|&byte| byte == b'\n')
                .map_or(block.len(), |index| index + 1)
        };
        if let Some(found) = search_backwards(&block[complete_from..], &mut parse) {
            return Ok(Some(found));
        }
        block.truncate(complete_from);
        carry = block;
        end = start;
    }
    Ok(None)
}

fn read_block(file: &mut File, start: u64, end: u64) -> io::Result<Vec<u8>> {
    file.seek(SeekFrom::Start(start))?;
    let mut block = Vec::new();
    file.take(end - start).read_to_end(&mut block)?;
    Ok(block)
}

fn search_backwards<T>(bytes: &[u8], parse: &mut impl FnMut(&str) -> Option<T>) -> Option<T> {
    bytes
        .rsplit(|&byte| byte == b'\n')
        .filter_map(|line| std::str::from_utf8(line).ok())
        .find_map(|line| parse(line.trim_end_matches('\r')))
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    fn lines_matching(content: &str, needle: &str) -> Option<String> {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("log.jsonl");
        fs::write(&path, content).unwrap();
        find_last_line(&path, |line| line.contains(needle).then(|| line.to_owned())).unwrap()
    }

    #[test]
    fn returns_the_last_matching_line() {
        let content = "hit 1\nmiss\nhit 2\nmiss\n";
        assert_eq!(lines_matching(content, "hit"), Some("hit 2".into()));
    }

    #[test]
    fn handles_lines_spanning_chunks() {
        let long = "x".repeat(200_000);
        let content = format!("first hit\n{long}\nhit {long}\ntail\n");
        let found = lines_matching(&content, "hit").unwrap();
        assert_eq!(found.len(), 4 + long.len());
        assert!(found.starts_with("hit "));
    }

    #[test]
    fn finds_the_first_line_of_a_file() {
        let content = format!("only hit\n{}\n", "y".repeat(150_000));
        assert_eq!(lines_matching(&content, "hit"), Some("only hit".into()));
    }

    #[test]
    fn empty_file_or_no_match_is_none() {
        assert_eq!(lines_matching("", "hit"), None);
        assert_eq!(lines_matching("a\nb", "hit"), None);
    }
}
