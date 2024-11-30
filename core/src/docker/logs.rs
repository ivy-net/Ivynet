use chrono::{Datelike, NaiveDateTime};

/// regex for timestamp string in the format of "Nov 28 06:37:07.908"
const TIMESTAMP_REGEX: &str = r"^\w{3} \d{2} \d{2}:\d{2}:\d{2}\.\d{3}";

pub fn get_log_timestamp(log: &str) -> i64 {
    let re = regex::Regex::new(TIMESTAMP_REGEX).unwrap();
    if let Some(timestamp) = re.find(log) {
        let this_year = chrono::Utc::now().year();
        if let Ok(timestamp) = parse_timestamp(timestamp.as_str(), this_year) {
            return timestamp.and_utc().timestamp();
        }
    }
    chrono::Utc::now().timestamp()
}

pub fn get_log_level(log: &str) -> String {
    if log.contains("ERR") {
        "error".to_string()
    } else if log.contains("WRN") {
        "warning".to_string()
    } else if log.contains("INF") {
        "info".to_string()
    } else if log.contains("DBG") {
        "debug".to_string()
    } else {
        "unknown".to_string()
    }
}

// TODO: This should probably be a method of a struct that composes the log transformation process
fn parse_timestamp(timestamp: &str, this_year: i32) -> Result<NaiveDateTime, chrono::ParseError> {
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
    fn test_log_level_detection() {
        assert_eq!(get_log_level(ERR_LOG), "error");
        assert_eq!(get_log_level(DBG_LOG), "debug");
        assert_eq!(get_log_level(WRN_LOG), "warning");
        assert_eq!(get_log_level(INF_LOG), "info");
        assert_eq!(get_log_level(UNKNOWN_LOG), "unknown");
    }

    #[test]
    fn test_log_timestamp_parsing() {
        let this_year = 2024;
        let expected_timestamp =
            NaiveDateTime::parse_from_str("2024 Nov 28 06:43:08.470", "%Y %b %d %H:%M:%S.%3f")
                .unwrap();
        assert_eq!(parse_timestamp("Nov 28 06:43:08.470", this_year).unwrap(), expected_timestamp);
    }
}
