#[cfg(test)]
mod check_consistency {
    use crate::TxExec;
    use bitcoin::{transaction::Version, Amount};

    #[test]
    fn run_test() {
        let tx_exec = TxExec::new();

        let tx_out = tx_exec.build();
        let tx_spend = tx_exec.spend(&tx_out);

        assert_eq!(tx_out.version, Version::TWO);
        assert_eq!(tx_spend.version, Version::TWO);

        let previous_tx_out = tx_out.output[0].clone();
        let previous_tx_value = previous_tx_out.value;

        let current_tx_out = tx_spend.output[0].clone();
        let current_tx_value = current_tx_out.value;

        let fee = Amount::from_sat(1000);
        assert_eq!(
            current_tx_value,
            previous_tx_value.checked_sub(fee).unwrap()
        );

        assert!(previous_tx_out.script_pubkey.is_p2pkh());
        assert!(current_tx_out.script_pubkey.is_p2pkh());
    }
}