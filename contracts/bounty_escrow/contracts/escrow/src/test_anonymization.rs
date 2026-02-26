//! Tests for optional anonymization and pseudonymization (Issue #680).

use crate::{
    AnonymousParty, BountyEscrowContract, BountyEscrowContractClient, Error as ContractError,
    EscrowInfo,
};
use soroban_sdk::{
    testutils::{Address as _, Ledger, MockAuth, MockAuthInvoke},
    token, Address, BytesN, Env, IntoVal,
};

fn create_test_env() -> (Env, BountyEscrowContractClient<'static>, Address) {
    let env = Env::default();
    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);
    (env, client, contract_id)
}

fn create_token_contract<'a>(
    e: &'a Env,
    admin: &Address,
) -> (Address, token::Client<'a>, token::StellarAssetClient<'a>) {
    let token_id = e.register_stellar_asset_contract_v2(admin.clone());
    let token = token_id.address();
    let token_client = token::Client::new(e, &token);
    let token_admin_client = token::StellarAssetClient::new(e, &token);
    (token, token_client, token_admin_client)
}

fn commitment_from_bytes(env: &Env, bytes: &[u8; 32]) -> BytesN<32> {
    BytesN::from_array(env, bytes)
}

#[test]
fn test_lock_funds_anonymous_and_get_escrow_info_v2() {
    let (env, client, _) = create_test_env();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let (token, _token_client, token_admin_client) = create_token_contract(&env, &token_admin);

    client.init(&admin, &token);
    token_admin_client.mint(&depositor, &5_000);

    let bounty_id = 100u64;
    let amount = 1_000i128;
    let deadline = env.ledger().timestamp() + 3600;
    let commitment = commitment_from_bytes(&env, &[0xab; 32]);

    client.lock_funds_anonymous(&depositor, &commitment, &bounty_id, &amount, &deadline);

    let info: EscrowInfo = client.get_escrow_info_v2(&bounty_id);
    assert_eq!(info.amount, amount);
    assert_eq!(info.remaining_amount, amount);
    assert_eq!(info.deadline, deadline);
    match &info.depositor {
        AnonymousParty::Address(_) => panic!("expected commitment"),
        AnonymousParty::Commitment(c) => assert_eq!(*c, commitment),
    }
}

#[test]
fn test_get_escrow_info_rejects_anonymous() {
    let (env, client, _) = create_test_env();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let (token, _token_client, token_admin_client) = create_token_contract(&env, &token_admin);

    client.init(&admin, &token);
    token_admin_client.mint(&depositor, &5_000);

    let bounty_id = 101u64;
    let commitment = commitment_from_bytes(&env, &[0xcd; 32]);
    client.lock_funds_anonymous(
        &depositor,
        &commitment,
        &bounty_id,
        &2_000,
        &(env.ledger().timestamp() + 3600),
    );

    let res = client.try_get_escrow_info(&bounty_id);
    assert!(res.is_err());
    assert_eq!(
        res.unwrap_err().unwrap(),
        ContractError::UseGetEscrowInfoV2ForAnonymous
    );
}

#[test]
fn test_refund_requires_resolution_for_anonymous() {
    let (env, client, _) = create_test_env();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let (token, _token_client, token_admin_client) = create_token_contract(&env, &token_admin);

    client.init(&admin, &token);
    token_admin_client.mint(&depositor, &5_000);

    let bounty_id = 102u64;
    let amount = 1_500i128;
    let commitment = commitment_from_bytes(&env, &[0xef; 32]);
    client.lock_funds_anonymous(
        &depositor,
        &commitment,
        &bounty_id,
        &amount,
        &(env.ledger().timestamp() + 3600),
    );

    let res = client.try_refund(&bounty_id);
    assert!(res.is_err());
    assert_eq!(
        res.unwrap_err().unwrap(),
        ContractError::AnonymousRefundRequiresResolution
    );
}

#[test]
fn test_refund_resolved_success() {
    let (env, client, contract_id) = create_test_env();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let resolver = Address::generate(&env);
    let resolved_recipient = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let (token, token_client, token_admin_client) = create_token_contract(&env, &token_admin);

    client.init(&admin, &token);
    client.set_anonymous_resolver(&Some(resolver));
    token_admin_client.mint(&depositor, &5_000);

    let bounty_id = 103u64;
    let amount = 2_000i128;
    let commitment = commitment_from_bytes(&env, &[0x11; 32]);
    env.ledger().set_timestamp(1000);
    let deadline = 500; // already in the past so refund is allowed
    client.lock_funds_anonymous(&depositor, &commitment, &bounty_id, &amount, &deadline);

    env.ledger().set_timestamp(2000);
    client.refund_resolved(&bounty_id, &resolved_recipient);

    assert_eq!(token_client.balance(&resolved_recipient), amount);
    assert_eq!(token_client.balance(&contract_id), 0);

    let info: EscrowInfo = client.get_escrow_info_v2(&bounty_id);
    assert_eq!(info.remaining_amount, 0);
}

