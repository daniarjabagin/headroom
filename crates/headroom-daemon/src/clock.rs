use jiff::Timestamp;

pub trait Clock: Send + Sync {
    fn now(&self) -> Timestamp;
}

#[derive(Debug, Clone, Copy, Default)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> Timestamp {
        Timestamp::now()
    }
}

#[cfg(test)]
pub(crate) mod testing {
    use std::sync::Mutex;

    use jiff::{SignedDuration, Timestamp};

    use super::Clock;

    pub struct ManualClock(Mutex<Timestamp>);

    impl ManualClock {
        pub fn at(text: &str) -> ManualClock {
            ManualClock(Mutex::new(text.parse().unwrap()))
        }

        pub fn advance(&self, by: SignedDuration) {
            let mut now = self.0.lock().unwrap();
            *now = now.checked_add(by).unwrap();
        }
    }

    impl Clock for ManualClock {
        fn now(&self) -> Timestamp {
            *self.0.lock().unwrap()
        }
    }
}
