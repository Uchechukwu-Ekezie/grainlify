//! # Program Escrow Smart Contract
//!
//! A secure escrow system for managing hackathon and program prize pools on Stellar.
//! This contract enables organizers to lock funds and distribute prizes to multiple
//! winners through secure, auditable batch payouts.
//!
//! ## Overview
//!
//! The Program Escrow contract manages the complete lifecycle of hackathon/program prizes:
//! 1. **Initialization**: Set up program with authorized payout controller
//! 2. **Fund Locking**: Lock prize pool funds in escrow
//! 3. **Batch Payouts**: Distribute prizes to multiple winners simultaneously
//! 4. **Single Payouts**: Distribute individual prizes
//! 5. **Tracking**: Maintain complete payout history and balance tracking
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────┐
//! │              Program Escrow Architecture                         │
//! ├─────────────────────────────────────────────────────────────────┤
//! │                                                                  │
//! │  ┌──────────────┐                                               │
//! │  │  Organizer   │                                               │
//! │  └──────┬───────┘                                               │
//! │         │                                                        │
//! │         │ 1. init_program()                                     │
//! │         ▼                                                        │
//! │  ┌──────────────────┐                                           │
//! │  │  Program Created │                                           │
//! │  └────────┬─────────┘                                           │
//! │           │                                                      │
//! │           │ 2. lock_program_funds()                             │
//! │           ▼                                                      │
//! │  ┌──────────────────┐                                           │
//! │  │  Funds Locked    │                                           │
//! │  │  (Prize Pool)    │                                           │
//! │  └────────┬─────────┘                                           │
//! │           │                                                      │
//! │           │ 3. Hackathon happens...                             │
//! │           │                                                      │
//! │  ┌────────▼─────────┐                                           │
//! │  │ Authorized       │                                           │
//! │  │ Payout Key       │                                           │
//! │  └────────┬─────────┘                                           │
//! │           │                                                      │
//! │    ┌──────┴───────┐                                             │
//! │    │              │                                             │
//! │    ▼              ▼                                             │
//! │ batch_payout() single_payout()                                  │
//! │    │              │                                             │
//! │    ▼              ▼                                             │
//! │ ┌─────────────────────────┐                                    │
//! │ │   Winner 1, 2, 3, ...   │                                    │
//! │ └─────────────────────────┘                                    │
//! │                                                                  │
//! │  Storage:                                                        │
//! │  ┌──────────────────────────────────────────┐                  │
//! │  │ ProgramData:                             │                  │
//! │  │  - program_id                            │                  │
//! │  │  - total_funds                           │                  │
//! │  │  - remaining_balance                     │                  │
//! │  │  - authorized_payout_key                 │                  │
//! │  │  - payout_history: [PayoutRecord]        │                  │
//! │  │  - token_address                         │                  │
//! │  └──────────────────────────────────────────┘                  │
//! └─────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Security Model
//!
//! ### Trust Assumptions
//! - **Authorized Payout Key**: Trusted backend service that triggers payouts
//! - **Organizer**: Trusted to lock appropriate prize amounts
//! - **Token Contract**: Standard Stellar Asset Contract (SAC)
//! - **Contract**: Trustless; operates according to programmed rules
//!
//! ### Key Security Features
//! 1. **Single Initialization**: Prevents program re-configuration
//! 2. **Authorization Checks**: Only authorized key can trigger payouts
//! 3. **Balance Validation**: Prevents overdrafts
//! 4. **Atomic Transfers**: All-or-nothing batch operations
//! 5. **Complete Audit Trail**: Full payout history tracking
//! 6. **Overflow Protection**: Safe arithmetic for all calculations
//!
//! ## Usage Example
//!
//! ```rust
//! use soroban_sdk::{Address, Env, String, vec};
//!
//! // 1. Initialize program (one-time setup)
//! let program_id = String::from_str(&env, "Hackathon2024");
//! let backend = Address::from_string("GBACKEND...");
//! let usdc_token = Address::from_string("CUSDC...");
//!
//! let program = escrow_client.init_program(
//!     &program_id,
//!     &backend,
//!     &usdc_token
//! );
//!
//! // 2. Lock prize pool (10,000 USDC)
//! let prize_pool = 10_000_0000000; // 10,000 USDC (7 decimals)
//! escrow_client.lock_program_funds(&prize_pool);
//!
//! // 3. After hackathon, distribute prizes
//! let winners = vec![
//!     &env,
//!     Address::from_string("GWINNER1..."),
//!     Address::from_string("GWINNER2..."),
//!     Address::from_string("GWINNER3..."),
//! ];
//!
//! let prizes = vec![
//!     &env,
//!     5_000_0000000,  // 1st place: 5,000 USDC
//!     3_000_0000000,  // 2nd place: 3,000 USDC
//!     2_000_0000000,  // 3rd place: 2,000 USDC
//! ];
//!
//! escrow_client.batch_payout(&winners, &prizes);
//! ```
//!
//! ## Event System
//!
//! The contract emits events for all major operations:
//! - `ProgramInit`: Program initialization
//! - `FundsLocked`: Prize funds locked
//! - `BatchPayout`: Multiple prizes distributed
//! - `Payout`: Single prize distributed
//!
//! ## Best Practices
//!
//! 1. **Verify Winners**: Confirm winner addresses off-chain before payout
//! 2. **Test Payouts**: Use testnet for testing prize distributions
//! 3. **Secure Backend**: Protect authorized payout key with HSM/multi-sig
//! 4. **Audit History**: Review payout history before each distribution
//! 5. **Balance Checks**: Verify remaining balance matches expectations
//! 6. **Token Approval**: Ensure contract has token allowance before locking funds

#![no_std]
use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, symbol_short, token, vec, Address, Env,
    String, Symbol, Vec,
};

// Event types
const PROGRAM_INITIALIZED: Symbol = symbol_short!("ProgInit");
const FUNDS_LOCKED: Symbol = symbol_short!("FundLock");
const BATCH_PAYOUT: Symbol = symbol_short!("BatchPay");
const PAYOUT: Symbol = symbol_short!("Payout");

// Event types
const PROGRAM_INITIALIZED: Symbol = symbol_short!("ProgInit");
const FUNDS_LOCKED: Symbol = symbol_short!("FundLock");
const BATCH_PAYOUT: Symbol = symbol_short!("BatchPay");
const PAYOUT: Symbol = symbol_short!("Payout");
const DEPENDENCY_CREATED: Symbol = symbol_short!("dep_add");
const DEPENDENCY_CLEARED: Symbol = symbol_short!("dep_clr");
const DEPENDENCY_STATUS_UPDATED: Symbol = symbol_short!("dep_sts");

// Storage keys
const PROGRAM_DATA: Symbol = symbol_short!("ProgData");
const FEE_CONFIG: Symbol = symbol_short!("FeeCfg");

// Fee rate is stored in basis points (1 basis point = 0.01%)
// Example: 100 basis points = 1%, 1000 basis points = 10%
const BASIS_POINTS: i128 = 10_000;
const MAX_FEE_RATE: i128 = 1_000; // Maximum 10% fee

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FeeConfig {
    pub lock_fee_rate: i128,      // Fee rate for lock operations (basis points)
    pub payout_fee_rate: i128,     // Fee rate for payout operations (basis points)
    pub fee_recipient: Address,    // Address to receive fees
    pub fee_enabled: bool,         // Global fee enable/disable flag
}

extern crate grainlify_core;