#[test]
fn test_refund_resolved_fails_without_resolver_set() {
    let (env, client, _) = create_test_env();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let _resolver = Address::generate(&env);
    let recipient = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let (token, _token_client, token_admin_client) = create_token_contract(&env, &token_admin);

    client.init(&admin, &token);
    // do not set anonymous resolver
    token_admin_client.mint(&depositor, &5_000);

    let bounty_id = 104u64;
    let commitment = commitment_from_bytes(&env, &[0x22; 32]);
    env.ledger().set_timestamp(1000);
    client.lock_funds_anonymous(&depositor, &commitment, &bounty_id, &2_000, &500);

    let res = client.try_refund_resolved(&bounty_id, &recipient);
    assert!(res.is_err());
    assert_eq!(
        res.unwrap_err().unwrap(),
        ContractError::AnonymousResolverNotSet
    );
}

#[test]
fn test_refund_resolved_fails_when_not_resolver() {
    let (env, client, _) = create_test_env();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let resolver = Address::generate(&env);
    let wrong_caller = Address::generate(&env);
    let recipient = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let (token, _token_client, token_admin_client) = create_token_contract(&env, &token_admin);

    client.init(&admin, &token);
    client.set_anonymous_resolver(&Some(resolver));
    token_admin_client.mint(&depositor, &5_000);

    let bounty_id = 105u64;
    let commitment = commitment_from_bytes(&env, &[0x33; 32]);
    env.ledger().set_timestamp(1000);
    client.lock_funds_anonymous(&depositor, &commitment, &bounty_id, &2_000, &500);

    env.ledger().set_timestamp(2000);
    env.mock_auths(&[MockAuth {
        address: &wrong_caller,
        invoke: &MockAuthInvoke {
            contract: &client.address,
            fn_name: "refund_resolved",
            args: (bounty_id, recipient.clone()).into_val(&env),
            sub_invokes: &[],
        },
    }]);
    let res = client.try_refund_resolved(&bounty_id, &recipient);
    assert!(res.is_err());
    // Wrong caller: resolver.require_auth() fails (auth error, not NotAnonymousResolver)
}

#[test]
fn test_release_funds_works_for_anonymous_escrow() {
    let (env, client, contract_id) = create_test_env();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let contributor = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let (token, token_client, token_admin_client) = create_token_contract(&env, &token_admin);

    client.init(&admin, &token);
    token_admin_client.mint(&depositor, &5_000);

    let bounty_id = 106u64;
    let amount = 3_000i128;
    let commitment = commitment_from_bytes(&env, &[0x44; 32]);
    client.lock_funds_anonymous(
        &depositor,
        &commitment,
        &bounty_id,
        &amount,
        &(env.ledger().timestamp() + 3600),
    );

    client.release_funds(&bounty_id, &contributor);

    assert_eq!(token_client.balance(&contributor), amount);
    assert_eq!(token_client.balance(&contract_id), 0);

    let info: EscrowInfo = client.get_escrow_info_v2(&bounty_id);
    assert_eq!(info.remaining_amount, 0);
}

#[test]
fn test_claim_ticket_works_for_anonymous_escrow() {
    let (env, client, contract_id) = create_test_env();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let beneficiary = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let (token, token_client, token_admin_client) = create_token_contract(&env, &token_admin);

    client.init(&admin, &token);
    token_admin_client.mint(&depositor, &5_000);

    let bounty_id = 107u64;
    let amount = 1_000i128;
    let commitment = commitment_from_bytes(&env, &[0x55; 32]);
    client.lock_funds_anonymous(
        &depositor,
        &commitment,
        &bounty_id,
        &amount,
        &(env.ledger().timestamp() + 3600),
    );

    let expires_at = env.ledger().timestamp() + 86400;
    let ticket_id = client.issue_claim_ticket(&bounty_id, &beneficiary, &amount, &expires_at);
    client.claim_with_ticket(&ticket_id);

    assert_eq!(token_client.balance(&beneficiary), amount);
    assert_eq!(token_client.balance(&contract_id), 0);
}
