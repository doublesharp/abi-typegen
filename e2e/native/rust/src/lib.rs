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
}
