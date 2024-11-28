use chrono::{Datelike, NaiveDateTime};

/// regex for timestamp string in the format of "Nov 28 06:37:07.908"
const TIMESTAMP_REGEX: &str = r"^\w{3} \d{2} \d{2}:\d{2}:\d{2}\.\d{3}";

/// This function formats the docker log to find timestamp and log level in string and return a tuple of
/// the log, the timestamp, and the log level.
pub fn format_docker_log(log: &str) -> (String, String, String) {
    let timestamp = get_log_timestamp(log);
    let log_level = get_log_level(log);
    (log.to_string(), timestamp, log_level)
}

fn get_log_timestamp(log: &str) -> String {
    let re = regex::Regex::new(TIMESTAMP_REGEX).unwrap();
    if let Some(timestamp) = re.find(log) {
        if let Ok(timestamp) = parse_timestamp(timestamp.as_str()) {
            return timestamp.to_string();
        }
    }
    let now = chrono::Utc::now();
    #[allow(deprecated)]
    NaiveDateTime::from_timestamp(now.timestamp(), now.timestamp_subsec_nanos()).to_string()
}

pub fn get_log_level(log: &str) -> String {
    if log.contains("ERR") {
        "ERR".to_string()
    } else if log.contains("WRN") {
        "WRN".to_string()
    } else if log.contains("INF") {
        "INF".to_string()
    } else if log.contains("DBG") {
        "DBG".to_string()
    } else {
        "UNKNOWN".to_string()
    }
}

// WARN: Getting year internally is a hard antipattern when it comes to testing this func. Needs to
// be rewritten to be more flexible with year for tests.
fn parse_timestamp(timestamp: &str) -> Result<NaiveDateTime, chrono::ParseError> {
    let this_year = chrono::Utc::now().year();
    let timestamp_with_year = format!("{} {}", this_year, timestamp);
    NaiveDateTime::parse_from_str(&timestamp_with_year, "%Y %b %d %H:%M:%S.%3f")
}

#[cfg(test)]
mod test_log_parse {
    use super::*;

    const ERR_LOG: &str = r#"Nov 28 06:43:08.470 ERR node/metrics.go:241 Failed to query chain RPC for quorum bitmap component=NodeMetrics blockNumber=2829562 err="execution reverted: revert: RegCoord.getQuorumBitmapIndexAtBlockNumber: no bitmap update found for operator at blockNumber""#;
    const DBG_LOG: &str = r#"Nov 28 06:44:07.909 DBG node/node.go:739 Calling reachability check component=Node url="https://dataapi-holesky.eigenda.xyz/api/v1/operators-info/port-check?operator_id=EEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEE""#;
    const WRN_LOG: &str = r#"Nov 28 06:44:08.002 WRN node/node.go:750 Reachability check operator id not found component=Node status=404 operator_id=EEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEE"#;
    const INF_LOG: &str = r#"Nov 28 06:43:07.908 INF node/node.go:270 Complete an expiration cycle to remove expired batches component=Node "num expired batches found and removed"=0 "num expired mappings found and removed"=0 "num expired blobs found and removed"=0"#;
    const UNKNOWN_LOG: &str = r#"I'M A LUMBERJACK AND I'M OKAY!"#;

    #[test]
    fn test_invalid_timestamp_format() {
        let invalid_log = "Invalid timestamp format";
        // This will return current timestamp, but we can't assert exact value
        // since it may change. Just verify it returns something non-empty
        assert!(!get_log_timestamp(invalid_log).is_empty());
    }

    #[test]
    fn test_log_level_detection() {
        assert_eq!(get_log_level(ERR_LOG), "ERR");
        assert_eq!(get_log_level(DBG_LOG), "DBG");
        assert_eq!(get_log_level(WRN_LOG), "WRN");
        assert_eq!(get_log_level(INF_LOG), "INF");
        assert_eq!(get_log_level(UNKNOWN_LOG), "UNKNOWN");
    }

    #[test]
    fn test_format_docker_log() {
        let (log, timestamp, level) = format_docker_log(ERR_LOG);
        assert_eq!(log, ERR_LOG);
        assert!(!timestamp.is_empty());
        assert_eq!(level, "ERR");

        let (log, timestamp, level) = format_docker_log(INF_LOG);
        assert_eq!(log, INF_LOG);
        assert!(timestamp.is_empty());
        assert_eq!(level, "INF");
    }
}
