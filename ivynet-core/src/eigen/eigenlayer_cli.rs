// curl -sSfL https://raw.githubusercontent.com/layr-labs/eigenlayer-cli/master/scripts/install.sh | sh -s

use ethers_core::types::transaction::request;

const EIGENLAYER_SOURCE: &str = "https://raw.githubusercontent.com/layr-labs/eigenlayer-cli/master/scripts/install.sh";

fn get_eigenlayer_cli() {
    let install = reqwest::get(EIGENLAYER_SOURCE).await?;
    println!("{}", install);
}
