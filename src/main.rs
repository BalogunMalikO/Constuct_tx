use bitcoin::{
    ecdsa::Signature,
    hashes::Hash,
    locktime::absolute,
    secp256k1::{rand, All, Message, Secp256k1, SecretKey},
    sighash::{EcdsaSighashType, SighashCache},
    Address, Amount, Network, OutPoint, PublicKey, ScriptBuf, Sequence, Transaction, TxIn, TxOut,
    Txid, WPubkeyHash, Witness,
};

fn main() {
    let tx_exec = TxExec::new();
    let tx_out = tx_exec.build();
    let tx_spend = tx_exec.spend(&tx_out);

    dbg!(tx_exec.receiver_address());
    dbg!(tx_exec.sender_address());
    dbg!(&tx_out);
    dbg!(&tx_spend);
}

pub struct TxExec {
    network: Network,
    tx_version: bitcoin::transaction::Version,
    secp: Secp256k1<All>,
    secret_key: SecretKey,
    public_key: PublicKey,
    wpkh: WPubkeyHash,
    receiver_secret_key: SecretKey,
    receiver_public_key: PublicKey,
}

impl TxExec {
    pub fn new() -> Self {
        let secp = Secp256k1::new();
        let secret_key = SecretKey::new(&mut rand::thread_rng());
        let public_key = PublicKey::new(secret_key.public_key(&secp));
        let wpkh = public_key.wpubkey_hash().unwrap();
        let receiver_secret_key = SecretKey::new(&mut rand::thread_rng());
        let receiver_public_key =
            bitcoin::PublicKey::new(receiver_secret_key.public_key(&secp));

        Self {
            network: Network::Regtest,
            tx_version: bitcoin::transaction::Version::TWO,
            secp,
            secret_key,
            public_key,
            wpkh,
            receiver_secret_key,
            receiver_public_key,
        }
    }

    pub fn receiver_address(&self) -> Address {
        Address::p2pkh(&self.receiver_public_key, self.network)
    }

    pub fn sender_address(&self) -> Address {
        Address::p2pkh(&self.public_key, self.network)
    }

    pub fn build(&self) -> Transaction {
        let (previous_outpoint, previous_utxo) = dummy_utxo(&self.wpkh);
        let script_code = previous_utxo.script_pubkey.p2wpkh_script_code().unwrap();

//transaction input
        let input = TxIn {
            previous_output: previous_outpoint,
            script_sig: ScriptBuf::new(),
            sequence: Sequence::ENABLE_LOCKTIME_NO_RBF,
            witness: Witness::default(),
        };

        let spend = TxOut {
            value: SPEND_AMOUNT,
            script_pubkey: self.receiver_address().script_pubkey(),
        };
// the change output
        let change = TxOut {
            value: CHANGE_AMOUNT,
            script_pubkey: ScriptBuf::new_p2wpkh(&self.wpkh),
        };

        let unsigned_tx = Transaction {
            version: self.tx_version,
            lock_time: absolute::LockTime::ZERO,
            input: vec![input],
            output: vec![spend, change],
        };
//sign the unsigned transaction
        let mut sighash_cache = SighashCache::new(unsigned_tx);
        let sighash = sighash_cache
            .p2wsh_signature_hash(0, &script_code, DUMMY_UTXO_AMOUNT, EcdsaSighashType::All)
            .unwrap();

        let msg = Message::from(sighash);
        let sig = self.secp.sign_ecdsa(&msg, &self.secret_key);

//convert into a transaction
        let mut tx = sighash_cache.into_transaction();

        let pk = self.secret_key.public_key(&self.secp);
        let witness = &mut tx.input[0].witness;
        witness.push_ecdsa_signature(&Signature {
            sig,
            hash_ty: EcdsaSighashType::All,
        });
        witness.push(pk.serialize());

        tx
    }

    pub fn spend(&self, previous_tx: &Transaction) -> Transaction {
        let previous_tx_out = &previous_tx.output[0];
        let current_balance = previous_tx_out.value;
        let fee = Amount::from_sat(1000);
        let current_spend = current_balance.checked_sub(fee).unwrap();

        let spend = TxOut {
            value: current_spend,
            script_pubkey: self.receiver_address().script_pubkey(),
        };

        let input = TxIn {
            previous_output: OutPoint {
                txid: previous_tx.txid(),
                vout: 0,
            },
            script_sig: previous_tx.output[0].script_pubkey.clone(),
            sequence: Sequence::ENABLE_LOCKTIME_NO_RBF,
            witness: Witness::default(),
        };

        let unsigned_tx = Transaction {
            version: self.tx_version,
            lock_time: absolute::LockTime::ZERO,
            input: vec![input.clone()],
            output: vec![spend],
        };

        let mut sighash_cache = SighashCache::new(unsigned_tx);
        let sighash = sighash_cache
            .p2wsh_signature_hash(
                0,
                &previous_tx_out.script_pubkey,
                DUMMY_UTXO_AMOUNT,
                EcdsaSighashType::All,
            )
            .unwrap();

        let msg = Message::from(sighash);
        let sig = self.secp.sign_ecdsa(&msg, &self.receiver_secret_key);

        let mut tx = sighash_cache.into_transaction();

        let pk = self.secret_key.public_key(&self.secp);
        let witness = &mut tx.input[0].witness;
        witness.push_ecdsa_signature(&Signature {
            sig,
            hash_ty: EcdsaSighashType::All,
        });
        witness.push(pk.serialize());

        tx
    }
}

fn dummy_utxo(wpkh: &WPubkeyHash) -> (OutPoint, TxOut) {
    let script_pubkey = ScriptBuf::new_p2wpkh(wpkh);

    let out_point = OutPoint {
        txid: Txid::all_zeros(), // not invalid.
        vout: 0,
    };

    let utxo = TxOut {
        value: DUMMY_UTXO_AMOUNT,
        script_pubkey,
    };

    (out_point, utxo)
}

const DUMMY_UTXO_AMOUNT: Amount = Amount::from_sat(20_000_000);
const SPEND_AMOUNT: Amount = Amount::from_sat(5_000_000);
const CHANGE_AMOUNT: Amount = Amount::from_sat(14_999_000); // remove 1000 satoshis fee

