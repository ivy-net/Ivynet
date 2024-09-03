use std::{
    ffi::OsStr,
    process::{Child, Command, ExitStatus, Stdio},
};

pub fn docker_cmd<I, S>(args: I) -> Result<Child, std::io::Error>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    if which::which("docker-compose").is_ok() {
        Command::new("docker-compose").args(args).spawn()
    } else {
        Command::new("docker").arg("compose").args(args).spawn()
    }
}

pub fn docker_cmd_status<I, S>(args: I) -> Result<ExitStatus, std::io::Error>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    if which::which("docker-compose").is_ok() {
        Command::new("docker-compose").args(args).status()
    } else {
        Command::new("docker").arg("compose").args(args).status()
    }
}

pub fn docker_cmd_logs<I, S>(args: I) -> Result<Child, std::io::Error>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    if which::which("docker-compose").is_ok() {
        Command::new("docker-compose")
            .args(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
    } else {
        Command::new("docker")
            .arg("compose")
            .args(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
    }
}
