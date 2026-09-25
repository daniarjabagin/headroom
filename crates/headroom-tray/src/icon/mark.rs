use super::shapes::Polygon;

pub const MARK_GRID: f64 = 16.0;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum MarkError {
    #[error("the mark has no path data")]
    NoPath,
    #[error("the mark path uses an unsupported command {0}")]
    Command(char),
    #[error("the mark path has an unreadable number {0}")]
    Number(String),
}

fn path_data(svg: &str) -> Result<&str, MarkError> {
    let start = svg.find(" d=\"").ok_or(MarkError::NoPath)? + 4;
    let length = svg[start..].find('"').ok_or(MarkError::NoPath)?;
    Ok(&svg[start..start + length])
}

fn tokens(data: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut number = String::new();
    for c in data.chars() {
        if c.is_ascii_alphabetic() {
            tokens.extend(
                std::mem::take(&mut number)
                    .split_whitespace()
                    .map(str::to_owned),
            );
            tokens.push(c.to_string());
        } else if c == ',' {
            number.push(' ');
        } else {
            number.push(c);
        }
    }
    tokens.extend(number.split_whitespace().map(str::to_owned));
    tokens
}

fn close(current: &mut Vec<(f64, f64)>, polygons: &mut Vec<Polygon>) {
    if current.len() >= 3 {
        polygons.push(Polygon {
            points: std::mem::take(current),
        });
    } else {
        current.clear();
    }
}

pub fn parse_mark(svg: &str) -> Result<Vec<Polygon>, MarkError> {
    let mut polygons = Vec::new();
    let mut current = Vec::new();
    let mut numbers = Vec::new();
    for token in tokens(path_data(svg)?) {
        match token.as_str() {
            "M" | "Z" | "z" => close(&mut current, &mut polygons),
            "L" => {}
            text => {
                if let Some(c) = text.chars().next().filter(char::is_ascii_alphabetic) {
                    return Err(MarkError::Command(c));
                }
                let value = text
                    .parse::<f64>()
                    .map_err(|_| MarkError::Number(text.to_owned()))?;
                numbers.push(value);
                if let [x, y] = numbers[..] {
                    current.push((x, y));
                    numbers.clear();
                }
            }
        }
    }
    close(&mut current, &mut polygons);
    Ok(polygons)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::icon::canvas::Shape;

    #[test]
    fn the_brand_mark_is_four_plates() {
        let plates = parse_mark(crate::assets::MARK).unwrap();
        assert_eq!(plates.len(), 4);
        assert!(plates.iter().all(|plate| plate.points.len() == 4));
        assert!(plates[0].contains(3.8, 9.0));
        assert!(!plates[0].contains(6.0, 9.0));
    }

    #[test]
    fn curves_are_rejected() {
        let svg = r#"<svg><path d="M0 0C1 1 2 2 3 3Z"/></svg>"#;
        assert_eq!(parse_mark(svg), Err(MarkError::Command('C')));
        assert_eq!(parse_mark("<svg/>"), Err(MarkError::NoPath));
    }
}
