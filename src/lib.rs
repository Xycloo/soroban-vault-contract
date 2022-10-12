#![no_std]

#[cfg(feature = "testutils")]
extern crate std;

mod test;
pub mod testutils;

use soroban_auth::{Identifier, Signature};
use soroban_sdk::{contractimpl, contracttype, BigInt, BytesN, Env};

mod token {
    soroban_sdk::contractimport!(file = "./soroban_token_spec.wasm");
}

#[derive(Clone)]
#[contracttype]
pub enum DataKey {
    TokenId,
    Admin,
    TotSupply,
    Balance(Identifier),
    Nonce(Identifier),
}

#[derive(Clone)]
#[contracttype]
pub struct Auth {
    pub sig: Signature,
    pub nonce: BigInt,
}

fn get_contract_id(e: &Env) -> Identifier {
    Identifier::Contract(e.get_current_contract())
}

fn put_tot_supply(e: &Env, supply: BigInt) {
    let key = DataKey::TotSupply;
    e.data().set(key, supply);
}

fn get_tot_supply(e: &Env) -> BigInt {
    let key = DataKey::TotSupply;
    e.data().get(key).unwrap_or(Ok(BigInt::zero(&e))).unwrap()
}

fn put_id_balance(e: &Env, id: Identifier, amount: BigInt) {
    let key = DataKey::Balance(id);
    e.data().set(key, amount);
}

fn get_id_balance(e: &Env, id: Identifier) -> BigInt {
    let key = DataKey::Balance(id);
    e.data().get(key).unwrap_or(Ok(BigInt::zero(&e))).unwrap()
}

fn put_token_id(e: &Env, token_id: BytesN<32>) {
    let key = DataKey::TokenId;
    e.data().set(key, token_id);
}

fn get_token_id(e: &Env) -> BytesN<32> {
    let key = DataKey::TokenId;
    e.data().get(key).unwrap().unwrap()
}

fn get_token_balance(e: &Env) -> BigInt {
    let contract_id = get_token_id(e);
    token::Client::new(e, contract_id).balance(&get_contract_id(e))
}

fn transfer(e: &Env, to: Identifier, amount: BigInt) {
    let client = token::Client::new(e, get_token_id(e));
    client.xfer(
        &Signature::Invoker,
        &client.nonce(&Signature::Invoker.identifier(e)),
        &to,
        &amount,
    );
}

fn has_administrator(e: &Env) -> bool {
    let key = DataKey::Admin;
    e.data().has(key)
}

fn read_administrator(e: &Env) -> Identifier {
    let key = DataKey::Admin;
    e.data().get_unchecked(key).unwrap()
}

fn write_administrator(e: &Env, id: Identifier) {
    let key = DataKey::Admin;
    e.data().set(key, id);
}

pub fn check_admin(e: &Env, auth: &Signature) {
    let auth_id = auth.identifier(e);
    if auth_id != read_administrator(e) {
        panic!("not authorized by admin")
    }
}

fn read_nonce(e: &Env, id: &Identifier) -> BigInt {
    let key = DataKey::Nonce(id.clone());
    e.data()
        .get(key)
        .unwrap_or_else(|| Ok(BigInt::zero(e)))
        .unwrap()
}

fn verify_and_consume_nonce(e: &Env, auth: &Signature, expected_nonce: &BigInt) {
    match auth {
        Signature::Invoker => {
            if BigInt::zero(e) != expected_nonce {
                panic!("nonce should be zero for Invoker")
            }
            return;
        }
        _ => {}
    }

    let id = auth.identifier(e);
    let key = DataKey::Nonce(id.clone());
    let nonce = read_nonce(e, &id);

    if nonce != expected_nonce {
        panic!("incorrect nonce")
    }
    e.data().set(key, &nonce + 1);
}

fn mint_shares(e: &Env, to: Identifier, shares: BigInt) {
    let tot_supply = get_tot_supply(e);
    let id_balance = get_id_balance(e, to.clone());

    put_tot_supply(e, tot_supply + shares.clone());
    put_id_balance(e, to, id_balance + shares);
}

fn burn_shares(e: &Env, to: Identifier, shares: BigInt) {
    let tot_supply = get_tot_supply(e);
    let id_balance = get_id_balance(e, to.clone());

    assert!(shares < id_balance);

    put_tot_supply(e, tot_supply - shares.clone());
    put_id_balance(e, to, id_balance - shares);
}

pub trait VaultContractTrait {
    // Sets the admin and the vault's token id
    fn initialize(e: Env, admin: Identifier, token_id: BytesN<32>);

    // Returns the nonce for the admin
    fn nonce(e: Env) -> BigInt;

    // deposit shares into the vault: mints the vault shares to "from"
    fn deposit(e: Env, auth: Auth, from: Identifier, amount: BigInt);

    // withdraw an ammount of the vault's token id to "to" by burning shares
    fn withdraw(e: Env, auth: Auth, to: Identifier, shares: BigInt);

    // get vault shares for a user
    fn get_shares(e: Env, id: Identifier) -> BigInt;
}

pub struct VaultContract;

#[contractimpl]
impl VaultContractTrait for VaultContract {
    fn initialize(e: Env, admin: Identifier, token_id: BytesN<32>) {
        if has_administrator(&e) {
            panic!("admin is already set");
        }

        write_administrator(&e, admin);

        put_token_id(&e, token_id)
    }

    fn nonce(e: Env) -> BigInt {
        read_nonce(&e, &read_administrator(&e))
    }

    fn deposit(e: Env, admin_auth: Auth, from: Identifier, amount: BigInt) {
        check_admin(&e, &admin_auth.sig);
        verify_and_consume_nonce(&e, &admin_auth.sig, &admin_auth.nonce);

        let tot_supply = get_tot_supply(&e);

        let shares = if BigInt::zero(&e) == tot_supply {
            amount
        } else {
            (amount.clone() * tot_supply) / (get_token_balance(&e) - amount)
        };

        mint_shares(&e, from, shares);
    }

    fn get_shares(e: Env, id: Identifier) -> BigInt {
        e.data()
            .get(DataKey::Balance(id))
            .unwrap_or(Ok(BigInt::zero(&e)))
            .unwrap()
    }

    fn withdraw(e: Env, admin_auth: Auth, to: Identifier, shares: BigInt) {
        check_admin(&e, &admin_auth.sig);
        verify_and_consume_nonce(&e, &admin_auth.sig, &admin_auth.nonce);

        let tot_supply = get_tot_supply(&e);
        let amount = (shares.clone() * get_token_balance(&e)) / tot_supply;

        burn_shares(&e, to.clone(), shares);
        transfer(&e, to, amount);
    }
}
