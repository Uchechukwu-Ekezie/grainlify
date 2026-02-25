#![no_std]
pub mod nonce;

use soroban_sdk::{contract, contractimpl, contracttype, Address, BytesN, Env};

#[contract]
pub struct GrainlifyContract;

#[contracttype]
#[derive(Clone)]
enum DataKey {
    Admin,
    Version,
}

const VERSION: u32 = 1;

#[contractimpl]
impl GrainlifyContract {
    pub fn init(env: Env, admin: Address) {
        if env.storage().instance().has(&DataKey::Admin) {
            panic!("Already initialized");
        }
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::Version, &VERSION);
    }

    pub fn upgrade(env: Env, new_wasm_hash: BytesN<32>) {
        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();

        env.deployer().update_current_contract_wasm(new_wasm_hash);
    }

    pub fn get_version(env: Env) -> u32 {
        env.storage().instance().get(&DataKey::Version).unwrap_or(0)
    }
    
    pub fn set_version(env: Env, new_version: u32) {
         let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
         admin.require_auth();
         env.storage().instance().set(&DataKey::Version, &new_version);
    }
}
