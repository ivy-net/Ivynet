use std::error::Error;
use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;

#[derive(Debug, Clone)]
enum EnvLine {
    KeyValue(String, String),
    Comment(String),
    Blank,
}

pub struct EnvLines {
    lines: Vec<EnvLine>,
}

impl EnvLines {
    pub fn load(path: &Path) -> Result<Self, Box<dyn Error>> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        let mut lines = Vec::new();

        for line in reader.lines() {
            let line = line?;
            if line.trim().starts_with('#') {
                lines.push(EnvLine::Comment(line));
            } else if line.trim().is_empty() {
                lines.push(EnvLine::Blank);
            } else {
                let parts: Vec<&str> = line.splitn(2, '=').collect();
                if parts.len() == 2 {
                    lines.push(EnvLine::KeyValue(parts[0].trim().to_string(), parts[1].trim().to_string()));
                } else {
                    lines.push(EnvLine::Comment(line)); // Treat malformed lines as comments
                }
            }
        }

        Ok(Self { lines })
    }

    pub fn save(&self, path: &Path) -> Result<(), Box<dyn Error>> {
        let mut file = File::create(path)?;
        for line in &self.lines {
            match line {
                EnvLine::KeyValue(key, value) => writeln!(file, "{}={}", key, value)?,
                EnvLine::Comment(comment) => writeln!(file, "{}", comment)?,
                EnvLine::Blank => writeln!(file)?,
            }
        }
        Ok(())
    }

    pub fn set(&mut self, key: &str, value: &str) {
        let mut found = false;
        for line in &mut self.lines {
            if let EnvLine::KeyValue(k, v) = line {
                if k == key {
                    *v = value.to_string();
                    found = true;
                    break;
                }
            }
        }
        if !found {
            self.lines.push(EnvLine::KeyValue(key.to_string(), value.to_string()));
        }
    }

    pub fn delete(&mut self, key: &str) {
        self.lines.retain(|line| if let EnvLine::KeyValue(k, _) = line { k != key } else { true });
    }
}
