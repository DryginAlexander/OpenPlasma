use crate::account::AccountState;
use sapling_crypto_ce::eddsa::Signature;

use super::super::{
    tree::account::AccountsTree,
};

use crate::utils::utils::{
    optionalize,
    fr_to_usize,
    usize_to_fr,
    fr_to_bytes_le,
};

use sapling_crypto_ce::{
    eddsa::{
        PrivateKey,
        PublicKey,
    },
    poseidon::{
        poseidon_hash,
        bn256::Bn256PoseidonParams,
    },
    jubjub::FixedGenerators,
    alt_babyjubjub::AltJubjubBn256,
};

use pairing_ce::{
    bn256,
    bn256::Bn256,
};

use rand::thread_rng;

pub const NUM_BYTES_TO_SIGN: usize = 31;

#[derive(Clone)]
pub struct OffchainWithdrawal {
    pub account_id: usize,
    pub amount: usize,
    pub nonce: usize,
    pub sign: Option<Signature::<Bn256>>,
}

impl OffchainWithdrawal {

    pub fn hash(
        & self, 
        hash_params: &Bn256PoseidonParams
    ) -> bn256::Fr {
        let request = vec![
            usize_to_fr(self.account_id),
            usize_to_fr(self.amount),
            usize_to_fr(self.nonce),
        ];
    
        let hash_vec = poseidon_hash::<Bn256>(hash_params, &request);
        hash_vec[0]
    }

    pub fn sign(
        &mut self,
        seckey: &PrivateKey::<Bn256>,
        hash_params: &Bn256PoseidonParams,
        sign_params: &AltJubjubBn256,
    ) {
        let hash = self.hash(hash_params);
        let hash_bytes: Vec<_> = fr_to_bytes_le(hash, NUM_BYTES_TO_SIGN);
        let mut rng = thread_rng();

        let sign = seckey.sign_raw_message(
            &hash_bytes,
            &mut rng,
            FixedGenerators::SpendingKeyGenerator,
            sign_params,
            NUM_BYTES_TO_SIGN,
        );

        self.sign = Some(sign);
    }

    pub fn verify_signature(
        & self,
        pubkey: &PublicKey::<Bn256>,
        hash_params: &Bn256PoseidonParams,
        sign_params: &AltJubjubBn256,
    ) -> bool {
        let hash = self.hash(hash_params);
        let hash_bytes: Vec<_> = fr_to_bytes_le(hash, NUM_BYTES_TO_SIGN);

        pubkey.verify_for_raw_message(
            &hash_bytes,
            &self.sign.clone().unwrap(),
            FixedGenerators::SpendingKeyGenerator,
            sign_params,
            NUM_BYTES_TO_SIGN,
        )
    }

    pub fn update_tree_and_record_state(
        &self,
        tree: &mut AccountsTree,
    ) -> AccountState::<Bn256> {
        assert!(self.account_id < tree.accounts.len());

        // count balances
        let old_balance = tree.accounts[self.account_id].balance;
        let new_balance = {
            let old_balance = fr_to_usize(old_balance);
            assert!(old_balance >= self.amount);
            usize_to_fr(old_balance - self.amount)
        };

        // prepare paths, indices, pubkeys, nonces
        let pubkey = tree.accounts[self.account_id].pubkey.clone();
        let old_nonce = tree.accounts[self.account_id].nonce;
        assert!(fr_to_usize(old_nonce) == self.nonce - 1);
        let new_nonce = usize_to_fr(self.nonce);
        let account_path = tree.accounts_tree.get_leaf_path(self.account_id);
        let account_indices = tree.accounts_tree.get_leaf_indices(self.account_id);

        // update balance
        tree.update_balance(
            self.account_id,
            new_balance,
        );
        
        tree.update_account(
            self.account_id,
            tree.accounts[self.account_id].pubkey.clone(),
            new_nonce,
        );

        // record account state
        AccountState::<Bn256> {
            old_balance: Some(old_balance),
            new_balance: Some(new_balance),
            old_pubkey: Some(pubkey.0.clone()),
            new_pubkey: Some(pubkey.0),
            old_nonce: Some(old_nonce),
            new_nonce: Some(new_nonce),
            account_path: optionalize(account_path),
            account_indices: optionalize(account_indices),
        }
    }
}