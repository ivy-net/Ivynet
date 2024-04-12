use ethers_providers::{Http, Provider};
use url::Url;

pub fn try_connection() -> Provider<Http>{
    // Initialize a new HTTP Client with authentication
    let url = Url::parse("http://localhost:8545").expect("Could not parse URL");
    let provider = Provider::new(Http::new(url));

    provider
}