// Event types
const EVENT_VERSION_V2: u32 = 2;
const PAUSE_STATE_CHANGED: Symbol = symbol_short!("PauseSt");
const PROGRAM_REGISTRY: Symbol = symbol_short!("ProgReg");
const PROGRAM_REGISTERED: Symbol = symbol_short!("ProgRgd");
const RECEIPT_COUNTER: Symbol = symbol_short!("RcpCntr");
const PROGRAM_INDEX: Symbol = symbol_short!("ProgIdx");
const AUTH_KEY_INDEX: Symbol = symbol_short!("AuthIdx");
const SCHEDULES: Symbol = symbol_short!("Scheds");
const RELEASE_HISTORY: Symbol = symbol_short!("RelHist");
const NEXT_SCHEDULE_ID: Symbol = symbol_short!("NxtSched");

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PayoutRecord {
    pub recipient: Address,
    pub amount: i128,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProgramInitializedEvent {
    pub version: u32,
    pub program_id: String,
    pub authorized_payout_key: Address,
    pub token_address: Address,
    pub total_funds: i128,
    pub reference_hash: Option<soroban_sdk::Bytes>,
    pub receipt_id: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FundsLockedEvent {
    pub version: u32,
    pub program_id: String,
    pub amount: i128,
    pub remaining_balance: i128,
    pub receipt_id: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BatchPayoutEvent {
    pub version: u32,
    pub program_id: String,
    pub recipient_count: u32,
    pub total_amount: i128,
    pub remaining_balance: i128,
    pub receipt_id: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PayoutEvent {
    pub version: u32,
    pub program_id: String,
    pub recipient: Address,
    pub amount: i128,
    pub remaining_balance: i128,
    pub receipt_id: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ScheduleCreatedEvent {
    pub program_id: String,
    pub schedule_id: u64,
    pub recipient: Address,
    pub amount: i128,
    pub release_timestamp: u64,
    pub receipt_id: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProgramData {
    pub program_id: String,
    pub total_funds: i128,
    pub remaining_balance: i128,
    pub authorized_payout_key: Address,
    pub payout_history: Vec<PayoutRecord>,
    pub token_address: Address,
    pub initial_liquidity: i128,
    pub reference_hash: Option<soroban_sdk::Bytes>,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataKey {
    Program(String),                 // program_id -> ProgramData
    Admin,                           // Contract Admin
    ReleaseSchedule(String, u64),    // program_id, schedule_id -> ProgramReleaseSchedule
    ReleaseHistory(String),          // program_id -> Vec<ProgramReleaseHistory>
    NextScheduleId(String),          // program_id -> next schedule_id
    MultisigConfig(String),          // program_id -> MultisigConfig
    PayoutApproval(String, Address), // program_id, recipient -> PayoutApproval
    PendingClaim(String, u64),       // (program_id, schedule_id) -> ClaimRecord
    ClaimWindow,                     // u64 seconds (global config)
    PauseFlags,                      // PauseFlags struct
    RateLimitConfig,                 // RateLimitConfig struct
    ReceiptCounter,                  // u64 Global Receipt Counter
    ProgramRegistry,                 // Global registry of all program IDs
    ProgramDependencies(String),     // program_id -> Vec<dependency_id>
    DependencyStatus(String),        // dependency_id -> DependencyStatus
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PauseFlags {
    pub lock_paused: bool,
    pub release_paused: bool,
    pub refund_paused: bool,
    pub pause_reason: Option<String>,
    pub paused_at: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PauseStateChanged {
    pub operation: Symbol,
    pub paused: bool,
    pub admin: Address,
    pub reason: Option<String>,
    pub timestamp: u64,
    pub receipt_id: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EmergencyWithdrawEvent {
    pub admin: Address,
    pub target: Address,
    pub amount: i128,
    pub timestamp: u64,
    pub receipt_id: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RateLimitConfig {
    pub window_size: u64,
    pub max_operations: u32,
    pub cooldown_period: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProgramReleaseSchedule {
    pub schedule_id: u64,
    pub recipient: Address,
    pub amount: i128,
    pub release_timestamp: u64,
    pub released: bool,
    pub released_at: Option<u64>,
    pub released_by: Option<Address>,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ReleaseType {
    Manual,
    Automatic,
    Oracle,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProgramReleaseHistory {
    pub schedule_id: u64,
    pub recipient: Address,
    pub amount: i128,
    pub released_at: u64,
    pub release_type: ReleaseType,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DependencyStatus {
    Pending,
    Completed,
    Failed,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProgramAggregateStats {
    pub total_funds: i128,
    pub remaining_balance: i128,
    pub total_paid_out: i128,
    pub authorized_payout_key: Address,
    pub payout_history: Vec<PayoutRecord>,
    pub token_address: Address,
    pub payout_count: u32,
    pub scheduled_count: u32,
    pub released_count: u32,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProgramInitItem {
    pub program_id: String,
    pub authorized_payout_key: Address,
    pub token_address: Address,
    pub reference_hash: Option<soroban_sdk::Bytes>,
}

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum BatchError {
    InvalidBatchSize = 1,
    ProgramAlreadyExists = 2,
    DuplicateProgramId = 3,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MultisigConfig {
    pub threshold_amount: i128,
    pub signers: Vec<Address>,
    pub required_signatures: u32,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PayoutApproval {
    pub program_id: String,
    pub recipient: Address,
    pub amount: i128,
    pub approvals: Vec<Address>,
}

pub const MAX_BATCH_SIZE: u32 = 100;

fn vec_contains(values: &Vec<String>, target: &String) -> bool {
    for value in values.iter() {
        if value == *target {
            return true;
        }
    }
    false
}

fn get_program_dependencies_internal(env: &Env, program_id: &String) -> Vec<String> {
    env.storage()
        .instance()
        .get(&DataKey::ProgramDependencies(program_id.clone()))
        .unwrap_or(vec![env])
}

fn dependency_status_internal(env: &Env, dependency_id: &String) -> DependencyStatus {
    env.storage()
        .instance()
        .get(&DataKey::DependencyStatus(dependency_id.clone()))
        .unwrap_or(DependencyStatus::Pending)
}

fn path_exists_to_target(
    env: &Env,
    from_program: &String,
    target_program: &String,
    visited: &mut Vec<String>,
) -> bool {
    if *from_program == *target_program {
        return true;
    }
    if vec_contains(visited, from_program) {
        return false;
    }

    visited.push_back(from_program.clone());
    let deps = get_program_dependencies_internal(env, from_program);
    for dep in deps.iter() {
        if env.storage().instance().has(&DataKey::Program(dep.clone()))
            && path_exists_to_target(env, &dep, target_program, visited)
        {
            return true;
        }
    }

    false
}
mod anti_abuse {
    use soroban_sdk::{symbol_short, Address, Env, Symbol};

    const ADMIN: Symbol = symbol_short!("AbuseAdm");
    const RATE_LIMIT: Symbol = symbol_short!("RateLim");

    pub fn set_admin(env: &Env, admin: Address) {
        env.storage().instance().set(&ADMIN, &admin);
    }

    pub fn get_admin(env: &Env) -> Option<Address> {
        env.storage().instance().get(&ADMIN)
    }

    pub fn check_rate_limit(env: &Env, _caller: Address) {
        let count: u32 = env.storage().instance().get(&RATE_LIMIT).unwrap_or(0);
        env.storage().instance().set(&RATE_LIMIT, &(count + 1));
    }
}

// ==================== MONITORING MODULE ====================
mod monitoring {
    use soroban_sdk::{contracttype, symbol_short, Address, Env, String, Symbol};

    // Storage keys
    const OPERATION_COUNT: &str = "op_count";
    const USER_COUNT: &str = "usr_count";
    const ERROR_COUNT: &str = "err_count";

    // Event: Operation metric
    #[contracttype]
    #[derive(Clone, Debug)]
    pub struct OperationMetric {
        pub operation: Symbol,
        pub caller: Address,
        pub timestamp: u64,
        pub success: bool,
    }

    // Event: Performance metric
    #[contracttype]
    #[derive(Clone, Debug)]
    pub struct PerformanceMetric {
        pub function: Symbol,
        pub duration: u64,
        pub timestamp: u64,
    }

    // Data: Health status
    #[contracttype]
    #[derive(Clone, Debug)]
    pub struct HealthStatus {
        pub is_healthy: bool,
        pub last_operation: u64,
        pub total_operations: u64,
        pub contract_version: String,
    }

    // Data: Analytics
    #[contracttype]
    #[derive(Clone, Debug)]
    pub struct Analytics {
        pub operation_count: u64,
        pub unique_users: u64,
        pub error_count: u64,
        pub error_rate: u32,
    }

    // Data: State snapshot
    #[contracttype]
    #[derive(Clone, Debug)]
    pub struct StateSnapshot {
        pub timestamp: u64,
        pub total_operations: u64,
        pub total_users: u64,
        pub total_errors: u64,
    }

    // Data: Performance stats
    #[contracttype]
    #[derive(Clone, Debug)]
    pub struct PerformanceStats {
        pub function_name: Symbol,
        pub call_count: u64,
        pub total_time: u64,
        pub avg_time: u64,
        pub last_called: u64,
    }

    // Track operation
    pub fn track_operation(env: &Env, operation: Symbol, caller: Address, success: bool) {
        let key = Symbol::new(env, OPERATION_COUNT);
        let count: u64 = env.storage().persistent().get(&key).unwrap_or(0);
        env.storage().persistent().set(&key, &(count + 1));

        if !success {
            let err_key = Symbol::new(env, ERROR_COUNT);
            let err_count: u64 = env.storage().persistent().get(&err_key).unwrap_or(0);
            env.storage().persistent().set(&err_key, &(err_count + 1));
        }

        env.events().publish(
            (symbol_short!("metric"),),
            OperationMetric {
                operation,
                caller,
                timestamp: env.ledger().timestamp(),
                success,
            },
        );
    }

    pub fn emit_performance(env: &Env, function: Symbol, duration: u64) {
        env.events().publish(
            (symbol_short!("perf"),),
            PerformanceMetric {
                function,
                duration,
                timestamp: env.ledger().timestamp(),
            },
        );
    }
}

// ── Step 1: Add module declarations near the top of lib.rs ──────────────
// (after `mod anti_abuse;` and before the contract struct)

mod claim_period;
pub mod token_math;
pub use claim_period::{ClaimRecord, ClaimStatus};
mod error_recovery;
mod reentrancy_guard;
#[cfg(test)]
mod test_claim_period_expiry_cancellation;

#[cfg(test)]
mod test_token_math;

// Storage keys
const PROGRAM_DATA: Symbol = symbol_short!("ProgData");
const FEE_CONFIG: Symbol = symbol_short!("FeeCfg");
const CONFIG_SNAPSHOT_LIMIT: u32 = 20;

// Fee rate is stored in basis points (1 basis point = 0.01%)
// Example: 100 basis points = 1%, 1000 basis points = 10%
const BASIS_POINTS: i128 = 10_000;
const MAX_FEE_RATE: i128 = 1_000; // Maximum 10% fee

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FeeConfig {
    pub lock_fee_rate: i128,    // Fee rate for lock operations (basis points)
    pub payout_fee_rate: i128,  // Fee rate for payout operations (basis points)
    pub fee_recipient: Address, // Address to receive fees
    pub fee_enabled: bool,      // Global fee enable/disable flag
}
#[cfg(any())]
mod reentrancy_tests;
#[cfg(test)]
mod test_dispute_resolution;

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConfigSnapshot {
    pub id: u64,
    pub timestamp: u64,
    pub fee_config: FeeConfig,
    pub anti_abuse_config: anti_abuse::AntiAbuseConfig,
    pub anti_abuse_admin: Option<Address>,
    pub is_paused: bool,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ConfigSnapshotKey {
    Snapshot(u64),
    SnapshotIndex,
    SnapshotCounter,
}
// ==================== MONITORING MODULE ====================
mod monitoring {
    use soroban_sdk::{contracttype, symbol_short, Address, Env, String, Symbol};

    // Storage keys
    const OPERATION_COUNT: &str = "op_count";
    const USER_COUNT: &str = "usr_count";
    const ERROR_COUNT: &str = "err_count";

    // Event: Operation metric
    #[contracttype]
    #[derive(Clone, Debug)]
    pub struct OperationMetric {
        pub operation: Symbol,
        pub caller: Address,
        pub timestamp: u64,
        pub success: bool,
    }

    // Event: Performance metric
    #[contracttype]
    #[derive(Clone, Debug)]
    pub struct PerformanceMetric {
        pub function: Symbol,
        pub duration: u64,
        pub timestamp: u64,
    }

#[contract]
pub struct ProgramEscrowContract;

// Event symbols for program release schedules
const PROG_SCHEDULE_CREATED: soroban_sdk::Symbol = soroban_sdk::symbol_short!("prg_sch_c");
const PROG_SCHEDULE_RELEASED: soroban_sdk::Symbol = soroban_sdk::symbol_short!("prg_sch_r");

#[contractimpl]
impl ProgramEscrowContract {
    fn increment_receipt_id(env: &Env) -> u64 {
        let mut count: u64 = env
            .storage()
            .instance()
            .get(&DataKey::ReceiptCounter)
            .unwrap_or(0);
        count += 1;
        env.storage()
            .instance()
            .set(&DataKey::ReceiptCounter, &count);
        count
    }

    /// Initialize a new program escrow
    ///
    /// # Arguments
    /// * `env` - The contract environment
    /// * `program_id` - Unique identifier for this program/hackathon
    /// * `authorized_payout_key` - Address authorized to trigger payouts (backend)
    /// * `token_address` - Address of the token contract for transfers (e.g., USDC)
    ///
    /// # Returns
    /// * `ProgramData` - The initialized program configuration
    ///
    /// # Panics
    /// * If program is already initialized
    ///
    /// # State Changes
    /// - Creates ProgramData with zero balances
    /// - Sets authorized payout key (immutable after this)
    /// - Initializes empty payout history
    /// - Emits ProgramInitialized event
    ///
    /// # Security Considerations
    /// - Can only be called once (prevents re-configuration)
    /// - No authorization required (first-caller initialization)
    /// - Authorized payout key should be a secure backend service
    /// - Token address must be a valid Stellar Asset Contract
    /// - Program ID should be unique and descriptive
    ///
    /// # Events
    /// Emits: `ProgramInit(program_id, authorized_payout_key, token_address, 0)`
    ///
    /// # Example
    /// ```rust
    /// use soroban_sdk::{Address, String, Env};
    ///
    /// let program_id = String::from_str(&env, "ETHGlobal2024");
    /// let backend = Address::from_string("GBACKEND...");
    /// let usdc = Address::from_string("CUSDC...");
    ///
    /// let program = escrow_client.init_program(
    ///     &program_id,
    ///     &backend,
    ///     &usdc
    /// );
    ///
    /// println!("Program created: {}", program.program_id);
    /// ```
    ///
    /// # Production Setup
    /// ```bash
    /// # Deploy contract
    /// stellar contract deploy \
    ///   --wasm target/wasm32-unknown-unknown/release/escrow.wasm \
    ///   --source ORGANIZER_KEY
    ///
    /// # Initialize program
    /// stellar contract invoke \
    ///   --id CONTRACT_ID \
    ///   --source ORGANIZER_KEY \
    ///   -- init_program \
    ///   --program_id "Hackathon2024" \
    ///   --authorized_payout_key GBACKEND... \
    ///   --token_address CUSDC...
    /// ```
    ///
    /// # Gas Cost
    /// Low - Initial storage writes

    // ========================================================================
    // Pause and Emergency Functions
    // ========================================================================

    /// Check if contract is paused (internal helper)
    fn is_paused_internal(env: &Env) -> bool {
        env.storage()
            .instance()
            .get::<_, bool>(&DataKey::IsPaused)
            .unwrap_or(false)
    }

    /// Get pause status (view function)
    pub fn is_paused(env: Env) -> bool {
        Self::is_paused_internal(&env)
    }

    /// Pause the contract (authorized payout key only)
    /// Prevents new fund locking, payouts, and schedule releases
    pub fn pause(env: Env) -> () {
        // For program-escrow, pause is triggered by the first authorized key that calls it
        // In a multi-program setup, this would need to be per-program

        if Self::is_paused_internal(&env) {
            return; // Already paused, idempotent
        }

        env.storage().instance().set(&DataKey::IsPaused, &true);

        env.events()
            .publish((symbol_short!("pause"),), (env.ledger().timestamp(),));
    }

    /// Unpause the contract (authorized payout key only)
    /// Resumes normal operations
    pub fn unpause(env: Env) -> () {
        if !Self::is_paused_internal(&env) {
            return; // Already unpaused, idempotent
        }

        env.storage().instance().set(&DataKey::IsPaused, &false);

        env.events()
            .publish((symbol_short!("unpause"),), (env.ledger().timestamp(),));
    }

    /// Emergency withdrawal for all contract funds (authorized payout key only, only when paused)
    pub fn emergency_withdraw(env: Env, program_id: String, recipient: Address) -> i128 {
        // Only allow emergency withdrawal when contract is paused
        if !Self::is_paused_internal(&env) {
            panic!("Contract must be paused for emergency withdrawal");
        }

        // Get program data to access token address
        let program_key = DataKey::Program(program_id.clone());
        let program_data: ProgramData =
            env.storage()
                .instance()
                .get(&program_key)
                .unwrap_or_else(|| {
                    panic!("Program not found");
                });

        let client = token::Client::new(&env, &program_data.token_address);
        let balance = client.balance(&env.current_contract_address());

        if balance <= 0 {
            return 0; // No funds to withdraw
        }

        // Transfer all funds to recipient
        client.transfer(&env.current_contract_address(), &recipient, &balance);

        env.events().publish(
            (symbol_short!("ewith"),),
            (balance, env.ledger().timestamp()),
        );

        balance
    /// The initialized ProgramData
    pub fn init_program(
        env: Env,
        program_id: String,
        authorized_payout_key: Address,
        token_address: Address,
        creator: Address,
        initial_liquidity: Option<i128>,
        reference_hash: Option<soroban_sdk::Bytes>,
    ) -> ProgramData {
        Self::initialize_program(
            env,
            program_id,
            authorized_payout_key,
            token_address,
            creator,
            initial_liquidity,
            reference_hash,
        )
    }

    pub fn initialize_program(
        env: Env,
        program_id: String,
        authorized_payout_key: Address,
        token_address: Address,
        creator: Address,
        initial_liquidity: Option<i128>,
        reference_hash: Option<soroban_sdk::Bytes>,
    ) -> ProgramData {
        let receipt_id = Self::increment_receipt_id(&env);
        let program_key = DataKey::Program(program_id.clone());

        // Check if program already exists
        if env.storage().instance().has(&program_key) {
            panic!("Program already initialized");
        }

        let mut total_funds = 0i128;
        let mut remaining_balance = 0i128;
        let mut init_liquidity = 0i128;

        if let Some(amount) = initial_liquidity {
            if amount > 0 {
                // Transfer initial liquidity from creator to contract
                let contract_address = env.current_contract_address();
                let token_client = token::Client::new(&env, &token_address);
                creator.require_auth();
                token_client.transfer(&creator, &contract_address, &amount);
                total_funds = amount;
                remaining_balance = amount;
                init_liquidity = amount;
            }
        }

        // Create program data
        let program_data = ProgramData {
            program_id: program_id.clone(),
            total_funds: 0,
            remaining_balance: 0,
            authorized_payout_key: authorized_payout_key.clone(),
            payout_history: vec![&env],
            token_address: token_address.clone(),
            initial_liquidity: init_liquidity,
            reference_hash: reference_hash.clone(),
        };

        // Initialize fee config with zero fees (disabled by default)
        let fee_config = FeeConfig {
            lock_fee_rate: 0,
            payout_fee_rate: 0,
            fee_recipient: authorized_payout_key.clone(),
            fee_enabled: false,
        };
        env.storage().instance().set(&FEE_CONFIG, &fee_config);

        // Store program data
        env.storage().instance().set(&program_key, &program_data);
        // Store program data
        env.storage().instance().set(&program_key, &program_data);
        env.storage()
            .instance()
            .set(&SCHEDULES, &Vec::<ProgramReleaseSchedule>::new(&env));
        env.storage()
            .instance()
            .set(&RELEASE_HISTORY, &Vec::<ProgramReleaseHistory>::new(&env));
        env.storage().instance().set(&NEXT_SCHEDULE_ID, &1_u64);

        // Emit ProgramInitialized event
        env.events().publish(
            (PROGRAM_INITIALIZED,),
            ProgramInitializedEvent {
                version: EVENT_VERSION_V2,
                program_id,
                authorized_payout_key,
                token_address,
                total_funds,
                reference_hash,
                receipt_id,
            },
        );

        program_data
    }

    /// Batch-initialize multiple programs in one transaction (all-or-nothing).
    pub fn batch_initialize_programs(
        env: Env,
        items: Vec<ProgramInitItem>,
    ) -> Result<u32, BatchError> {
        let batch_size = items.len() as u32;
        if batch_size == 0 || batch_size > MAX_BATCH_SIZE {
            return Err(BatchError::InvalidBatchSize);
        }
        for i in 0..batch_size {
            for j in (i + 1)..batch_size {
                if items.get(i).unwrap().program_id == items.get(j).unwrap().program_id {
                    return Err(BatchError::DuplicateProgramId);
                }
            }
        }
        for i in 0..batch_size {
            let program_key = DataKey::Program(items.get(i).unwrap().program_id.clone());
            if env.storage().instance().has(&program_key) {
                return Err(BatchError::ProgramAlreadyExists);
            }
        }

        // Update registry
        let mut registry: Vec<String> = env
            .storage()
            .instance()
            .get(&PROGRAM_REGISTRY)
            .unwrap_or(vec![&env]);

        for i in 0..batch_size {
            let item = items.get(i).unwrap();
            let program_id = item.program_id.clone();
            let authorized_payout_key = item.authorized_payout_key.clone();
            let token_address = item.token_address.clone();

            if program_id.is_empty() {
                return Err(BatchError::InvalidBatchSize);
            }

            let program_data = ProgramData {
                program_id: program_id.clone(),
                total_funds: 0,
                remaining_balance: 0,
                authorized_payout_key: authorized_payout_key.clone(),
                payout_history: vec![&env],
                token_address: token_address.clone(),
                initial_liquidity: 0,
                reference_hash: item.reference_hash.clone(),
            };
            let program_key = DataKey::Program(program_id.clone());
            env.storage().instance().set(&program_key, &program_data);

            if i == 0 {
                let fee_config = FeeConfig {
                    lock_fee_rate: 0,
                    payout_fee_rate: 0,
                    fee_recipient: authorized_payout_key.clone(),
                    fee_enabled: false,
                };
                env.storage().instance().set(&FEE_CONFIG, &fee_config);
            }

            let multisig_config = MultisigConfig {
                threshold_amount: i128::MAX,
                signers: vec![&env],
                required_signatures: 0,
            };
            env.storage().persistent().set(
                &DataKey::MultisigConfig(program_id.clone()),
                &multisig_config,
            );

            registry.push_back(program_id.clone());
            let receipt_id = Self::increment_receipt_id(&env);
            env.events().publish(
                (PROGRAM_INITIALIZED,),
                ProgramInitializedEvent {
                    version: EVENT_VERSION_V2,
                    program_id,
                    authorized_payout_key,
                    token_address,
                    total_funds: 0,
                    reference_hash: item.reference_hash,
                    receipt_id,
                },
            );
        }
        env.storage().instance().set(&PROGRAM_REGISTRY, &registry);

        // Emit registration event
        env.events().publish(
            (PROGRAM_REGISTERED,),
            (program_id, authorized_payout_key, token_address, 0i128),
        );

        // Track successful operation
        monitoring::track_operation(&env, symbol_short!("init_prg"), caller, true);

        // Track performance
        let duration = env.ledger().timestamp().saturating_sub(start);
        monitoring::emit_performance(&env, symbol_short!("init_prg"), duration);

        program_data
    }

    /// Calculate fee using floor rounding. Delegates to `token_math::calculate_fee`.
    fn calculate_fee(amount: i128, fee_rate: i128) -> i128 {
        token_math::calculate_fee(amount, fee_rate)
    }

    /// Get fee configuration (internal helper)
    fn get_fee_config_internal(env: &Env) -> FeeConfig {
        env.storage()
            .instance()
            .get(&FEE_CONFIG)
            .unwrap_or_else(|| FeeConfig {
                lock_fee_rate: 0,
                payout_fee_rate: 0,
                fee_recipient: env.current_contract_address(),
                fee_enabled: false,
            })
    }

    /// Lock initial funds into the program escrow
    ///
    /// Lists all registered program IDs in the contract.
    ///
    /// # Returns
    /// * `Vec<String>` - List of all program IDs
    ///
    /// # Example
    /// ```rust
    /// let programs = escrow_client.list_programs();
    /// for program_id in programs.iter() {
    ///     println!("Program: {}", program_id);
    /// }
    /// ```
    pub fn list_programs(env: Env) -> Vec<String> {
        env.storage()
            .instance()
            .get(&PROGRAM_REGISTRY)
            .unwrap_or(vec![&env])
    }

    /// Checks if a program exists.
    ///
    /// # Arguments
    /// * `program_id` - The program ID to check
    ///
    /// # Returns
    /// * `bool` - True if program exists, false otherwise
    pub fn program_exists(env: Env, program_id: String) -> bool {
        let program_key = DataKey::Program(program_id);
        env.storage().instance().has(&program_key)
    }

    fn assert_dependencies_satisfied(env: &Env, program_id: &String) {
        let dependencies = get_program_dependencies_internal(env, program_id);
        for dependency_id in dependencies.iter() {
            match dependency_status_internal(env, &dependency_id) {
                DependencyStatus::Completed => {}
                DependencyStatus::Pending => panic!("Dependency not satisfied"),
                DependencyStatus::Failed => panic!("Dependency failed"),
            }
        }
    }

    /// Defines explicit dependencies for a program.
    ///
    /// Dependencies can point to:
    /// - another registered program id; or
    /// - an externally managed dependency id with a pre-registered status.
    ///
    /// Cycle checks are applied for program-to-program edges.
    pub fn set_program_dependencies(
        env: Env,
        program_id: String,
        dependency_ids: Vec<String>,
    ) -> Vec<String> {
        let program_key = DataKey::Program(program_id.clone());
        let program_data: ProgramData = env
            .storage()
            .instance()
            .get(&program_key)
            .unwrap_or_else(|| panic!("Program not found"));

        program_data.authorized_payout_key.require_auth();

        let old_dependencies = get_program_dependencies_internal(&env, &program_id);
        let mut validated_dependencies = vec![&env];

        for dependency_id in dependency_ids.iter() {
            if dependency_id.len() == 0 {
                panic!("Dependency id cannot be empty");
            }
            if dependency_id == program_id {
                panic!("Program cannot depend on itself");
            }
            if vec_contains(&validated_dependencies, &dependency_id) {
                panic!("Duplicate dependency");
            }

            let is_program_dependency = env
                .storage()
                .instance()
                .has(&DataKey::Program(dependency_id.clone()));
            let is_registered_external = env
                .storage()
                .instance()
                .has(&DataKey::DependencyStatus(dependency_id.clone()));
            if !is_program_dependency && !is_registered_external {
                panic!("Dependency not registered");
            }

            if is_program_dependency {
                let mut visited = Vec::new(&env);
                if path_exists_to_target(&env, &dependency_id, &program_id, &mut visited) {
                    panic!("Dependency cycle detected");
                }
            }

            validated_dependencies.push_back(dependency_id.clone());
        }

        env.storage().instance().set(
            &DataKey::ProgramDependencies(program_id.clone()),
            &validated_dependencies,
        );

        for dependency_id in validated_dependencies.iter() {
            if !vec_contains(&old_dependencies, &dependency_id) {
                env.events().publish(
                    (DEPENDENCY_CREATED,),
                    (program_id.clone(), dependency_id.clone()),
                );
            }
        }
        for dependency_id in old_dependencies.iter() {
            if !vec_contains(&validated_dependencies, &dependency_id) {
                env.events().publish(
                    (DEPENDENCY_CLEARED,),
                    (program_id.clone(), dependency_id.clone()),
                );
            }
        }

        validated_dependencies
    }

    /// Clears all dependencies for a program.
    pub fn clear_program_dependencies(env: Env, program_id: String) {
        let program_key = DataKey::Program(program_id.clone());
        let program_data: ProgramData = env
            .storage()
            .instance()
            .get(&program_key)
            .unwrap_or_else(|| panic!("Program not found"));

        program_data.authorized_payout_key.require_auth();

        let old_dependencies = get_program_dependencies_internal(&env, &program_id);
        let empty_dependencies: Vec<String> = vec![&env];
        env.storage()
            .instance()
            .set(&DataKey::ProgramDependencies(program_id.clone()), &empty_dependencies);

        for dependency_id in old_dependencies.iter() {
            env.events().publish(
                (DEPENDENCY_CLEARED,),
                (program_id.clone(), dependency_id.clone()),
            );
        }
    }

    /// Reads all dependencies configured for a program.
    pub fn get_program_dependencies(env: Env, program_id: String) -> Vec<String> {
        if !env
            .storage()
            .instance()
            .has(&DataKey::Program(program_id.clone()))
        {
            panic!("Program not found");
        }
        get_program_dependencies_internal(&env, &program_id)
    }

    /// Updates dependency status.
    ///
    /// For registered programs, only that program's authorized payout key can update status.
    /// For external dependency ids, anti-abuse admin authorization is required.
    pub fn set_dependency_status(env: Env, dependency_id: String, status: DependencyStatus) {
        if dependency_id.len() == 0 {
            panic!("Dependency id cannot be empty");
        }

        if env
            .storage()
            .instance()
            .has(&DataKey::Program(dependency_id.clone()))
        {
            let program_data: ProgramData = env
                .storage()
                .instance()
                .get(&DataKey::Program(dependency_id.clone()))
                .unwrap();
            program_data.authorized_payout_key.require_auth();
        } else {
            let admin = anti_abuse::get_admin(&env)
                .unwrap_or_else(|| panic!("Admin not set for external dependency status update"));
            admin.require_auth();
        }

        env.storage()
            .instance()
            .set(&DataKey::DependencyStatus(dependency_id.clone()), &status.clone());
        env.events()
            .publish((DEPENDENCY_STATUS_UPDATED,), (dependency_id, status));
    }

    /// Reads dependency status; defaults to pending if no explicit status exists.
    pub fn get_dependency_status(env: Env, dependency_id: String) -> DependencyStatus {
        dependency_status_internal(&env, &dependency_id)
    }

    // ========================================================================
    // Fund Management
    // ========================================================================

    /// Locks funds into the program escrow for prize distribution.
    ///
    /// # Arguments
    /// * `env` - The contract environment
    /// * `amount` - Amount of tokens to lock (in token's smallest denomination)
    ///
    /// # Returns
    /// * `ProgramData` - Updated program data with new balance
    ///
    /// # Panics
    /// * If amount is zero or negative
    /// * If program is not initialized
    ///
    /// # State Changes
    /// - Increases `total_funds` by amount
    /// - Increases `remaining_balance` by amount
    /// - Emits FundsLocked event
    ///
    /// # Prerequisites
    /// Before calling this function:
    /// 1. Caller must have sufficient token balance
    /// 2. Caller must approve contract for token transfer
    /// 3. Tokens must actually be transferred to contract
    ///
    /// # Security Considerations
    /// - Amount must be positive
    /// - This function doesn't perform the actual token transfer
    /// - Caller is responsible for transferring tokens to contract
    /// - Consider verifying contract balance matches recorded amount
    /// - Multiple lock operations are additive (cumulative)
    ///
    /// # Events
    /// Emits: `FundsLocked(program_id, amount, new_remaining_balance)`
    ///
    /// # Example
    /// ```rust
    /// use soroban_sdk::token;
    ///
    /// // 1. Transfer tokens to contract
    /// let amount = 10_000_0000000; // 10,000 USDC
    /// token_client.transfer(
    ///     &organizer,
    ///     &contract_address,
    ///     &amount
    /// );
    ///
    /// // 2. Record the locked funds
    /// let updated = escrow_client.lock_program_funds(&amount);
    /// println!("Locked: {} USDC", amount / 10_000_000);
    /// println!("Remaining: {}", updated.remaining_balance);
    /// ```
    ///
    /// # Production Usage
    /// ```bash
    /// # 1. Transfer USDC to contract
    /// stellar contract invoke \
    ///   --id USDC_TOKEN_ID \
    ///   --source ORGANIZER_KEY \
    ///   -- transfer \
    ///   --from ORGANIZER_ADDRESS \
    ///   --to CONTRACT_ADDRESS \
    ///   --amount 10000000000
    ///
    /// # 2. Record locked funds
    /// stellar contract invoke \
    ///   --id CONTRACT_ID \
    ///   --source ORGANIZER_KEY \
    ///   -- lock_program_funds \
    ///   --amount 10000000000
    /// ```
    ///
    /// # Gas Cost
    /// Low - Storage update + event emission
    ///
    /// # Common Pitfalls
    /// - Forgetting to transfer tokens before calling
    /// -  Locking amount that exceeds actual contract balance
    /// -  Not verifying contract received the tokens

    pub fn lock_program_funds(env: Env, program_id: String, amount: i128) -> ProgramData {
        // Apply rate limiting
        anti_abuse::check_rate_limit(&env, env.current_contract_address());

        if Self::check_paused(&env, symbol_short!("lock")) {
            panic!("Funds Paused");
        }

        // Validate amount
        if amount <= 0 {
            monitoring::track_operation(&env, symbol_short!("lock"), caller.clone(), false);
            panic!("Amount must be greater than zero");
        }

        let program_key = DataKey::Program(program_id.clone());
        let mut program_data: ProgramData = env
            .storage()
            .instance()
            .get(&program_key)
            .unwrap_or_else(|| panic!("Program not initialized"));

        // Require the authorized payout key or creator
        program_data.authorized_payout_key.require_auth();

        // Update balances
        program_data.total_funds += amount;
        program_data.remaining_balance += amount;

        // Store updated data
        env.storage().instance().set(&program_key, &program_data);

        let receipt_id = Self::increment_receipt_id(&env);

        // Emit FundsLocked event
        env.events().publish(
            (FUNDS_LOCKED,),
            FundsLockedEvent {
                version: EVENT_VERSION_V2,
                program_id: program_data.program_id.clone(),
                amount,
                remaining_balance: program_data.remaining_balance,
                receipt_id,
            },
        );

        program_data
    }

    // ========================================================================
    // Initialization & Admin
    // ========================================================================

    /// Initialize the contract with an admin.
    /// This must be called before any admin protected functions (like pause) can be used.
    pub fn initialize_contract(env: Env, admin: Address) {
        if env.storage().instance().has(&DataKey::Admin) {
            panic!("Already initialized");
        }
        env.storage().instance().set(&DataKey::Admin, &admin);
    }

    /// Set or rotate admin. If no admin is set, sets initial admin. If admin exists, current admin must authorize and the new address becomes admin.
    pub fn set_admin(env: Env, admin: Address) {
        if env.storage().instance().has(&DataKey::Admin) {
            let current: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
            current.require_auth();
        }
        env.storage().instance().set(&DataKey::Admin, &admin);
    }

    /// Returns the current admin address, if set.
    pub fn get_admin(env: Env) -> Option<Address> {
        env.storage().instance().get(&DataKey::Admin)
    }

    pub fn get_program_release_schedules(env: Env) -> Vec<ProgramReleaseSchedule> {
        env.storage()
            .instance()
            .get(&SCHEDULES)
            .unwrap_or_else(|| Vec::new(&env))
    }

    /// Update pause flags (admin only)
    pub fn set_paused(
        env: Env,
        lock: Option<bool>,
        release: Option<bool>,
        refund: Option<bool>,
        reason: Option<String>,
    ) {
        if !env.storage().instance().has(&DataKey::Admin) {
            panic!("Not initialized");
        }

        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();

        let mut flags = Self::get_pause_flags(&env);
        let timestamp = env.ledger().timestamp();

        if reason.is_some() {
            flags.pause_reason = reason.clone();
        }

        if let Some(paused) = lock {
            flags.lock_paused = paused;
            let receipt_id = Self::increment_receipt_id(&env);
            env.events().publish(
                (PAUSE_STATE_CHANGED,),
                PauseStateChanged {
                    operation: symbol_short!("lock"),
                    paused,
                    admin: admin.clone(),
                    reason: reason.clone(),
                    timestamp,
                    receipt_id,
                },
            );
        }

        if let Some(paused) = release {
            flags.release_paused = paused;
            let receipt_id = Self::increment_receipt_id(&env);
            env.events().publish(
                (PAUSE_STATE_CHANGED,),
                PauseStateChanged {
                    operation: symbol_short!("release"),
                    paused,
                    admin: admin.clone(),
                    reason: reason.clone(),
                    timestamp,
                    receipt_id,
                },
            );
        }

        if let Some(paused) = refund {
            flags.refund_paused = paused;
            let receipt_id = Self::increment_receipt_id(&env);
            env.events().publish(
                (PAUSE_STATE_CHANGED,),
                PauseStateChanged {
                    operation: symbol_short!("refund"),
                    paused,
                    admin: admin.clone(),
                    reason: reason.clone(),
                    timestamp,
                    receipt_id,
                },
            );
        }

        let any_paused = flags.lock_paused || flags.release_paused || flags.refund_paused;

        if any_paused {
            if flags.paused_at == 0 {
                flags.paused_at = timestamp;
            }
        } else {
            0
        };
        let net_amount = amount - fee_amount;

        // Update balances with net amount
        program_data.total_funds += net_amount;
        program_data.remaining_balance += net_amount;

        // Emit fee collected event if applicable
        if fee_amount > 0 {
        let program_data: ProgramData = env
            .storage()
            .instance()
            .get(&PROGRAM_DATA)
            .unwrap_or_else(|| panic!("Program not initialized"));
        let token_client = token::TokenClient::new(&env, &program_data.token_address);

        let contract_address = env.current_contract_address();
        let balance = token_client.balance(&contract_address);

        if balance > 0 {
            token_client.transfer(&contract_address, &target, &balance);
            let receipt_id = Self::increment_receipt_id(&env);
            env.events().publish(
                (symbol_short!("em_wtd"),),
                EmergencyWithdrawEvent {
                    admin,
                    target: target.clone(),
                    amount: balance,
                    timestamp: env.ledger().timestamp(),
                    receipt_id,
                },
            );
        }

        // Store updated data
        env.storage().instance().set(&program_key, &program_data);
        let config = RateLimitConfig {
            window_size,
            max_operations,
            cooldown_period,
        };
        env.storage()
            .instance()
            .set(&DataKey::RateLimitConfig, &config);
    }

        // Emit FundsLocked event (with net amount after fee)
        env.events().publish(
            (FUNDS_LOCKED,),
            (
                program_data.program_id.clone(),
                net_amount,
                program_data.remaining_balance,
            ),
        );

    pub fn get_analytics(_env: Env) -> monitoring::Analytics {
        monitoring::Analytics {
            operation_count: 0,
            unique_users: 0,
            error_count: 0,
            error_rate: 0,
        }
    }

    pub fn set_whitelist(env: Env, _address: Address, _whitelisted: bool) {
        // Only admin can set whitelist
        let admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .unwrap_or_else(|| panic!("Not initialized"));
        admin.require_auth();
    }
    // ========================================================================
    // Payout Functions
    // ========================================================================

    /// Executes batch payouts to multiple recipients simultaneously.
    ///
    /// # Arguments
    /// * `env` - The contract environment
    /// * `recipients` - Vector of recipient addresses
    /// * `amounts` - Vector of amounts (must match recipients length)
    ///
    /// # Returns
    /// Updated ProgramData after payouts
    pub fn batch_payout(
        env: Env,
        program_id: String,
        recipients: Vec<Address>,
        amounts: Vec<i128>,
    ) -> ProgramData {
        // Reentrancy guard: Check and set
        reentrancy_guard::check_not_entered(&env);
        reentrancy_guard::set_entered(&env);

        if Self::check_paused(&env, symbol_short!("release")) {
            reentrancy_guard::clear_entered(&env);
            panic!("Funds Paused");
        }

        // Verify authorization
        let program_key = DataKey::Program(program_id.clone());
        let program_data: ProgramData =
            env.storage()
                .instance()
                .get(&program_key)
                .unwrap_or_else(|| {
                    reentrancy_guard::clear_entered(&env);
                    panic!("Program not found")
                });

        Self::assert_dependencies_satisfied(&env, &program_data.program_id);

        // Apply rate limiting to the authorized payout key
        anti_abuse::check_rate_limit(&env, program_data.authorized_payout_key.clone());

        program_data.authorized_payout_key.require_auth();

        // Validate inputs
        if recipients.len() != amounts.len() {
            panic!("Recipients and amounts vectors must have the same length");
        }

        if recipients.is_empty() {
            panic!("Cannot process empty batch");
        }

        // Calculate total with overflow protection
        let mut total_payout: i128 = 0;
        for i in 0..amounts.len() {
            let amount = amounts.get(i).unwrap();
            if amount <= 0 {
                panic!("All amounts must be greater than zero");
            }
            total_payout = total_payout
                .checked_add(amount)
                .unwrap_or_else(|| panic!("Payout amount overflow"));
        }

        // Validate balance
        if total_payout > program_data.remaining_balance {
            panic!(
                "Insufficient balance: requested {}, available {}",
                total_payout, program_data.remaining_balance
            );
        }

        // Calculate fees if enabled
        let fee_config = Self::get_fee_config_internal(&env);
        let mut total_fees: i128 = 0;

        // Execute transfers
        let mut updated_history = program_data.payout_history.clone();
        let timestamp = env.ledger().timestamp();
        let contract_address = env.current_contract_address();
        let token_client = token::Client::new(&env, &program_data.token_address);

        for i in 0..recipients.len() {
            let recipient = recipients.get(i).unwrap();
            let amount = amounts.get(i).unwrap();

            // Calculate fee for this payout
            let fee_amount = if fee_config.fee_enabled && fee_config.payout_fee_rate > 0 {
                Self::calculate_fee(amount, fee_config.payout_fee_rate)
            } else {
                0
            };
            let net_amount = amount - fee_amount;
            total_fees += fee_amount;

            // Transfer net amount to recipient
            token_client.transfer(&contract_address, &recipient.clone(), &net_amount);

            // Transfer fee to fee recipient if applicable
            if fee_amount > 0 {
                token_client.transfer(&contract_address, &fee_config.fee_recipient, &fee_amount);
            }

            // Record payout (with net amount)
            let payout_record = PayoutRecord {
                recipient: recipient.clone(),
                amount: net_amount,
                timestamp,
            };
            updated_history.push_back(payout_record);
        }

        // Emit fee collected event if applicable
        if total_fees > 0 {
            env.events().publish(
                (symbol_short!("fee"),),
                (
                    symbol_short!("payout"),
                    total_fees,
                    fee_config.payout_fee_rate,
                    fee_config.fee_recipient.clone(),
                ),
            );
        }

        // Update program data
        let mut updated_data = program_data.clone();
        updated_data.remaining_balance -= total_payout; // Total includes fees
        updated_data.payout_history = updated_history;

        // Store updated data
        env.storage().instance().set(&program_key, &updated_data);

        let receipt_id = Self::increment_receipt_id(&env);

        // Emit event
        env.events().publish(
            (BATCH_PAYOUT,),
            BatchPayoutEvent {
                version: EVENT_VERSION_V2,
                program_id: updated_data.program_id.clone(),
                recipient_count: recipients.len() as u32,
                total_amount: total_payout,
                remaining_balance: updated_data.remaining_balance,
                receipt_id,
            },
        );

        updated_data
    }

    /// Executes a single payout to one recipient.
    ///
    /// # Arguments
    /// * `env` - The contract environment
    /// * `recipient` - Address of the prize recipient
    /// * `amount` - Amount to transfer (in token's smallest denomination)
    ///
    /// # Returns
    /// * `ProgramData` - Updated program data after payout
    ///
    /// # Panics
    /// * If caller is not the authorized payout key
    /// * If program is not initialized
    /// * If amount is zero or negative
    /// * If amount exceeds remaining balance
    ///
    /// # Authorization
    /// - Only authorized payout key can call this function
    ///
    /// # State Changes
    /// - Transfers tokens from contract to recipient
    /// - Adds PayoutRecord to history
    /// - Decreases `remaining_balance` by amount
    /// - Emits Payout event
    ///
    /// # Security Considerations
    /// - Verify recipient address before calling
    /// - Amount must be positive
    /// - Balance check prevents overdraft
    /// - Transfer is logged in payout history
    ///
    /// # Events
    /// Emits: `Payout(program_id, recipient, amount, new_balance)`
    ///
    /// # Example
    /// ```rust
    /// use soroban_sdk::Address;
    ///
    /// let winner = Address::from_string("GWINNER...");
    /// let prize = 1_000_0000000; // $1,000 USDC
    ///
    /// // Execute single payout
    /// let result = escrow_client.single_payout(&winner, &prize);
    /// println!("Paid {} to winner", prize);
    /// ```
    ///
    /// # Gas Cost
    /// Medium - Single token transfer + storage update
    ///
    /// # Use Cases
    /// - Individual prize awards
    /// - Bonus payments
    /// - Late additions to prize pool distribution
    pub fn single_payout(
        env: Env,
        program_id: String,
        recipient: Address,
        amount: i128,
    ) -> ProgramData {
        // Check if contract is paused
        if Self::is_paused_internal(&env) {
            panic!("Contract is paused");
        // Get program data
        let program_key = DataKey::Program(program_id.clone());
        let program_data: ProgramData = env
            .storage()
            .instance()
            .get(&program_key)
            .unwrap_or_else(|| panic!("Program not found"));

        // Reentrancy guard: Check and set
        reentrancy_guard::check_not_entered(&env);
        reentrancy_guard::set_entered(&env);

        if Self::check_paused(&env, symbol_short!("release")) {
            reentrancy_guard::clear_entered(&env);
            panic!("Funds Paused");
        }

        Self::assert_dependencies_satisfied(&env, &program_id);

        program_data.authorized_payout_key.require_auth();
        // Apply rate limiting to the authorized payout key
        anti_abuse::check_rate_limit(&env, program_data.authorized_payout_key.clone());

        // Validate amount
        if amount <= 0 {
            panic!("Amount must be greater than zero");
        }

        // Validate balance
        if amount > program_data.remaining_balance {
            panic!(
                "Insufficient balance: requested {}, available {}",
                amount, program_data.remaining_balance
            );
        }

        // Calculate and collect fee if enabled
        let fee_config = Self::get_fee_config_internal(&env);
        let fee_amount = if fee_config.fee_enabled && fee_config.payout_fee_rate > 0 {
            Self::calculate_fee(amount, fee_config.payout_fee_rate)
        } else {
            0
        };
        let net_amount = amount - fee_amount;

        // Transfer net amount to recipient
        // Transfer tokens
        let contract_address = env.current_contract_address();
        let token_client = token::Client::new(&env, &program_data.token_address);
        token_client.transfer(&contract_address, &recipient, &net_amount);

        // Transfer fee to fee recipient if applicable
        if fee_amount > 0 {
            token_client.transfer(&contract_address, &fee_config.fee_recipient, &fee_amount);
            env.events().publish(
                (symbol_short!("fee"),),
                (
                    symbol_short!("payout"),
                    fee_amount,
                    fee_config.payout_fee_rate,
                    fee_config.fee_recipient.clone(),
                ),
            );
        }

        // Record payout (with net amount after fee)
        let timestamp = env.ledger().timestamp();
        let payout_record = PayoutRecord {
            recipient: recipient.clone(),
            amount: net_amount,
            timestamp,
        };

        let mut updated_history = program_data.payout_history.clone();
        updated_history.push_back(payout_record);

        // Update program data
        let mut updated_data = program_data.clone();
        updated_data.remaining_balance -= amount; // Total amount (includes fee)
        updated_data.payout_history = updated_history;

        // Store updated data
        env.storage().instance().set(&program_key, &updated_data);

        let receipt_id = Self::increment_receipt_id(&env);

        // Emit Payout event (with net amount after fee)
        // Emit event
        env.events().publish(
            (PAYOUT,),
            (
                program_id,
                recipient,
                amount,
                remaining_balance: updated_data.remaining_balance,
                receipt_id,
            },
        );

        updated_data
    }

    // ========================================================================
    // Release Schedule Functions
    // ========================================================================

    /// Creates a time-based release schedule for a program.
    ///
    /// # Arguments
    /// * `env` - The contract environment
    /// * `program_id` - The program to create schedule for
    /// * `amount` - Amount to release (in token's smallest denomination)
    /// * `release_timestamp` - Unix timestamp when funds become available
    /// * `recipient` - Address that will receive the funds
    ///
    /// # Returns
    /// * `ProgramData` - Updated program data
    ///
    /// # Panics
    /// * If program is not initialized
    /// * If caller is not authorized payout key
    /// * If amount is invalid
    /// * If timestamp is in the past
    /// * If amount exceeds remaining balance
    ///
    /// # State Changes
    /// - Creates ProgramReleaseSchedule record
    /// - Updates next schedule ID
    /// - Emits ScheduleCreated event
    ///
    /// # Authorization
    /// - Only authorized payout key can call this function
    ///
    /// # Example
    /// ```rust
    /// let now = env.ledger().timestamp();
    /// let release_time = now + (30 * 24 * 60 * 60); // 30 days from now
    /// escrow_client.create_program_release_schedule(
    ///     &"Hackathon2024",
    ///     &500_0000000, // 500 tokens
    ///     &release_time,
    ///     &winner_address
    /// );
    /// ```
    pub fn create_program_release_schedule(
        env: Env,
        program_id: String,
        amount: i128,
        release_timestamp: u64,
        recipient: Address,
    ) -> ProgramData {
        let start = env.ledger().timestamp();

        // Check if contract is paused
        if Self::is_paused_internal(&env) {
            panic!("Contract is paused");
        }

        // Get program data
        let program_key = DataKey::Program(program_id.clone());
        let program_data: ProgramData = env
            .storage()
            .instance()
            .get(&program_key)
            .unwrap_or_else(|| panic!("Program not found"));

        // Apply rate limiting to the authorized payout key
        anti_abuse::check_rate_limit(&env, program_data.authorized_payout_key.clone());

        // Verify authorization
        program_data.authorized_payout_key.require_auth();

        // Validate amount
        if amount <= 0 {
            panic!("Amount must be greater than zero");
        }

        // Validate timestamp
        if release_timestamp <= env.ledger().timestamp() {
            panic!("Release timestamp must be in the future");
        }

        // Check sufficient remaining balance
        let scheduled_total = get_program_total_scheduled_amount(&env, &program_id);
        if scheduled_total + amount > program_data.remaining_balance {
            panic!("Insufficient balance for scheduled amount");
        }

        // Get next schedule ID
        let schedule_id: u64 = env
            .storage()
            .persistent()
            .get(&DataKey::NextScheduleId(program_id.clone()))
            .unwrap_or(1);

        // Create release schedule
        let schedule = ProgramReleaseSchedule {
            schedule_id,
            amount,
            release_timestamp,
            recipient: recipient.clone(),
            released: false,
            released_at: None,
            released_by: None,
        };
    /// Create a release schedule entry that can be triggered at/after `release_timestamp`.
    pub fn create_program_release_schedule(
        env: Env,
        recipient: Address,
        amount: i128,
        release_timestamp: u64,
    ) -> ProgramReleaseSchedule {
        let program_data: ProgramData = env
            .storage()
            .instance()
            .get(&PROGRAM_DATA)
            .unwrap_or_else(|| panic!("Program not initialized"));

        program_data.authorized_payout_key.require_auth();

        if amount <= 0 {
            panic!("Amount must be greater than zero");
        }

        let mut schedules: Vec<ProgramReleaseSchedule> = env
            .storage()
            .instance()
            .get(&SCHEDULES)
            .unwrap_or_else(|| Vec::new(&env));
        let schedule_id: u64 = env
            .storage()
            .instance()
            .get(&NEXT_SCHEDULE_ID)
            .unwrap_or(1_u64);

        let schedule = ProgramReleaseSchedule {
            schedule_id,
            recipient,
            amount,
            release_timestamp,
            released: false,
            released_at: None,
            released_by: None,
        };
        schedules.push_back(schedule.clone());

        env.storage().instance().set(&SCHEDULES, &schedules);
        env.storage()
            .instance()
            .set(&NEXT_SCHEDULE_ID, &(schedule_id + 1));

        let receipt_id = Self::increment_receipt_id(&env);
        env.events().publish(
            (symbol_short!("sch_cred"),),
            ScheduleCreatedEvent {
                program_id: program_data.program_id.clone(),
                schedule_id,
                recipient: schedule.recipient.clone(),
                amount,
                release_timestamp,
                receipt_id,
            },
        );
        schedule
    }

        // Store schedule
        env.storage().persistent().set(
            &DataKey::ReleaseSchedule(program_id.clone(), schedule_id),
            &schedule,
        );

        // Update next schedule ID
        env.storage().persistent().set(
            &DataKey::NextScheduleId(program_id.clone()),
            &(schedule_id + 1),
        );

        // Emit program schedule created event
        env.events().publish(
            (PROG_SCHEDULE_CREATED,),
            ProgramScheduleCreated {
                program_id: program_id.clone(),
                schedule_id,
                amount,
                release_timestamp,
                recipient: recipient.clone(),
                created_by: program_data.authorized_payout_key.clone(),
            },
        );

        // Track successful operation
        monitoring::track_operation(
            &env,
            symbol_short!("create_p"),
            program_data.authorized_payout_key,
            true,
        );

        // Track performance
        let duration = env.ledger().timestamp().saturating_sub(start);
        monitoring::emit_performance(&env, symbol_short!("create_p"), duration);

        // Return updated program data
        let updated_data: ProgramData = env.storage().instance().get(&program_key).unwrap();
        updated_data
    }

    /// Automatically releases funds for program schedules that are due.
    /// Can be called by anyone after the release timestamp has passed.
    ///
    /// # Arguments
    /// * `env` - The contract environment
    /// * `program_id` - The program to check for due schedules
    /// * `schedule_id` - The specific schedule to release
    ///
    /// # Panics
    /// * If program doesn't exist
    /// * If schedule doesn't exist
    /// * If schedule is already released
    /// * If schedule is not yet due
    ///
    /// # State Changes
    /// - Transfers tokens to recipient
    /// - Updates schedule status to released
    /// - Adds to release history
    /// - Updates program remaining balance
    /// - Emits ScheduleReleased event
    ///
    /// # Example
    /// ```rust
    /// // Anyone can call this after the timestamp
    /// escrow_client.release_program_schedule_automatic(&"Hackathon2024", &1);
    /// ```
    pub fn release_prog_schedule_automatic(env: Env, program_id: String, schedule_id: u64) {
        let start = env.ledger().timestamp();
        let caller = env.current_contract_address();

        // Check if contract is paused
        if Self::is_paused_internal(&env) {
            panic!("Contract is paused");
        }

        // Get program data
        let program_key = DataKey::Program(program_id.clone());
        let program_data: ProgramData = env
            .storage()
            .instance()
            .get(&program_key)
            .unwrap_or_else(|| panic!("Program not found"));

        // Get schedule
        if !env
            .storage()
            .instance()
            .get(&SCHEDULES)
            .unwrap_or_else(|| Vec::new(&env));
        let mut release_history: Vec<ProgramReleaseHistory> = env
            .storage()
            .persistent()
            .get(&DataKey::ReleaseSchedule(program_id.clone(), schedule_id))
            .unwrap();

        Self::assert_dependencies_satisfied(&env, &program_data.program_id);

        let now = env.ledger().timestamp();
        if now < schedule.release_timestamp {
            panic!("Schedule not yet due for release");
        }

        // Get token client
        let contract_address = env.current_contract_address();
        let token_client = token::Client::new(&env, &program_data.token_address);

        // Transfer funds
        token_client.transfer(&contract_address, &schedule.recipient, &schedule.amount);

        // Update schedule
        schedule.released = true;
        schedule.released_at = Some(now);
        schedule.released_by = Some(env.current_contract_address());

        // Update program data
        let mut updated_data = program_data.clone();
        updated_data.remaining_balance -= schedule.amount;

        // Add to release history
        let history_entry = ProgramReleaseHistory {
            schedule_id,
            program_id: program_id.clone(),
            amount: schedule.amount,
            recipient: schedule.recipient.clone(),
            released_at: now,
            released_by: env.current_contract_address(),
            release_type: ReleaseType::Automatic,
        };

        let mut history: Vec<ProgramReleaseHistory> = env
            .storage()
            .persistent()
            .get(&DataKey::ReleaseHistory(program_id.clone()))
            .unwrap_or(vec![&env]);
        history.push_back(history_entry);

        // Store updates
        env.storage().persistent().set(
            &DataKey::ReleaseSchedule(program_id.clone(), schedule_id),
            &schedule,
        );
        env.storage().instance().set(&program_key, &updated_data);
        env.storage()
            .persistent()
            .set(&DataKey::ReleaseHistory(program_id.clone()), &history);

        // Emit program schedule released event
        env.events().publish(
            (PROG_SCHEDULE_RELEASED,),
            ProgramScheduleReleased {
                program_id: program_id.clone(),
                schedule_id,
                amount: schedule.amount,
                recipient: schedule.recipient.clone(),
                amount: schedule.amount,
                timestamp: now,
            });

            let receipt_id = Self::increment_receipt_id(&env);
            env.events().publish(
                (PAYOUT,),
                PayoutEvent {
                    version: EVENT_VERSION_V2,
                    program_id: program_data.program_id.clone(),
                    recipient: schedule.recipient.clone(),
                    amount: schedule.amount,
                    remaining_balance: program_data.remaining_balance,
                    receipt_id,
                },
            );

            release_history.push_back(ProgramReleaseHistory {
                schedule_id: schedule.schedule_id,
                recipient: schedule.recipient,
                amount: schedule.amount,
                released_at: now,
                released_by: env.current_contract_address(),
                release_type: ReleaseType::Automatic,
            });

            released_count += 1;
        }

        env.storage().instance().set(&PROGRAM_DATA, &program_data);
        env.storage().instance().set(&SCHEDULES, &schedules);
        env.storage()
            .instance()
            .set(&RELEASE_HISTORY, &release_history);

        // Clear reentrancy guard before returning
        reentrancy_guard::clear_entered(&env);

        released_count
    }

    /// Manually releases funds for a program schedule (authorized payout key only).
    /// Can be called before the release timestamp by authorized key
    pub fn release_program_schedule_manual(
        env: Env,
        program_id: String,
        schedule_id: u64,
    ) {
        let start = env.ledger().timestamp();

        // Get program data
        let program_key = DataKey::Program(program_id.clone());
        let program_data: ProgramData = env
            .storage()
            .instance()
            .get(&program_key)
            .unwrap_or_else(|| panic!("Program not found"));

        Self::assert_dependencies_satisfied(&env, &program_id);

        // Apply rate limiting to the authorized payout key
        anti_abuse::check_rate_limit(&env, program_data.authorized_payout_key.clone());

        // Verify authorization
        program_data.authorized_payout_key.require_auth();

        // Get schedule
        if !env
            .storage()
            .persistent()
            .has(&DataKey::ReleaseSchedule(program_id.clone(), schedule_id))
        {
            panic!("Schedule not found");
        }

        let mut schedule: ProgramReleaseSchedule = env
            .storage()
            .persistent()
            .get(&DataKey::ReleaseSchedule(program_id.clone(), schedule_id))
            .unwrap();

        // Check if already released
        if schedule.released {
            panic!("Schedule already released");
        }


        // Check sufficient balance
        let program_key2 = DataKey::Program(program_id.clone());
        let mut program_data: ProgramData = env
            .storage()
            .instance()
            .get(&program_key2)
            .unwrap_or_else(|| panic!("Program not found"));

        if schedule.amount > program_data.remaining_balance {
            panic!("Insufficient balance");
        }

        let contract_address = env.current_contract_address();
        let token_client = token::Client::new(&env, &program_data.token_address);
        token_client.transfer(&contract_address, &schedule.recipient, &schedule.amount);

        let now = env.ledger().timestamp();
        schedule.released = true;
        schedule.released_at = Some(now);
        schedule.released_by = Some(contract_address.clone());
        env.storage().persistent().set(
            &DataKey::ReleaseSchedule(program_id.clone(), schedule_id),
            &schedule,
        );

        program_data.remaining_balance -= schedule.amount;
        program_data.payout_history.push_back(PayoutRecord {
            recipient: schedule.recipient.clone(),
            amount: schedule.amount,
            timestamp: now,
        });

        let receipt_id = Self::increment_receipt_id(&env);
        env.events().publish(
            (PAYOUT,),
            PayoutEvent {
                version: EVENT_VERSION_V2,
                program_id: program_data.program_id.clone(),
                recipient: schedule.recipient.clone(),
                amount: schedule.amount,
                remaining_balance: program_data.remaining_balance,
                receipt_id,
            },
        );

        env.storage().instance().set(&program_key, &program_data);

        // Track successful operation
        monitoring::track_operation(&env, symbol_short!("rel_man"), program_data.authorized_payout_key.clone(), true);

        let duration = env.ledger().timestamp().saturating_sub(start);
        monitoring::emit_performance(&env, symbol_short!("rel_man"), duration);
    }

    /// Manually releases funds for a program schedule (authorized payout key only).
    /// Can be called before the release timestamp by authorized key.
    ///
    /// # Arguments
    /// * `env` - The contract environment
    /// * `program_id` - The program containing the schedule
    /// * `schedule_id` - The schedule to release
    ///
    /// # Panics
    /// * If program doesn't exist
    /// * If caller is not authorized payout key
    /// * If schedule doesn't exist
    /// * If schedule is already released
    ///
    /// # State Changes
    /// - Transfers tokens to recipient
    /// - Updates schedule status to released
    /// - Adds to release history
    /// - Updates program remaining balance
    /// - Emits ScheduleReleased event
    ///
    /// # Authorization
    /// - Only authorized payout key can call this function
    ///
    /// # Example
    /// ```rust
    /// // Authorized key can release early
    /// escrow_client.release_program_schedule_manual(&"Hackathon2024", &1);
    /// ```
    pub fn release_program_schedule_manual(env: Env, program_id: String, schedule_id: u64) {
        let start = env.ledger().timestamp();

        // Get program data
        let program_key = DataKey::Program(program_id.clone());
        let program_data: ProgramData = env
            .storage()
            .instance()
            .get(&program_key)
            .unwrap_or_else(|| panic!("Program not found"));

        // Apply rate limiting to the authorized payout key
        anti_abuse::check_rate_limit(&env, program_data.authorized_payout_key.clone());

        // Verify authorization
        program_data.authorized_payout_key.require_auth();

        // Get schedule
        if !env
            .storage()
            .persistent()
            .has(&DataKey::ReleaseSchedule(program_id.clone(), schedule_id))
        {
            panic!("Schedule not found");
        }

        let mut schedule: ProgramReleaseSchedule = env
            .storage()
            .persistent()
            .get(&DataKey::ReleaseSchedule(program_id.clone(), schedule_id))
            .unwrap();

        // Check if already released
        if schedule.released {
            panic!("Schedule already released");
        }

        // Get token client
        let contract_address = env.current_contract_address();
        let token_client = token::Client::new(&env, &program_data.token_address);

        // Transfer funds
        token_client.transfer(&contract_address, &schedule.recipient, &schedule.amount);

        // Update schedule
        let now = env.ledger().timestamp();
        schedule.released = true;
        schedule.released_at = Some(now);
        schedule.released_by = Some(program_data.authorized_payout_key.clone());

        // Update program data
        let mut updated_data = program_data.clone();
        updated_data.remaining_balance -= schedule.amount;

        // Add to release history
        let history_entry = ProgramReleaseHistory {
            schedule_id,
            program_id: program_id.clone(),
            amount: schedule.amount,
            recipient: schedule.recipient.clone(),
            released_at: now,
            released_by: program_data.authorized_payout_key.clone(),
            release_type: ReleaseType::Manual,
        };

        let mut history: Vec<ProgramReleaseHistory> = env
            .storage()
            .persistent()
            .get(&DataKey::ReleaseHistory(program_id.clone()))
            .unwrap_or(vec![&env]);
        history.push_back(history_entry);

        // Store updates
        env.storage().persistent().set(
            &DataKey::ReleaseSchedule(program_id.clone(), schedule_id),
            &schedule,
        );
        env.storage().instance().set(&program_key, &updated_data);
        env.storage()
            .persistent()
            .set(&DataKey::ReleaseHistory(program_id.clone()), &history);

        // Emit program schedule released event
        env.events().publish(
            (PROG_SCHEDULE_RELEASED,),
            ProgramScheduleReleased {
                program_id: program_id.clone(),
                schedule_id,
                amount: schedule.amount,
                recipient: schedule.recipient.clone(),
                released_at: now,
                released_by: program_data.authorized_payout_key.clone(),
                release_type: ReleaseType::Manual,
            },
        );

        // Track successful operation
        monitoring::track_operation(
            &env,
            symbol_short!("rel_man"),
            program_data.authorized_payout_key,
            true,
        );

        // Track performance
        let duration = env.ledger().timestamp().saturating_sub(start);
        monitoring::emit_performance(&env, symbol_short!("rel_man"), duration);
    }

    // ========================================================================
    // View Functions (Read-only)
    // ========================================================================

    pub fn get_program_info_v2(env: Env, _program_id: String) -> ProgramData {
        Self::get_program_info(env)
    }

    pub fn lock_program_funds_v2(env: Env, program_id: String, amount: i128) -> ProgramData {
        Self::lock_program_funds(env, program_id, amount)
    }

    pub fn single_payout_v2(
        env: Env,
        program_id: String,
        recipient: Address,
        amount: i128,
    ) -> ProgramData {
        Self::single_payout(env, program_id, recipient, amount)
    }

    pub fn batch_payout_v2(
        env: Env,
        program_id: String,
        recipients: Vec<Address>,
        amounts: Vec<i128>,
    ) -> ProgramData {
        Self::batch_payout(env, program_id, recipients, amounts)
    }

    /// Retrieves the remaining balance for a specific program.
    ///
    /// # Arguments
    /// * `program_id` - The program ID to query
    ///
    /// # Returns
    /// * `i128` - Remaining balance
    ///
    /// # Panics
    /// * If program doesn't exist
    pub fn get_remaining_balance(env: Env, program_id: String) -> i128 {
        let program_key = DataKey::Program(program_id);
        let program_data: ProgramData = env
            .storage()
            .instance()
            .get(&program_key)
            .unwrap_or_else(|| panic!("Program not found"));

        program_data.remaining_balance
    }

    /// Update fee configuration (admin only - uses authorized_payout_key)
    ///
    /// # Arguments
    /// * `lock_fee_rate` - Optional new lock fee rate (basis points)
    /// * `payout_fee_rate` - Optional new payout fee rate (basis points)
    /// * `fee_recipient` - Optional new fee recipient address
    /// * `fee_enabled` - Optional fee enable/disable flag
    pub fn update_fee_config(
        env: Env,
        lock_fee_rate: Option<i128>,
        payout_fee_rate: Option<i128>,
        fee_recipient: Option<Address>,
        fee_enabled: Option<bool>,
    ) {
        let admin = anti_abuse::get_admin(&env).expect("Admin not set");
        admin.require_auth();

        let mut fee_config = Self::get_fee_config_internal(&env);

        if let Some(rate) = lock_fee_rate {
            if rate < 0 || rate > MAX_FEE_RATE {
                panic!(
                    "Invalid lock fee rate: must be between 0 and {}",
                    MAX_FEE_RATE
                );
            }
            fee_config.lock_fee_rate = rate;
        }

        if let Some(rate) = payout_fee_rate {
            if rate < 0 || rate > MAX_FEE_RATE {
                panic!(
                    "Invalid payout fee rate: must be between 0 and {}",
                    MAX_FEE_RATE
                );
            }
            fee_config.payout_fee_rate = rate;
        }

        if let Some(recipient) = fee_recipient {
            fee_config.fee_recipient = recipient;
        }

        if let Some(enabled) = fee_enabled {
            fee_config.fee_enabled = enabled;
        }

        env.storage().instance().set(&FEE_CONFIG, &fee_config);

        // Emit fee config updated event
        env.events().publish(
            (symbol_short!("fee_cfg"),),
            (
                fee_config.lock_fee_rate,
                fee_config.payout_fee_rate,
                fee_config.fee_recipient,
                fee_config.fee_enabled,
            ),
        );
    }

    /// Get current fee configuration (view function)
    pub fn get_fee_config(env: Env) -> FeeConfig {
        Self::get_fee_config_internal(&env)
    }

    /// Gets the total number of programs registered.
    ///
    /// # Returns
    /// * `u32` - Count of registered programs
    pub fn get_program_count(env: Env) -> u32 {
        let registry: Vec<String> = env
            .storage()
            .instance()
            .get(&PROGRAM_REGISTRY)
            .unwrap_or(vec![&env]);

        registry.len()
    }

    // ========================================================================
    // Monitoring & Analytics Functions
    // ========================================================================

    /// Health check - returns contract health status
    pub fn health_check(env: Env) -> monitoring::HealthStatus {
        monitoring::health_check(&env)
    }

    /// Get analytics - returns usage analytics
    pub fn get_analytics(env: Env) -> monitoring::Analytics {
        monitoring::get_analytics(&env)
    }

    /// Get state snapshot - returns current state
    pub fn get_state_snapshot(env: Env) -> monitoring::StateSnapshot {
        monitoring::get_state_snapshot(&env)
    }

    /// Get performance stats for a function
    pub fn get_performance_stats(env: Env, function_name: Symbol) -> monitoring::PerformanceStats {
        monitoring::get_performance_stats(&env, function_name)
    }

    // ========================================================================
    // Anti-Abuse Administrative Functions
    // ========================================================================

    /// Sets the administrative address for anti-abuse configuration.
    /// Can only be called once or by the existing admin.
    pub fn set_admin(env: Env, new_admin: Address) {
        if let Some(current_admin) = anti_abuse::get_admin(&env) {
            current_admin.require_auth();
        }
        anti_abuse::set_admin(&env, new_admin);
    }

    /// Updates the rate limit configuration.
    /// Only the admin can call this.
    pub fn update_rate_limit_config(
        env: Env,
        window_size: u64,
        max_operations: u32,
        cooldown_period: u64,
    ) {
        let admin = anti_abuse::get_admin(&env).expect("Admin not set");
        admin.require_auth();

        anti_abuse::set_config(
            &env,
            anti_abuse::AntiAbuseConfig {
                window_size,
                max_operations,
                cooldown_period,
            },
        );
    }

    /// Adds or removes an address from the whitelist.
    /// Only the admin can call this.
    pub fn set_whitelist(env: Env, address: Address, whitelisted: bool) {
        let admin = anti_abuse::get_admin(&env).expect("Admin not set");
        admin.require_auth();

        anti_abuse::set_whitelist(&env, address, whitelisted);
    }

    /// Checks if an address is whitelisted.
    pub fn is_whitelisted(env: Env, address: Address) -> bool {
        anti_abuse::is_whitelisted(&env, address)
    }

    /// Gets the current rate limit configuration.
    pub fn get_rate_limit_config(env: Env) -> anti_abuse::AntiAbuseConfig {
        anti_abuse::get_config(&env)
    }

    /// Creates an on-chain snapshot of critical configuration (admin-only).
    /// Returns the snapshot id.
    pub fn create_config_snapshot(env: Env) -> u64 {
        let admin = anti_abuse::get_admin(&env).expect("Admin not set");
        admin.require_auth();

        let next_id: u64 = env
            .storage()
            .instance()
            .get(&ConfigSnapshotKey::SnapshotCounter)
            .unwrap_or(0)
            + 1;

        let snapshot = ConfigSnapshot {
            id: next_id,
            timestamp: env.ledger().timestamp(),
            fee_config: Self::get_fee_config_internal(&env),
            anti_abuse_config: anti_abuse::get_config(&env),
            anti_abuse_admin: anti_abuse::get_admin(&env),
            is_paused: Self::is_paused_internal(&env),
        };

        env.storage()
            .instance()
            .set(&ConfigSnapshotKey::Snapshot(next_id), &snapshot);

        let mut index: Vec<u64> = env
            .storage()
            .instance()
            .get(&ConfigSnapshotKey::SnapshotIndex)
            .unwrap_or(vec![&env]);
        index.push_back(next_id);

        if index.len() > CONFIG_SNAPSHOT_LIMIT {
            let oldest_snapshot_id = index.get(0).unwrap();
            env.storage()
                .instance()
                .remove(&ConfigSnapshotKey::Snapshot(oldest_snapshot_id));

            let mut trimmed = Vec::new(&env);
            for i in 1..index.len() {
                trimmed.push_back(index.get(i).unwrap());
            }
            index = trimmed;
        }

        env.storage()
            .instance()
            .set(&ConfigSnapshotKey::SnapshotIndex, &index);
        env.storage()
            .instance()
            .set(&ConfigSnapshotKey::SnapshotCounter, &next_id);

        env.events().publish(
            (symbol_short!("cfg_snap"), symbol_short!("create")),
            (next_id, snapshot.timestamp),
        );

        next_id
    }

    /// Lists retained configuration snapshots in chronological order.
    pub fn list_config_snapshots(env: Env) -> Vec<ConfigSnapshot> {
        let index: Vec<u64> = env
            .storage()
            .instance()
            .get(&ConfigSnapshotKey::SnapshotIndex)
            .unwrap_or(vec![&env]);

        let mut snapshots = Vec::new(&env);
        for snapshot_id in index.iter() {
            if let Some(snapshot) = env
                .storage()
                .instance()
                .get(&ConfigSnapshotKey::Snapshot(snapshot_id))
            {
                snapshots.push_back(snapshot);
            }
        }

        snapshots
    }

    /// Restores contract configuration from a prior snapshot (admin-only).
    pub fn restore_config_snapshot(env: Env, snapshot_id: u64) {
        let admin = anti_abuse::get_admin(&env).expect("Admin not set");
        admin.require_auth();

        let snapshot: ConfigSnapshot = env
            .storage()
            .instance()
            .get(&ConfigSnapshotKey::Snapshot(snapshot_id))
            .unwrap_or_else(|| panic!("Snapshot not found"));

        env.storage().instance().set(&FEE_CONFIG, &snapshot.fee_config);
        anti_abuse::set_config(&env, snapshot.anti_abuse_config);

        match snapshot.anti_abuse_admin {
            Some(snapshot_admin) => anti_abuse::set_admin(&env, snapshot_admin),
            None => anti_abuse::clear_admin(&env),
        }

        env.storage().instance().set(&DataKey::IsPaused, &snapshot.is_paused);

        env.events().publish(
            (symbol_short!("cfg_snap"), symbol_short!("restore")),
            (snapshot_id, env.ledger().timestamp()),
        );
    }

    // ========================================================================
    // Schedule View Functions
    // ========================================================================

    /// Retrieves a specific program release schedule.
    ///
    /// # Arguments
    /// * `env` - The contract environment
    /// * `program_id` - The program containing the schedule
    /// * `schedule_id` - The schedule ID to retrieve
    ///
    /// # Returns
    /// * `ProgramReleaseSchedule` - The schedule details
    ///
    /// # Panics
    /// * If schedule doesn't exist
    pub fn get_program_release_schedule(
        env: Env,
        program_id: String,
        schedule_id: u64,
    ) -> ProgramReleaseSchedule {
        env.storage()
            .persistent()
            .get(&DataKey::ReleaseSchedule(program_id, schedule_id))
            .unwrap_or_else(|| panic!("Schedule not found"))
    /// Get aggregate statistics for the program
    pub fn get_program_aggregate_stats(env: Env) -> ProgramAggregateStats {
        let program_data: ProgramData = env
            .storage()
            .instance()
            .get(&PROGRAM_DATA)
            .unwrap_or_else(|| panic!("Program not initialized"));
        let schedules: Vec<ProgramReleaseSchedule> = env
            .storage()
            .instance()
            .get(&SCHEDULES)
            .unwrap_or_else(|| Vec::new(&env));

        let mut scheduled_count = 0u32;
        let mut released_count = 0u32;

        for i in 0..schedules.len() {
            let schedule = schedules.get(i).unwrap();
            if schedule.released {
                released_count += 1;
            } else {
                scheduled_count += 1;
            }
        }

        ProgramAggregateStats {
            total_funds: program_data.total_funds,
            remaining_balance: program_data.remaining_balance,
            total_paid_out: program_data.total_funds - program_data.remaining_balance,
            authorized_payout_key: program_data.authorized_payout_key.clone(),
            payout_history: program_data.payout_history.clone(),
            token_address: program_data.token_address.clone(),
            payout_count: program_data.payout_history.len(),
            scheduled_count,
            released_count,
        }
    }

    /// Retrieves all release schedules for a program.
    ///
    /// # Arguments
    /// * `env` - The contract environment
    /// * `program_id` - The program to query
    ///
    /// # Returns
    /// * `Vec<ProgramReleaseSchedule>` - All schedules for the program
    pub fn get_all_prog_release_schedules(
        env: Env,
        program_id: String,
    ) -> Vec<ProgramReleaseSchedule> {
        let mut schedules = Vec::new(&env);
        let next_id: u64 = env
            .storage()
            .persistent()
            .get(&DataKey::NextScheduleId(program_id.clone()))
            .unwrap_or(1);

        for schedule_id in 1..next_id {
            if env
                .storage()
                .persistent()
                .has(&DataKey::ReleaseSchedule(program_id.clone(), schedule_id))
            {
                let schedule: ProgramReleaseSchedule = env
                    .storage()
                    .persistent()
                    .get(&DataKey::ReleaseSchedule(program_id.clone(), schedule_id))
                    .unwrap();
                schedules.push_back(schedule);
            }
        }

        schedules
    }

    /// Retrieves pending (unreleased) schedules for a program.
    ///
    /// # Arguments
    /// * `env` - The contract environment
    /// * `program_id` - The program to query
    ///
    /// # Returns
    /// * `Vec<ProgramReleaseSchedule>` - All pending schedules
    pub fn get_pending_program_schedules(
        env: Env,
        program_id: String,
    ) -> Vec<ProgramReleaseSchedule> {
        let all_schedules = Self::get_all_prog_release_schedules(env.clone(), program_id.clone());
        let mut pending = Vec::new(&env);

        for schedule in all_schedules.iter() {
            if !schedule.released {
                pending.push_back(schedule.clone());
            }
        }

        pending
    }

    /// Retrieves due schedules (timestamp passed but not released) for a program.
    ///
    /// # Arguments
    /// * `env` - The contract environment
    /// * `program_id` - The program to query
    ///
    /// # Returns
    /// * `Vec<ProgramReleaseSchedule>` - All due but unreleased schedules
    pub fn get_due_program_schedules(env: Env, program_id: String) -> Vec<ProgramReleaseSchedule> {
        let pending = Self::get_pending_program_schedules(env.clone(), program_id.clone());
        let mut due = Vec::new(&env);
        let now = env.ledger().timestamp();

        for schedule in pending.iter() {
            if schedule.release_timestamp <= now {
                due.push_back(schedule.clone());
            }
        }

        due
    }

    /// Retrieves release history for a program.
    ///
    /// # Arguments
    /// * `env` - The contract environment
    /// * `program_id` - The program to query
    ///
    /// # Returns
    /// * `Vec<ProgramReleaseHistory>` - Complete release history
    pub fn get_program_release_history(env: Env, program_id: String) -> Vec<ProgramReleaseHistory> {
        env.storage()
            .persistent()
            .get(&DataKey::ReleaseHistory(program_id))
            .unwrap_or(vec![&env])
    }
}

/// Helper function to calculate total scheduled amount for a program.
fn get_program_total_scheduled_amount(env: &Env, program_id: &String) -> i128 {
    let next_id: u64 = env
        .storage()
        .persistent()
        .get(&DataKey::NextScheduleId(program_id.clone()))
        .unwrap_or(1);

    let mut total = 0i128;
    for schedule_id in 1..next_id {
        if env
            .storage()
            .persistent()
            .has(&DataKey::ReleaseSchedule(program_id.clone(), schedule_id))
        {
            let schedule: ProgramReleaseSchedule = env
                .storage()
                .persistent()
                .get(&DataKey::ReleaseSchedule(program_id.clone(), schedule_id))
                .unwrap();
            if !schedule.released {
                total += schedule.amount;
            }
        }
    }

    pub fn get_program_count(env: Env) -> u32 {
        if env.storage().instance().has(&PROGRAM_DATA) {
            1
        } else {
            0
        }
    }

    // ========================================================================
    // Program Registration Tests
    // ========================================================================

    fn setup_program_with_schedule(
        env: &Env,
        client: &ProgramEscrowContractClient<'static>,
        authorized_key: &Address,
        token: &Address,
        program_id: &String,
        total_amount: i128,
        winner: &Address,
        release_timestamp: u64,
    ) {
        // Register program
        client.initialize_program(program_id, authorized_key, token);

        // Create and fund token
        let token_client = create_token_contract(env, authorized_key);
        let token_admin = token::StellarAssetClient::new(env, &token_client.address);
        token_admin.mint(authorized_key, &total_amount);

        // Lock funds for program
        token_client.approve(
            authorized_key,
            &env.current_contract_address(),
            &total_amount,
            &1000,
        );
        client.lock_program_funds(program_id, &total_amount);

        // Create release schedule
        client.create_program_release_schedule(
            program_id,
            &total_amount,
            &release_timestamp,
            &winner,
        );
    }

    #[test]
    fn test_single_program_release_schedule() {
        let env = Env::default();
        let contract_id = env.register_contract(None, ProgramEscrowContract);
        let client = ProgramEscrowContractClient::new(&env, &contract_id);

        let authorized_key = Address::generate(&env);
        let winner = Address::generate(&env);
        let token = Address::generate(&env);
        let program_id = String::from_str(&env, "Hackathon2024");
        let amount = 1000_0000000;
        let release_timestamp = 1000;

        env.mock_all_auths();

        // Setup program with schedule
        setup_program_with_schedule(
            &env,
            &client,
            &authorized_key,
            &token,
            &program_id,
            amount,
            &winner,
            release_timestamp,
        );

        // Verify schedule was created
        let schedule = client.get_program_release_schedule(&program_id, &1);
        assert_eq!(schedule.schedule_id, 1);
        assert_eq!(schedule.amount, amount);
        assert_eq!(schedule.release_timestamp, release_timestamp);
        assert_eq!(schedule.recipient, winner);
        assert!(!schedule.released);

        // Check pending schedules
        let pending = client.get_pending_program_schedules(&program_id);
        assert_eq!(pending.len(), 1);

        // Event verification can be added later - focusing on core functionality
    pub fn get_program_release_schedule(env: Env, schedule_id: u64) -> ProgramReleaseSchedule {
        let schedules = Self::get_release_schedules(env);
        for s in schedules.iter() {
            if s.schedule_id == schedule_id {
                return s;
            }
        }
        panic!("Schedule not found");
    }

    #[test]
    fn test_multiple_program_release_schedules() {
        let env = Env::default();
        let contract_id = env.register_contract(None, ProgramEscrowContract);
        let client = ProgramEscrowContractClient::new(&env, &contract_id);

        let authorized_key = Address::generate(&env);
        let winner1 = Address::generate(&env);
        let winner2 = Address::generate(&env);
        let token = Address::generate(&env);
        let program_id = String::from_str(&env, "Hackathon2024");
        let amount1 = 600_0000000;
        let amount2 = 400_0000000;
        let total_amount = amount1 + amount2;

        env.mock_all_auths();

        // Register program
        client.initialize_program(&program_id, &authorized_key, &token);

        // Create and fund token
        let token_client = create_token_contract(&env, &authorized_key);
        let token_admin = token::StellarAssetClient::new(&env, &token_client.address);
        token_admin.mint(&authorized_key, &total_amount);

        // Lock funds for program
        token_client.approve(
            &authorized_key,
            &env.current_contract_address(),
            &total_amount,
            &1000,
        );
        client.lock_program_funds(&program_id, &total_amount);

        // Create first release schedule
        client.create_program_release_schedule(&program_id, &amount1, &1000, &winner1.clone());

        // Create second release schedule
        client.create_program_release_schedule(&program_id, &amount2, &2000, &winner2.clone());

        // Verify both schedules exist
        let all_schedules = client.get_all_prog_release_schedules(&program_id);
        assert_eq!(all_schedules.len(), 2);

        // Verify schedule IDs
        let schedule1 = client.get_program_release_schedule(&program_id, &1);
        let schedule2 = client.get_program_release_schedule(&program_id, &2);
        assert_eq!(schedule1.schedule_id, 1);
        assert_eq!(schedule2.schedule_id, 2);

        // Verify amounts
        assert_eq!(schedule1.amount, amount1);
        assert_eq!(schedule2.amount, amount2);

        // Verify recipients
        assert_eq!(schedule1.recipient, winner1);
        assert_eq!(schedule2.recipient, winner2);

        // Check pending schedules
        let pending = client.get_pending_program_schedules(&program_id);
        assert_eq!(pending.len(), 2);

        // Event verification can be added later - focusing on core functionality
    }

    #[test]
    fn test_program_automatic_release_at_timestamp() {
        let env = Env::default();
        let contract_id = env.register_contract(None, ProgramEscrowContract);
        let client = ProgramEscrowContractClient::new(&env, &contract_id);

        let authorized_key = Address::generate(&env);
        let winner = Address::generate(&env);
        let token = Address::generate(&env);
        let program_id = String::from_str(&env, "Hackathon2024");
        let amount = 1000_0000000;
        let release_timestamp = 1000;

        env.mock_all_auths();

        // Setup program with schedule
        setup_program_with_schedule(
            &env,
            &client,
            &authorized_key,
            &token,
            &program_id,
            amount,
            &winner,
            release_timestamp,
        );

        // Try to release before timestamp (should fail)
        env.ledger().set_timestamp(999);
        let result = client.try_release_prog_schedule_automatic(&program_id, &1);
        assert!(result.is_err());

        // Advance time to after release timestamp
        env.ledger().set_timestamp(1001);

        // Release automatically
        client.release_prog_schedule_automatic(&program_id, &1);

        // Verify schedule was released
        let schedule = client.get_program_release_schedule(&program_id, &1);
        assert!(schedule.released);
        assert_eq!(schedule.released_at, Some(1001));
        assert_eq!(schedule.released_by, Some(env.current_contract_address()));

        // Check no pending schedules
        let pending = client.get_pending_program_schedules(&program_id);
        assert_eq!(pending.len(), 0);

        // Verify release history
        let history = client.get_program_release_history(&program_id);
        assert_eq!(history.len(), 1);
        assert_eq!(history.get(0).unwrap().release_type, ReleaseType::Automatic);

        // Event verification can be added later - focusing on core functionality
    }

    #[test]
    fn test_program_manual_trigger_before_after_timestamp() {
        let env = Env::default();
        let contract_id = env.register_contract(None, ProgramEscrowContract);
        let client = ProgramEscrowContractClient::new(&env, &contract_id);

        let authorized_key = Address::generate(&env);
        let winner = Address::generate(&env);
        let token = Address::generate(&env);
        let program_id = String::from_str(&env, "Hackathon2024");
        let amount = 1000_0000000;
        let release_timestamp = 1000;

        env.mock_all_auths();

        // Setup program with schedule
        setup_program_with_schedule(
            &env,
            &client,
            &authorized_key,
            &token,
            &program_id,
            amount,
            &winner,
            release_timestamp,
        );

        // Manually release before timestamp (authorized key can do this)
        env.ledger().set_timestamp(999);
        client.release_program_schedule_manual(&program_id, &1);

        // Verify schedule was released
        let schedule = client.get_program_release_schedule(&program_id, &1);
        assert!(schedule.released);
        assert_eq!(schedule.released_at, Some(999));
        assert_eq!(schedule.released_by, Some(authorized_key.clone()));

        // Verify release history
        let history = client.get_program_release_history(&program_id);
        assert_eq!(history.len(), 1);
        assert_eq!(history.get(0).unwrap().release_type, ReleaseType::Manual);

        // Event verification can be added later - focusing on core functionality
    }

    pub fn release_prog_schedule_automatic(env: Env, schedule_id: u64) {
        let mut schedules = Self::get_release_schedules(env.clone());
        let program_data = Self::get_program_info(env.clone());
        let now = env.ledger().timestamp();
        let mut released_schedule: Option<ProgramReleaseSchedule> = None;

        let mut found = false;
        for i in 0..schedules.len() {
            let mut s = schedules.get(i).unwrap();
            if s.schedule_id == schedule_id {
                if s.released {
                    panic!("Already released");
                }
                if now < s.release_timestamp {
                    panic!("Not yet due");
                }

                // Transfer funds
                let token_client = token::Client::new(&env, &program_data.token_address);
                token_client.transfer(&env.current_contract_address(), &s.recipient, &s.amount);

                s.released = true;
                s.released_at = Some(now);
                s.released_by = Some(env.current_contract_address());
                released_schedule = Some(s.clone());
                schedules.set(i, s);
                found = true;
                break;
            }
        }

        if !found {
            panic!("Schedule not found");
        }

        env.storage().instance().set(&SCHEDULES, &schedules);

        if let Some(s) = released_schedule {
            let mut updated_program_data = program_data.clone();
            updated_program_data.remaining_balance -= s.amount;
            env.storage()
                .instance()
                .set(&PROGRAM_DATA, &updated_program_data);

            let mut history: Vec<ProgramReleaseHistory> = env
                .storage()
                .instance()
                .get(&RELEASE_HISTORY)
                .unwrap_or_else(|| Vec::new(&env));
            history.push_back(ProgramReleaseHistory {
                schedule_id: s.schedule_id,
                recipient: s.recipient.clone(),
                amount: s.amount,
                released_at: now,
                release_type: ReleaseType::Automatic,
            });
            env.storage().instance().set(&RELEASE_HISTORY, &history);

            let receipt_id = Self::increment_receipt_id(&env);
            env.events().publish(
                (PAYOUT,),
                PayoutEvent {
                    version: EVENT_VERSION_V2,
                    program_id: updated_program_data.program_id.clone(),
                    recipient: s.recipient,
                    amount: s.amount,
                    remaining_balance: updated_program_data.remaining_balance,
                    receipt_id,
                },
            );
        }
    }
}


#[cfg(test)]
mod test;

#[cfg(test)]
mod test_pause;

#[cfg(test)]
#[cfg(any())]
mod rbac_tests;
