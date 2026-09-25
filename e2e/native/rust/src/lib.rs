//! Compile check for generated Rust bindings.

pub mod contracts;

#[cfg(test)]
mod tests {
    use crate::contracts::{TupleCases, Vault};
    use alloy::primitives::{Address, U256};
    use alloy::sol_types::{SolCall, SolEvent};

    #[test]
    fn selectors_match_the_embedded_abi() {
        let abi: alloy::json_abi::JsonAbi =
            serde_json::from_str(crate::contracts::token::TOKEN_ABI).unwrap();
        let transfer = &abi.function("transfer").unwrap()[0];
        assert_eq!(
            transfer.selector(),
            crate::contracts::Token::transferCall::SELECTOR
        );
    }

    #[test]
    fn tuples_are_named_with_value_semantics_and_abi_field_names() {
        let position = TupleCases::TupleAccountPosition {
            account: Address::repeat_byte(1),
        };
        assert_eq!(position.clone(), position);
        let call = TupleCases::deposit_0Call { position };
        let decoded = TupleCases::deposit_0Call::abi_decode(&call.abi_encode()).unwrap();
        assert_eq!(decoded.position, call.position);

        let vault = Vault::Position {
            shares: U256::from(1),
            depositedAt: 2,
            token: Address::ZERO,
        };
        let json = serde_json::to_value(&vault).unwrap();
        assert!(json.get("depositedAt").is_some(), "{json}");
        assert_eq!(Vault::Position::default().shares, U256::ZERO);
    }

    #[test]
    fn events_keep_indexed_fields() {
        let moved = TupleCases::Moved {
            amount: U256::from(5),
            from: Address::repeat_byte(2),
            memo: Default::default(),
            to: Address::repeat_byte(3),
        };
        let log = moved.encode_log_data();
        assert_eq!(log.topics().len(), 3);
        assert_eq!(log.topics()[0], TupleCases::Moved::SIGNATURE_HASH);
    }

    #[tokio::test]
    async fn generated_bindings_submit_and_decode_on_anvil() {
        use alloy::network::EthereumWallet;
        use alloy::providers::{Provider, ProviderBuilder};
        use alloy::signers::local::PrivateKeySigner;
        use std::time::Duration;

        let Ok(rpc) = std::env::var("ATG_RPC_URL") else {
            return;
        };
        let address: Address = std::env::var("ATG_TOKEN_ADDRESS").unwrap().parse().unwrap();
        let signer: PrivateKeySigner = std::env::var("ATG_PRIVATE_KEY").unwrap().parse().unwrap();
        let expected_chain: u64 = std::env::var("ATG_CHAIN_ID").unwrap().parse().unwrap();
        let owner = signer.address();
        let spender = Address::repeat_byte(0x42);
        let provider = ProviderBuilder::new()
            .wallet(EthereumWallet::from(signer))
            .connect_http(rpc.parse().unwrap());
        let contract = crate::contracts::Token::new(address, provider);

        tokio::time::timeout(Duration::from_secs(30), async {
            assert_eq!(
                contract.provider().get_chain_id().await.unwrap(),
                expected_chain
            );
            let minted = U256::from(73);
            let mint_receipt = contract
                .mint(owner, minted)
                .send()
                .await
                .unwrap()
                .get_receipt()
                .await
                .unwrap();
            assert!(mint_receipt.status());
            assert_eq!(contract.balanceOf(owner).call().await.unwrap(), minted);
            let transfer = mint_receipt
                .inner
                .logs()
                .iter()
                .find_map(|log| crate::contracts::Token::Transfer::decode_log(&log.inner).ok())
                .expect("mint Transfer log");
            assert_eq!(transfer.to, owner);
            assert_eq!(transfer.amount, minted);

            let approved = U256::from(29);
            let approval_receipt = contract
                .approve(spender, approved)
                .send()
                .await
                .unwrap()
                .get_receipt()
                .await
                .unwrap();
            assert!(approval_receipt.status());
            assert_eq!(
                contract.allowance(owner, spender).call().await.unwrap(),
                approved
            );
            let approval = approval_receipt
                .inner
                .logs()
                .iter()
                .find_map(|log| crate::contracts::Token::Approval::decode_log(&log.inner).ok())
                .expect("Approval log");
            assert_eq!(approval.owner, owner);
            assert_eq!(approval.spender, spender);
            assert_eq!(approval.amount, approved);
        })
        .await
        .expect("Anvil consumer test timed out");
    }
}
