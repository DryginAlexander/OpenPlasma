use bellman_ce::{
    Circuit,
    ConstraintSystem,
    SynthesisError,
};

use sapling_crypto_ce::{
    poseidon::{
        PoseidonEngine,
        QuinticSBox,
    },
    jubjub::{
        JubjubEngine,
        edwards::Point,
        Unknown,
    },
    circuit::{
        num::AllocatedNum,
        poseidon_hash::poseidon_hash,
    },
};

use super::account::{ AccountState, AccountCircuit };
use super::utils::calc::check_decomposition_le;

#[derive(Clone)]
pub struct DepositCircuit<E: JubjubEngine + PoseidonEngine> {
    pub account_state: AccountState<E>,
    pub pubkey: Option::<Point<E, Unknown>>,
    pub account_id: Option::<E::Fr>,
    pub amount: Option::<E::Fr>,
}

impl<E> DepositCircuit<E>
    where E: JubjubEngine + PoseidonEngine<SBox = QuinticSBox<E>>,
{
    pub fn process_deposit<'a, CS: ConstraintSystem<E>> (
        &self,
        mut cs: CS,
        account_depth: usize,
        hash_params: &'a <E as PoseidonEngine>::Params,
        old_hash: &AllocatedNum<E>,
        old_root: &AllocatedNum<E>,
    ) -> Result<(AllocatedNum<E>, AllocatedNum<E>), SynthesisError> {
        // allocate circuit
        let account_circuit = AccountCircuit::new(
            cs.namespace(|| "allocate account circuit"),
            account_depth,
            hash_params,
            &self.account_state,
        )?;

        let (pubkey_x, pubkey_y) = match &self.pubkey {
            Some(point) => {
                let (x, y) = point.into_xy();
                (Some(x), Some(y))
            },
            None => (None, None),
        };

        let pubkey_x_alloc = AllocatedNum::alloc(
            cs.namespace(|| "allocate pubkey x"),
            || pubkey_x.ok_or(SynthesisError::AssignmentMissing),
        )?;

        let pubkey_y_alloc = AllocatedNum::alloc(
            cs.namespace(|| "allocate pubkey y"),
            || pubkey_y.ok_or(SynthesisError::AssignmentMissing),
        )?;

        let account_id_alloc = AllocatedNum::alloc(
            cs.namespace(|| "allocate account id"),
            || self.account_id.ok_or(SynthesisError::AssignmentMissing),
        )?;

        let amount_alloc = AllocatedNum::alloc(
            cs.namespace(|| "allocate amount"),
            || self.amount.ok_or(SynthesisError::AssignmentMissing),
        )?;

        // check pubkey consistence

        cs.enforce(
            || "check pubkey x consistence",
            |lc| lc + pubkey_x_alloc.get_variable(),
            |lc| lc + CS::one(),
            |lc| lc + account_circuit.accounts_tree.new_leaf_alloc[0].get_variable(),
        );

        cs.enforce(
            || "check pubkey y consistence",
            |lc| lc + pubkey_y_alloc.get_variable(),
            |lc| lc + CS::one(),
            |lc| lc + account_circuit.accounts_tree.new_leaf_alloc[1].get_variable(),
        );

        // check account id, asset id consistency

        check_decomposition_le(
            cs.namespace(|| "account id consistence"),
            &account_id_alloc,
            &account_circuit.accounts_tree.indices_alloc,
        )?;

        // check amount deposit

        cs.enforce(
            || "check amount deposit",
            |lc| lc + account_circuit.accounts_tree.old_leaf_alloc[3].get_variable()
                    + amount_alloc.get_variable(),
            |lc| lc + CS::one(),
            |lc| lc + account_circuit.accounts_tree.new_leaf_alloc[3].get_variable(),
        );

        // check nonce the same

        cs.enforce(
            || "check nonce the same",
            |lc| lc + account_circuit.accounts_tree.old_leaf_alloc[2].get_variable(),
            |lc| lc + CS::one(),
            |lc| lc + account_circuit.accounts_tree.new_leaf_alloc[2].get_variable(),
        );

        // calculate new hash

        let new_hash = {
            let hashes_vec = poseidon_hash(
                cs.namespace(|| "calculate new accum hash"),
                &[
                    old_hash.clone(),
                    pubkey_x_alloc,
                    pubkey_y_alloc,
                    account_id_alloc,
                    amount_alloc,
                ],
                hash_params,
            )?;
            hashes_vec[0].clone()
        };

        // verify old root & calculate new root

        account_circuit.accounts_tree.verify_old_root(
            cs.namespace(|| "verify old root"),
            old_root,
        )?;

        let new_root = account_circuit.accounts_tree.calc_new_root(
            cs.namespace(|| "calculate new root"),
        )?;

        Ok((new_hash, new_root))
    }
}

#[derive(Clone)]
pub struct DepositBatchCircuit<'a, E: JubjubEngine + PoseidonEngine> {
    pub deposit_batch: usize,
    pub account_depth: usize,
    pub hash_params: &'a <E as PoseidonEngine>::Params,

    pub deposit_queue: Vec::<DepositCircuit<E>>,
    pub old_accum_hash: Option::<E::Fr>,
    pub new_accum_hash: Option::<E::Fr>,
    pub old_account_root: Option::<E::Fr>,
    pub new_account_root: Option::<E::Fr>,
}

impl<'a, E> Circuit<E> for DepositBatchCircuit<'a, E>
    where E: JubjubEngine + PoseidonEngine<SBox = QuinticSBox<E>>,
{
    fn synthesize<CS: ConstraintSystem<E>> (
        self,
        cs: &mut CS,
    ) -> Result<(), SynthesisError> {
        assert_eq!(self.deposit_batch, self.deposit_queue.len());

        let mut prev_hash = AllocatedNum::alloc(
            cs.namespace(|| "allocate old accum hash"),
            || self.old_accum_hash.ok_or(SynthesisError::AssignmentMissing),
        )?;
        prev_hash.inputize(cs.namespace(|| "input old accum hash"))?;

        let new_hash = AllocatedNum::alloc(
            cs.namespace(|| "allocate new accum hash"),
            || self.new_accum_hash.ok_or(SynthesisError::AssignmentMissing),
        )?;
        new_hash.inputize(cs.namespace(|| "input new accum hash"))?;

        let mut prev_root = AllocatedNum::alloc(
            cs.namespace(|| "allocate old root"),
            || self.old_account_root.ok_or(SynthesisError::AssignmentMissing),
        )?;
        prev_root.inputize(cs.namespace(|| "input old root"))?;

        let new_root = AllocatedNum::alloc(
            cs.namespace(|| "allocate new root"),
            || self.new_account_root.ok_or(SynthesisError::AssignmentMissing),
        )?;
        new_root.inputize(cs.namespace(|| "input new root"))?;

        for (i, deposit) in self.deposit_queue.iter().enumerate() {
            let (hash, root) = deposit.process_deposit(
                cs.namespace(|| format!("verify deposit {}", i)),
                self.account_depth,
                self.hash_params,
                &prev_hash,
                &prev_root,
            )?;

            prev_hash = hash;
            prev_root = root;
        }

        cs.enforce(
            || "enforce new accum hash equivalence",
            |lc| lc + prev_hash.get_variable(),
            |lc| lc + CS::one(),
            |lc| lc + new_hash.get_variable(),
        );

        cs.enforce(
            || "enforce new root equivalence",
            |lc| lc + prev_root.get_variable(),
            |lc| lc + CS::one(),
            |lc| lc + new_root.get_variable(),
        );

        Ok(())
    }
}
