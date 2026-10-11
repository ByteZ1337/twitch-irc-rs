use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

/// Which wanted channels the server has acknowledged
#[derive(Default)]
pub(crate) struct JoinTracker {
    confirmed: HashSet<String>,
    /// When the wait for an acknowledgement began, for wanted channels the server has not confirmed.
    unconfirmed: HashMap<String, Instant>,
}

impl JoinTracker {
    pub fn is_confirmed(&self, channel_login: &str) -> bool {
        self.confirmed.contains(channel_login)
    }

    pub fn confirm(&mut self, channel_login: &str) {
        self.unconfirmed.remove(channel_login);
        if !self.confirmed.contains(channel_login) {
            self.confirmed.insert(channel_login.to_owned());
        }
    }

    pub fn unconfirm(&mut self, channel_login: &str, now: Instant) {
        self.confirmed.remove(channel_login);
        if !self.unconfirmed.contains_key(channel_login) {
            self.unconfirmed.insert(channel_login.to_owned(), now);
        }
    }

    pub fn forget(&mut self, channel_login: &str) {
        self.confirmed.remove(channel_login);
        self.unconfirmed.remove(channel_login);
    }

    pub fn unconfirmed_for(&self, min_age: Duration, now: Instant) -> Vec<String> {
        self.unconfirmed
            .iter()
            .filter(|(_, since)| now.saturating_duration_since(**since) >= min_age)
            .map(|(channel_login, _)| channel_login.clone())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MINUTE: Duration = Duration::from_secs(60);

    #[test]
    fn confirmed_channel_is_joined_and_no_longer_waiting() {
        let start = Instant::now();
        let mut joins = JoinTracker::default();
        joins.unconfirm("forsen", start);
        joins.confirm("forsen");

        assert!(joins.is_confirmed("forsen"));
        assert_eq!(
            joins.unconfirmed_for(Duration::ZERO, start + 60 * MINUTE),
            Vec::<String>::new()
        );
    }

    #[test]
    fn resent_join_keeps_the_time_of_the_first_one() {
        let start = Instant::now();
        let mut joins = JoinTracker::default();
        joins.unconfirm("renamed", start);
        joins.unconfirm("renamed", start + 10 * MINUTE);

        assert_eq!(
            joins.unconfirmed_for(5 * MINUTE, start + 10 * MINUTE),
            vec!["renamed".to_owned()]
        );
    }

    #[test]
    fn channel_is_reported_only_once_it_has_waited_min_age() {
        let start = Instant::now();
        let mut joins = JoinTracker::default();
        joins.unconfirm("slow", start);

        assert_eq!(
            joins.unconfirmed_for(5 * MINUTE, start + 4 * MINUTE),
            Vec::<String>::new()
        );
        assert_eq!(
            joins.unconfirmed_for(5 * MINUTE, start + 5 * MINUTE),
            vec!["slow".to_owned()]
        );
    }

    #[test]
    fn forgotten_channel_is_neither_joined_nor_waiting() {
        let start = Instant::now();
        let mut joins = JoinTracker::default();
        joins.confirm("parted");
        joins.unconfirm("dropped", start);
        joins.forget("parted");
        joins.forget("dropped");

        assert!(!joins.is_confirmed("parted"));
        assert_eq!(
            joins.unconfirmed_for(Duration::ZERO, start + 60 * MINUTE),
            Vec::<String>::new()
        );
    }

    #[test]
    fn losing_confirmation_starts_the_wait_at_that_moment() {
        let start = Instant::now();
        let mut joins = JoinTracker::default();
        joins.confirm("kicked");
        joins.unconfirm("kicked", start + 60 * MINUTE);

        assert!(!joins.is_confirmed("kicked"));
        assert_eq!(
            joins.unconfirmed_for(5 * MINUTE, start + 64 * MINUTE),
            Vec::<String>::new()
        );
        assert_eq!(
            joins.unconfirmed_for(5 * MINUTE, start + 65 * MINUTE),
            vec!["kicked".to_owned()]
        );
    }
}
