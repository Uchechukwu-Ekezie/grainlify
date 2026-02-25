#![no_std]
use soroban_sdk::{
    contract, contractimpl, contracttype, symbol_short, vec, Address, Env, String, Symbol, Vec,
};

mod nonce {
    use soroban_sdk::{contracttype, Address, Env};

    #[contracttype]
    pub enum NonceKey {
        Signer(Address),
    }

    pub fn get_nonce(env: &Env, signer: &Address) -> u64 {
        let key = NonceKey::Signer(signer.clone());
        env.storage().persistent().get(&key).unwrap_or(0)
    }

    pub fn validate_and_increment_nonce(env: &Env, signer: &Address, provided_nonce: u64) {
        let current_nonce = get_nonce(env, signer);
        
        if provided_nonce != current_nonce {
            panic!("Invalid nonce: expected {}, got {}", current_nonce, provided_nonce);
        }
        
        let key = NonceKey::Signer(signer.clone());
        env.storage().persistent().set(&key, &(current_nonce + 1));
    }
}

const PROGRAM_INITIALIZED: Symbol = symbol_short!("InitProg");
const FUNDS_LOCKED: Symbol = symbol_short!("Locked");
const PAYOUT: Symbol = symbol_short!("Payout");

const PROGRAM_DATA: Symbol = symbol_short!("ProgData");

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PayoutRecord {
    pub recipient: Address,
    pub amount: i128,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProgramData {
    pub program_id: String,
    pub total_funds: i128,
    pub remaining_balance: i128,
    pub authorized_payout_key: Address,
    pub payout_history: Vec<PayoutRecord>,
}

#[contract]
pub struct ProgramEscrowContract;

#[contractimpl]
impl ProgramEscrowContract {
    /// Initialize a new program escrow
    pub fn init_program(
        env: Env,
        program_id: String,
        authorized_payout_key: Address,
    ) -> ProgramData {
        if env.storage().instance().has(&PROGRAM_DATA) {
            panic!("Program already initialized");
        }

        let program_data = ProgramData {
            program_id: program_id.clone(),
            total_funds: 0,
            remaining_balance: 0,
            authorized_payout_key: authorized_payout_key.clone(),
            payout_history: vec![&env],
        };

        env.storage().instance().set(&PROGRAM_DATA, &program_data);

        env.events().publish(
            (PROGRAM_INITIALIZED,),
            (program_id, authorized_payout_key, 0i128),
        );

        program_data
    }

    /// Lock initial funds into the program escrow
    pub fn lock_program_funds(env: Env, amount: i128) -> ProgramData {
        if amount <= 0 {
            panic!("Amount must be greater than zero");
        }

        let mut program_data: ProgramData = env
            .storage()
            .instance()
            .get(&PROGRAM_DATA)
            .unwrap_or_else(|| panic!("Program not initialized"));

        program_data.total_funds += amount;
        program_data.remaining_balance += amount;

        env.storage().instance().set(&PROGRAM_DATA, &program_data);

        env.events().publish(
            (FUNDS_LOCKED,),
            (
                program_data.program_id.clone(),
                amount,
                program_data.remaining_balance,
            ),
        );

        program_data
    }

    /// Execute a single payout to one recipient with nonce-based replay protection
    /// 
    /// # Arguments
    /// * `recipient` - Address of the recipient
    /// * `amount` - Amount to transfer
    /// * `nonce` - Nonce for replay protection
    /// 
    /// # Returns
    /// Updated ProgramData after payout
    pub fn single_payout(env: Env, recipient: Address, amount: i128, nonce: u64) -> ProgramData {
        let program_data: ProgramData = env
            .storage()
            .instance()
            .get(&PROGRAM_DATA)
            .unwrap_or_else(|| panic!("Program not initialized"));

        // Require authorization from the authorized payout key
        program_data.authorized_payout_key.require_auth();
        
        // Validate and increment nonce to prevent replay
        nonce::validate_and_increment_nonce(&env, &program_data.authorized_payout_key, nonce);

        if amount <= 0 {
            panic!("Amount must be greater than zero");
        }

        if amount > program_data.remaining_balance {
            panic!("Insufficient balance: requested {}, available {}", 
                amount, program_data.remaining_balance);
        }

        // Record payout
        let timestamp = env.ledger().timestamp();
        let payout_record = PayoutRecord {
            recipient: recipient.clone(),
            amount,
            timestamp,
        };

        let mut updated_history = program_data.payout_history.clone();
        updated_history.push_back(payout_record);

        // Update program data
        let mut updated_data = program_data.clone();
        updated_data.remaining_balance -= amount;
        updated_data.payout_history = updated_history;

        env.storage().instance().set(&PROGRAM_DATA, &updated_data);

        env.events().publish(
            (PAYOUT,),
            (
                updated_data.program_id.clone(),
                recipient,
                amount,
                updated_data.remaining_balance,
            ),
        );

        updated_data
    }

    /// Get program information
    pub fn get_program_info(env: Env) -> ProgramData {
        env.storage()
            .instance()
            .get(&PROGRAM_DATA)
            .unwrap_or_else(|| panic!("Program not initialized"))
    }

    /// Get remaining balance
    pub fn get_remaining_balance(env: Env) -> i128 {
        let program_data: ProgramData = env
            .storage()
            .instance()
            .get(&PROGRAM_DATA)
            .unwrap_or_else(|| panic!("Program not initialized"));

        program_data.remaining_balance
    }
    
    /// Get current nonce for a signer (for replay protection)
    pub fn get_nonce(env: Env, signer: Address) -> u64 {
        nonce::get_nonce(&env, &signer)
    }
}

mod test;
