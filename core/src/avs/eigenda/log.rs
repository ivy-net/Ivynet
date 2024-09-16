use regex::Regex;

#[allow(dead_code)]
pub fn container_regex() -> Regex {
    Regex::new(r"^(\S+)\s+\|").unwrap()
}

#[allow(dead_code)]
pub fn datetime_regex() -> Regex {
    Regex::new(r"\s(\w{4} \d{2} \d{2}:\d{2}:\d{2}\.\d{3})\s").unwrap()
}

pub fn level_regex() -> Regex {
    Regex::new(r"\s(DBG|INF|WRN|ERR)\s").unwrap()
}

pub fn ansi_sanitization_regex() -> Regex {
    Regex::new(r"\x1B\[([0-9]{1,2}(;[0-9]{1,2})*)?[m]").unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_container_regex() {
        let container_regex = Regex::new(r"^(\S+)\s+\|").unwrap();

        // Test valid container names
        assert_eq!(
            container_regex
                .captures("eigenda-native-node | Some log message")
                .unwrap()
                .get(1)
                .unwrap()
                .as_str(),
            "eigenda-native-node"
        );
        assert_eq!(
            container_regex
                .captures("reverse-proxy-1 | Another log message")
                .unwrap()
                .get(1)
                .unwrap()
                .as_str(),
            "reverse-proxy-1"
        );

        // Test invalid formats
        assert!(container_regex.captures("Invalid log format").is_none());
        assert!(container_regex.captures("| No container name").is_none());
    }

    #[test]
    fn test_log_level_regex() {
        let log_level_regex = Regex::new(r"\s(DBG|INF|WRN|ERR)\s").unwrap();

        // Test valid log levels
        assert_eq!(
            log_level_regex
                .captures("Sep 11 19:41:58.858 DBG eth/tx.go:833")
                .unwrap()
                .get(1)
                .unwrap()
                .as_str(),
            "DBG"
        );
        assert_eq!(
            log_level_regex
                .captures(r#"eigenda-native-node  | Sep 11 19:42:00.531 INF grpc/server.go:100 port 32005=address [::]:32005="GRPC Listening""#)
                .unwrap()
                .get(1)
                .unwrap()
                .as_str(),
            "INF"
        );
        assert_eq!(
            log_level_regex
                .captures("Sep 11 19:42:00.530 WRN some_warning")
                .unwrap()
                .get(1)
                .unwrap()
                .as_str(),
            "WRN"
        );
        assert_eq!(
            log_level_regex
                .captures("Sep 11 19:42:00.530 ERR some_error")
                .unwrap()
                .get(1)
                .unwrap()
                .as_str(),
            "ERR"
        );

        // Test invalid formats
        assert!(log_level_regex.captures("Sep 11 19:42:00.530 INFO node/node.go:158").is_none());
        assert!(log_level_regex.captures("No log level present").is_none());
    }
}
