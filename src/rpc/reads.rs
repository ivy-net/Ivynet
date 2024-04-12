use ethers_core::types::{Address, TransactionRequest};
use ethers_middleware::SignerMiddleware;
use ethers_providers::{Http, Middleware, Provider};
use ethers_signers::LocalWallet;
use std::convert::TryFrom;

pub fn example_function() {
    let provider = Provider::<Http>::try_from("http://localhost:8545").expect("Failed to instantiate provider");

    // Transactions will be signed with the private key below and will be broadcast
    // via the eth_sendRawTransaction API)
    let wallet: LocalWallet = "380eb0f3d505f087e438eca80bc4df9a7faa24f868e69fc0440261a0fc0567dc".parse()?;

    let mut client = SignerMiddleware::new(provider, wallet);

    // You can sign messages with the key
    let signed_msg = client.sign(b"hello".to_vec(), &client.address()).await?;

    // ...and sign transactions
    let tx = TransactionRequest::pay("vitalik.eth", 100);
    let pending_tx = client.send_transaction(tx, None).await?;

    // You can `await` on the pending transaction to get the receipt with a pre-specified
    // number of confirmations
    let receipt = pending_tx.confirmations(6).await?;

    // You can connect with other wallets at runtime via the `with_signer` function
    let wallet2: LocalWallet = "cd8c407233c0560f6de24bb2dc60a8b02335c959a1a17f749ce6c1ccf63d74a7".parse()?;

    let signed_msg2 = client
        .with_signer(wallet2)
        .sign(b"hello".to_vec(), &client.address())
        .await?;

    // This call will be made with `wallet2` since `with_signer` takes a mutable reference.
    let tx2 = TransactionRequest::new()
        .to("0xd8da6bf26964af9d7eed9e03e53415d37aa96045".parse::<Address>()?)
        .value(200);
    let tx_hash2 = client.send_transaction(tx2, None).await?;
}

// Need to built out restake math
