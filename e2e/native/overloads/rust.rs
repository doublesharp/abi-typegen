mod overloads;

#[cfg(test)]
mod tests {
    use super::overloads::Overloads;
    use alloy::sol_types::SolCall;

    fn check<C: SolCall + Default>(signature: &str, selector: [u8; 4]) {
        assert_eq!(C::SIGNATURE, signature);
        assert_eq!(C::SELECTOR, selector);
        assert_eq!(C::default().abi_encode()[..4], selector);
    }

    #[test]
    fn aliases_encode_the_original_selectors() {
        check::<Overloads::safeTransferFrom_0Call>(
            "safeTransferFrom(address,address,uint256)",
            [0x42, 0x84, 0x2e, 0x0e],
        );
        check::<Overloads::safeTransferFrom_1Call>(
            "safeTransferFrom(address,address,uint256,bytes)",
            [0xb8, 0x8d, 0x4f, 0xde],
        );
        check::<Overloads::attestations_0Call>(
            "attestations(bytes32,bytes32)",
            [0x92, 0xab, 0xd1, 0x7b],
        );
        check::<Overloads::attestations_1Call>("attestations(bytes32)", [0x94, 0x09, 0x92, 0xa3]);
        check::<Overloads::setAddr_0Call>(
            "setAddr(bytes32,uint256,bytes)",
            [0x8b, 0x95, 0xdd, 0x71],
        );
        check::<Overloads::setAddr_1Call>("setAddr(bytes32,address)", [0xd5, 0xfa, 0x2b, 0x00]);
    }
}
