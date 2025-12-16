use crate::SignalRole;
use once_cell::sync::Lazy;
use regex::Regex;

static STATUS_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^(caller|callee)/([^/]+)/status$").unwrap());
static SIGNAL_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^(caller|callee)/([^/]+)/signal$").unwrap());

pub fn get_status_topic(id: &str, role: SignalRole) -> String {
    format!("{}/{}/status", role.as_ref(), id)
}

pub fn get_signal_topic(id: &str, role: SignalRole) -> String {
    format!("{}/{}/signal", role.as_ref(), id)
}

pub fn split_status_topic(topic: &str) -> Option<String> {
    STATUS_RE.captures(topic).and_then(|caps| caps.get(2).map(|m| m.as_str().to_string()))
}

pub fn split_signal_topic(topic: &str) -> Option<String> {
    SIGNAL_RE.captures(topic).and_then(|caps| caps.get(2).map(|m| m.as_str().to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SignalRole;

    #[test]
    fn test_get_topics() {
        assert_eq!(get_status_topic("abc", SignalRole::Caller), "caller/abc/status");
        assert_eq!(get_status_topic("xyz", SignalRole::Callee), "callee/xyz/status");
        assert_eq!(get_signal_topic("123", SignalRole::Caller), "caller/123/signal");
        assert_eq!(get_signal_topic("999", SignalRole::Callee), "callee/999/signal");
    }

    #[test]
    fn test_split_status_topic() {
        assert_eq!(split_status_topic("caller/abc/status"), Some("abc".to_string()));
        assert_eq!(split_status_topic("callee/xyz/status"), Some("xyz".to_string()));
        assert_eq!(split_status_topic("invalid/topic"), None);
    }

    #[test]
    fn test_split_signal_topic() {
        assert_eq!(split_signal_topic("caller/123/signal"), Some("123".to_string()));
        assert_eq!(split_signal_topic("callee/999/signal"), Some("999".to_string()));
        assert_eq!(split_signal_topic("nope/aaa"), None);
    }
}
