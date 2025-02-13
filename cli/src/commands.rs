use clap::Subcommand;
use std::fmt::Display;

#[derive(Subcommand, Debug)]
pub enum RegisterCommands {
    #[command(
        name = "optin",
        about = "Opt in to valid quorums with the given AVS. Valid for: EigenDA, AltLayer"
    )]
    Optin {},
    #[command(
        name = "optout",
        about = "Opt out of valid quorums with the given AVS. Valid for: EigenDA, AltLayer"
    )]
    Optout {},
    #[command(name = "register", about = "Register an operator. Valid for: Lagrange")]
    Register {},
}

#[derive(Subcommand, Debug)]
pub enum LogCommands {
    #[command(name = "stdout", about = "get all stdout logs")]
    STDOUT,
    #[command(name = "stderr", about = "get all stderr logs")]
    STDERR,
    #[command(name = "debug", about = "get debug logs from stdout")]
    DEBUG,
    #[command(name = "info", about = "get info logs from stdout")]
    INFO,
    #[command(name = "warn", about = "get warning logs from stdout")]
    WARN,
    #[command(name = "error", about = "get error logs from stdout")]
    ERROR,
}

impl Display for LogCommands {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LogCommands::STDOUT => write!(f, "stdout"),
            LogCommands::STDERR => write!(f, "stderr"),
            LogCommands::DEBUG => write!(f, "debug"),
            LogCommands::INFO => write!(f, "info"),
            LogCommands::WARN => write!(f, "warn"),
            LogCommands::ERROR => write!(f, "error"),
        }
    }
}
