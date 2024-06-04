use std::path::PathBuf;

use once_cell::sync::Lazy;

pub const IVY_METADATA: &str = "https://eigenoperator.s3.us-east-2.amazonaws.com/metadata.json";
pub const IVY_LOGO: &str = "https://eigenoperator.s3.us-east-2.amazonaws.com/Group+30_10x.png";
pub const IVY_FAVICON: &str = "https://eigenoperator.s3.us-east-2.amazonaws.com/favicon.png";

pub static IVY_PATH: Lazy<PathBuf> = Lazy::new(|| {
    let mut ivy_path = dirs::home_dir().expect("Could not get home directory");
    ivy_path.push(".ivynet");
    ivy_path
});
