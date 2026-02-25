#![cfg(test)]

use super::*;
use soroban_sdk::{testutils::Address as _, token, Address, Env};

fn setup_bounty(env: &Env) -> (BountyEscrowContractClient<'_>, Address, Address, Address) {
    env.mock_all_auths();

    let contract_id = env.register(BountyEscrowContract, ());
    let contract = BountyEscrowContractClient::new(env, &contract_id);

    let admin = Address::generate(env);
    let depositor = Address::generate(env);
    let token_admin = Address::generate(env);
    let token_contract = env.register_stellar_asset_contract_v2(token_admin);
    let token_address = token_contract.address();

    let token_admin_client = token::StellarAssetClient::new(env, &token_address);
    token_admin_client.mint(&depositor, &1_000_000_000_000i128);

    contract.init(&admin, &token_address);

    (contract, contract_id, admin, depositor)
}

#[test]
fn test_release_funds_nonce_increments() {
    let env = Env::default();
    let (contract, _contract_id, admin, depositor) = setup_bounty(&env);
    let contributor = Address::generate(&env);

    assert_eq!(contract.get_nonce(&admin), 0);

    contract.lock_funds(&depositor, &1, &100_000_000_000, &0);
    contract.release_funds(&1, &contributor, &0);
    assert_eq!(contract.get_nonce(&admin), 1);
}

#[test]
#[should_panic(expected = "Error(Contract, #8)")]
fn test_release_funds_replay_rejected() {
    let env = Env::default();
    let (contract, _contract_id, admin, depositor) = setup_bounty(&env);
    let contributor = Address::generate(&env);

    contract.lock_funds(&depositor, &7, &100_000_000_000, &0);
    contract.release_funds(&7, &contributor, &0);
    assert_eq!(contract.get_nonce(&admin), 1);

    // Reusing nonce=0 must fail even before escrow-status checks.
    contract.release_funds(&7, &contributor, &0);
}

#[test]
#[should_panic(expected = "Error(Contract, #8)")]
fn test_release_funds_out_of_order_nonce_rejected() {
    let env = Env::default();
    let (contract, _contract_id, _admin, depositor) = setup_bounty(&env);
    let contributor = Address::generate(&env);

    contract.lock_funds(&depositor, &9, &100_000_000_000, &0);
    contract.release_funds(&9, &contributor, &5);
}
