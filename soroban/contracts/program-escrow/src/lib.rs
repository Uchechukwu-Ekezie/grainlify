#![no_std]
use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, symbol_short, token, Address, Env, String,
    Vec,
};

const MAX_BATCH_SIZE: u32 = 20;
const PROGRAM_REGISTERED: soroban_sdk::Symbol = symbol_short!("prg_reg");

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    AlreadyInitialized = 1,
    NotInitialized = 2,
    ProgramExists = 3,
    ProgramNotFound = 4,
    Unauthorized = 5,
    InvalidBatchSize = 6,
    DuplicateProgramId = 7,
    InvalidAmount = 8,
    InvalidName = 9,
    JurisdictionKycRequired = 10,
    JurisdictionFundingLimitExceeded = 11,
    JurisdictionPaused = 12,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProgramStatus {
    Active,
    Completed,
    Cancelled,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Program {
    pub admin: Address,
    pub name: String,
    pub total_funding: i128,
    pub status: ProgramStatus,
    pub jurisdiction: Option<ProgramJurisdictionConfig>,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProgramJurisdictionConfig {
    pub tag: Option<String>,
    pub requires_kyc: bool,
    pub max_funding: Option<i128>,
    pub registration_paused: bool,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProgramRegistrationItem {
    pub program_id: u64,
    pub admin: Address,
    pub name: String,
    pub total_funding: i128,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProgramRegistrationWithJurisdictionItem {
    pub program_id: u64,
    pub admin: Address,
    pub name: String,
    pub total_funding: i128,
    pub jurisdiction: Option<ProgramJurisdictionConfig>,
    pub kyc_attested: Option<bool>,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProgramRegisteredEvent {
    pub version: u32,
    pub program_id: u64,
    pub admin: Address,
    pub total_funding: i128,
    pub jurisdiction_tag: Option<String>,
    pub requires_kyc: bool,
    pub max_funding: Option<i128>,
    pub registration_paused: bool,
    pub timestamp: u64,
}

#[contracttype]
pub enum DataKey {
    Admin,
    Token,
    Program(u64),
}

#[contract]
pub struct ProgramEscrowContract;

#[contractimpl]
impl ProgramEscrowContract {
    fn validate_program_input(name: &String, total_funding: i128) -> Result<(), Error> {
        if total_funding <= 0 {
            return Err(Error::InvalidAmount);
        }
        if name.len() == 0 {
            return Err(Error::InvalidName);
        }
        Ok(())
    }

    fn enforce_jurisdiction_rules(
        jurisdiction: &Option<ProgramJurisdictionConfig>,
        total_funding: i128,
        kyc_attested: Option<bool>,
    ) -> Result<(), Error> {
        if let Some(config) = jurisdiction {
            if config.registration_paused {
                return Err(Error::JurisdictionPaused);
            }

            if let Some(max_funding) = config.max_funding {
                if total_funding > max_funding {
                    return Err(Error::JurisdictionFundingLimitExceeded);
                }
            }

            if config.requires_kyc && !kyc_attested.unwrap_or(false) {
                return Err(Error::JurisdictionKycRequired);
            }
        }
        Ok(())
    }

    fn emit_program_registered(
        env: &Env,
        program_id: u64,
        admin: Address,
        total_funding: i128,
        jurisdiction: &Option<ProgramJurisdictionConfig>,
    ) {
        let (jurisdiction_tag, requires_kyc, max_funding, registration_paused) =
            if let Some(config) = jurisdiction {
                (
                    config.tag.clone(),
                    config.requires_kyc,
                    config.max_funding,
                    config.registration_paused,
                )
            } else {
                (None, false, None, false)
            };

        env.events().publish(
            (PROGRAM_REGISTERED, program_id),
            ProgramRegisteredEvent {
                version: 2,
                program_id,
                admin,
                total_funding,
                jurisdiction_tag,
                requires_kyc,
                max_funding,
                registration_paused,
                timestamp: env.ledger().timestamp(),
            },
        );
    }

    /// Initialize the contract with an admin and token address. Call once.
    pub fn init(env: Env, admin: Address, token: Address) -> Result<(), Error> {
        if env.storage().instance().has(&DataKey::Admin) {
            return Err(Error::AlreadyInitialized);
        }
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::Token, &token);
        Ok(())
    }

    /// Register a single program.
    pub fn register_program(
        env: Env,
        program_id: u64,
        admin: Address,
        name: String,
        total_funding: i128,
    ) -> Result<(), Error> {
        Self::register_program_with_jurisdiction(
            env,
            program_id,
            admin,
            name,
            total_funding,
            None,
            None,
        )
    }

    /// Register a single program with optional jurisdiction controls.
    pub fn register_program_with_jurisdiction(
        env: Env,
        program_id: u64,
        admin: Address,
        name: String,
        total_funding: i128,
        jurisdiction: Option<ProgramJurisdictionConfig>,
        kyc_attested: Option<bool>,
    ) -> Result<(), Error> {
        if !env.storage().instance().has(&DataKey::Admin) {
            return Err(Error::NotInitialized);
        }
        let contract_admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        contract_admin.require_auth();

        if env
            .storage()
            .persistent()
            .has(&DataKey::Program(program_id))
        {
            return Err(Error::ProgramExists);
        }

        Self::validate_program_input(&name, total_funding)?;
        Self::enforce_jurisdiction_rules(&jurisdiction, total_funding, kyc_attested)?;

        // Transfer funding from the program admin to the contract
        let token_addr: Address = env.storage().instance().get(&DataKey::Token).unwrap();
        let token_client = token::Client::new(&env, &token_addr);
        admin.require_auth();
        token_client.transfer(&admin, &env.current_contract_address(), &total_funding);

        let program = Program {
            admin: admin.clone(),
            name,
            total_funding,
            status: ProgramStatus::Active,
            jurisdiction: jurisdiction.clone(),
        };
        env.storage()
            .persistent()
            .set(&DataKey::Program(program_id), &program);

        Self::emit_program_registered(&env, program_id, admin, total_funding, &jurisdiction);
        Ok(())
    }

    /// Batch register multiple programs in a single transaction.
    ///
    /// This operation is atomic — if any item fails validation, the entire
    /// batch is rejected and no programs are registered.
    ///
    /// # Errors
    /// * `InvalidBatchSize` — batch is empty or exceeds `MAX_BATCH_SIZE`
    /// * `ProgramExists` — a program_id already exists in storage
    /// * `DuplicateProgramId` — duplicate program_ids within the batch
    /// * `InvalidAmount` — zero or negative funding amount
    /// * `InvalidName` — empty program name
    /// * `NotInitialized` — contract has not been initialized
    pub fn batch_register_programs(
        env: Env,
        items: Vec<ProgramRegistrationItem>,
    ) -> Result<u32, Error> {
        let batch_size = items.len() as u32;
        if batch_size == 0 || batch_size > MAX_BATCH_SIZE {
            return Err(Error::InvalidBatchSize);
        }

        if !env.storage().instance().has(&DataKey::Admin) {
            return Err(Error::NotInitialized);
        }
        let contract_admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        contract_admin.require_auth();

        let token_addr: Address = env.storage().instance().get(&DataKey::Token).unwrap();
        let token_client = token::Client::new(&env, &token_addr);
        let contract_address = env.current_contract_address();

        // --- Validation pass (all-or-nothing) ---
        for item in items.iter() {
            if env
                .storage()
                .persistent()
                .has(&DataKey::Program(item.program_id))
            {
                return Err(Error::ProgramExists);
            }
            Self::validate_program_input(&item.name, item.total_funding)?;

            // Detect duplicate program_ids within the batch
            let mut count = 0u32;
            for other in items.iter() {
                if other.program_id == item.program_id {
                    count += 1;
                }
            }
            if count > 1 {
                return Err(Error::DuplicateProgramId);
            }
        }

        // Collect unique admins and require auth once per admin
        let mut seen_admins: Vec<Address> = Vec::new(&env);
        for item in items.iter() {
            let mut found = false;
            for seen in seen_admins.iter() {
                if seen == item.admin {
                    found = true;
                    break;
                }
            }
            if !found {
                seen_admins.push_back(item.admin.clone());
                item.admin.require_auth();
            }
        }

        // --- Processing pass (atomic) ---
        let mut registered_count = 0u32;
        for item in items.iter() {
            token_client.transfer(&item.admin, &contract_address, &item.total_funding);

            let program = Program {
                admin: item.admin.clone(),
                name: item.name.clone(),
                total_funding: item.total_funding,
                status: ProgramStatus::Active,
                jurisdiction: None,
            };
            env.storage()
                .persistent()
                .set(&DataKey::Program(item.program_id), &program);

            Self::emit_program_registered(
                &env,
                item.program_id,
                item.admin.clone(),
                item.total_funding,
                &None,
            );
            registered_count += 1;
        }

        Ok(registered_count)
    }

    /// Batch register programs with optional jurisdiction controls.
    pub fn batch_register_programs_with_jurisdiction(
        env: Env,
        items: Vec<ProgramRegistrationWithJurisdictionItem>,
    ) -> Result<u32, Error> {
        let batch_size = items.len() as u32;
        if batch_size == 0 || batch_size > MAX_BATCH_SIZE {
            return Err(Error::InvalidBatchSize);
        }

        if !env.storage().instance().has(&DataKey::Admin) {
            return Err(Error::NotInitialized);
        }
        let contract_admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        contract_admin.require_auth();

        let token_addr: Address = env.storage().instance().get(&DataKey::Token).unwrap();
        let token_client = token::Client::new(&env, &token_addr);
        let contract_address = env.current_contract_address();

        for item in items.iter() {
            if env
                .storage()
                .persistent()
                .has(&DataKey::Program(item.program_id))
            {
                return Err(Error::ProgramExists);
            }
            Self::validate_program_input(&item.name, item.total_funding)?;
            Self::enforce_jurisdiction_rules(
                &item.jurisdiction,
                item.total_funding,
                item.kyc_attested,
            )?;

            let mut count = 0u32;
            for other in items.iter() {
                if other.program_id == item.program_id {
                    count += 1;
                }
            }
            if count > 1 {
                return Err(Error::DuplicateProgramId);
            }
        }

        let mut seen_admins: Vec<Address> = Vec::new(&env);
        for item in items.iter() {
            let mut found = false;
            for seen in seen_admins.iter() {
                if seen == item.admin {
                    found = true;
                    break;
                }
            }
            if !found {
                seen_admins.push_back(item.admin.clone());
                item.admin.require_auth();
            }
        }

        let mut registered_count = 0u32;
        for item in items.iter() {
            token_client.transfer(&item.admin, &contract_address, &item.total_funding);

            let program = Program {
                admin: item.admin.clone(),
                name: item.name.clone(),
                total_funding: item.total_funding,
                status: ProgramStatus::Active,
                jurisdiction: item.jurisdiction.clone(),
            };
            env.storage()
                .persistent()
                .set(&DataKey::Program(item.program_id), &program);

            Self::emit_program_registered(
                &env,
                item.program_id,
                item.admin.clone(),
                item.total_funding,
                &item.jurisdiction,
            );

            registered_count += 1;
        }

        Ok(registered_count)
    }

    /// Read a program's state.
    pub fn get_program(env: Env, program_id: u64) -> Result<Program, Error> {
        env.storage()
            .persistent()
            .get(&DataKey::Program(program_id))
            .ok_or(Error::ProgramNotFound)
    }

    /// Read jurisdiction configuration for a program.
    pub fn get_program_jurisdiction(
        env: Env,
        program_id: u64,
    ) -> Result<Option<ProgramJurisdictionConfig>, Error> {
        let program = Self::get_program(env, program_id)?;
        Ok(program.jurisdiction)
    }
}

mod test;
