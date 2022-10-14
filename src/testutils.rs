#![cfg(any(test, feature = "testutils"))]

use crate::{Auth, VaultContractClient};
use soroban_auth::{Identifier, Signature};

use soroban_sdk::{AccountId, BigInt, BytesN, Env};

pub fn register_test_contract(e: &Env, contract_id: &[u8; 32]) {
    let contract_id = BytesN::from_array(e, contract_id);
    e.register_contract(&contract_id, crate::VaultContract {});
}

pub struct VaultContract {
    env: Env,
    contract_id: BytesN<32>,
}

impl VaultContract {
    fn client(&self) -> VaultContractClient {
        VaultContractClient::new(&self.env, &self.contract_id)
    }

    pub fn new(env: &Env, contract_id: &[u8; 32]) -> Self {
        Self {
            env: env.clone(),
            contract_id: BytesN::from_array(env, contract_id),
        }
    }

    pub fn initialize(&self, admin: &Identifier, token_id: &[u8; 32]) {
        self.client()
            .initialize(admin, &BytesN::from_array(&self.env, token_id));
    }

    pub fn nonce(&self) -> BigInt {
        self.client().nonce()
    }

    pub fn deposit(&self, admin: AccountId, from: Identifier, amount: BigInt) {
        self.env.set_source_account(&admin);
        self.client().deposit(&from, &amount)
    }

    pub fn withdraw(&self, admin: AccountId, to: Identifier, shares: BigInt) {
        self.env.set_source_account(&admin);
        self.client().withdraw(
            &Auth {
                sig: Signature::Invoker,
                nonce: BigInt::zero(&self.env),
            },
            &to,
            &shares,
        )
    }

    pub fn get_shares(&self, id: &Identifier) -> BigInt {
        self.client().get_shares(id)
    }
}
