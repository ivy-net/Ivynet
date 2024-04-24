use clap::Parser;

#[derive(Parser, Debug, Clone)]
pub enum AvsCommands {
    #[command(name = "todo", about = "todo")]
    Todo { private_key: String },
}