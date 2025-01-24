use std::str::FromStr;

use regex::Regex;

/// Regex for parsing netstat -tupln output.
/// ^(\S+)                 Protocol (e.g., "tcp", "tcp6")
/// \s+(\d+)               Recv-Q
/// \s+(\d+)               Send-Q
/// \s+(\S+)               Local Address/Port (e.g., "0.0.0.0:80" or "::1:631")
/// \s+(\S+)               Foreign Address/Port (e.g., "0.0.0.0:*")
/// \s+(\S+)               State (e.g., "LISTEN")
/// \s+(.+)                PID/Program name (e.g., "1/nginx: master pro")
///
/// We capture the last group with (.+) because the program name can contain spaces.
const NETSTAT_REGEX: &str = r"^(?P<proto>\S+)\s+(?P<recv>\d+)\s+(?P<send>\d+)\s+(?P<local>\S+)\s+(?P<foreign>\S+)\s+(?P<state>\S+)\s+(?P<pidprog>.+)$";

/// A structured representation of one line of `netstat -tupln` output.
#[derive(Clone, Debug)]
#[allow(dead_code)]
pub(crate) struct NetstatEntry {
    pub protocol: String,
    pub recv_q: u64,
    pub send_q: u64,
    pub local_address: String,
    pub local_port: String,
    pub foreign_address: String,
    pub foreign_port: String,
    pub state: String,
    pub pid: String,
    pub program: String,
}

#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    #[error("Invalid netstat line")]
    InvalidLine,
    #[error("Regex error: {0}")]
    RegexError(#[from] regex::Error),
    #[error("Parse INT error: {0}")]
    ParseIntError(#[from] std::num::ParseIntError),
}

impl FromStr for NetstatEntry {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        parse_netstat_line(s)
    }
}

/// Parse a single line from `netstat -tupln` into a `NetstatEntry`.
///
/// Returns `None` if the line does not match the expected format.
fn parse_netstat_line(line: &str) -> Result<NetstatEntry, ParseError> {
    let line = line.trim_end(); // strip trailing whitespace and \r if present
    let re = Regex::new(NETSTAT_REGEX)?;

    let caps = re.captures(line).ok_or(ParseError::InvalidLine)?;

    // Convert numeric fields
    let recv_q = caps["recv"].parse()?;
    let send_q = caps["send"].parse()?;

    // Split local/foreign into address and port
    let (local_address, local_port) = split_address_port(&caps["local"]);
    let (foreign_address, foreign_port) = split_address_port(&caps["foreign"]);

    // Split PID/program
    let (pid, program) = split_pid_program(&caps["pidprog"]);

    Ok(NetstatEntry {
        protocol: caps["proto"].to_string(),
        recv_q,
        send_q,
        local_address,
        local_port,
        foreign_address,
        foreign_port,
        state: caps["state"].to_string(),
        pid,
        program,
    })
}

/// Split an address string like "0.0.0.0:80" or ":::80" into (address, port).
fn split_address_port(addr_port: &str) -> (String, String) {
    // Find the *last* colon so IPv6 addresses (with multiple colons) still work.
    // Example: ":::80" => ip="::", port="80"
    // Example: "127.0.0.1:1234" => ip="127.0.0.1", port="1234"
    // Example: "0.0.0.0:*" => ip="0.0.0.0", port="*"
    if let Some(idx) = addr_port.rfind(':') {
        let ip = &addr_port[..idx];
        let port = &addr_port[idx + 1..];
        (ip.to_string(), port.to_string())
    } else {
        // If there's no colon, just return as-is
        (addr_port.to_string(), "".to_string())
    }
}

/// Split a PID/Program string like "1/nginx: master pro" into (pid, program).
/// If there's no slash, returns (empty, entire string).
fn split_pid_program(pidprog: &str) -> (String, String) {
    if let Some(idx) = pidprog.find('/') {
        let pid = &pidprog[..idx];
        let program = &pidprog[idx + 1..];
        (pid.to_string(), program.to_string())
    } else {
        ("".to_string(), pidprog.to_string())
    }
}
