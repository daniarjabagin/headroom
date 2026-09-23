use std::ops::{Add, AddAssign};

use serde::{Deserialize, Serialize};

use crate::units::Tokens;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TokenCounts {
    pub input: Tokens,
    pub cache_read: Tokens,
    pub cache_write_5m: Tokens,
    pub cache_write_1h: Tokens,
    pub output: Tokens,
    pub reasoning: Tokens,
}

impl TokenCounts {
    #[must_use]
    pub fn total(&self) -> Tokens {
        self.input + self.cache_read + self.cache_write() + self.output
    }

    #[must_use]
    pub fn cache_write(&self) -> Tokens {
        self.cache_write_5m + self.cache_write_1h
    }
}

impl Add for TokenCounts {
    type Output = TokenCounts;

    fn add(self, other: TokenCounts) -> TokenCounts {
        TokenCounts {
            input: self.input + other.input,
            cache_read: self.cache_read + other.cache_read,
            cache_write_5m: self.cache_write_5m + other.cache_write_5m,
            cache_write_1h: self.cache_write_1h + other.cache_write_1h,
            output: self.output + other.output,
            reasoning: self.reasoning + other.reasoning,
        }
    }
}

impl AddAssign for TokenCounts {
    fn add_assign(&mut self, other: TokenCounts) {
        *self = *self + other;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> TokenCounts {
        TokenCounts {
            input: Tokens(100),
            cache_read: Tokens(20),
            cache_write_5m: Tokens(3),
            cache_write_1h: Tokens(4),
            output: Tokens(50),
            reasoning: Tokens(30),
        }
    }

    #[test]
    fn total_excludes_reasoning_subset() {
        assert_eq!(sample().total(), Tokens(177));
    }

    #[test]
    fn total_of_empty_is_zero() {
        assert_eq!(TokenCounts::default().total(), Tokens::ZERO);
    }

    #[test]
    fn cache_write_sums_both_ttls() {
        assert_eq!(sample().cache_write(), Tokens(7));
    }

    #[test]
    fn add_is_field_wise() {
        let mut sum = sample();
        sum += sample();
        assert_eq!(sum.reasoning, Tokens(60));
        assert_eq!(sum.total(), Tokens(354));
    }
}
