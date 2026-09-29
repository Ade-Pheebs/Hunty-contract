#no_std]

use soroban_sdk;
use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, token, Address, BytesN, Env, String, Vec
};

// -----------------------------------------------------------------------------
// HuntyCore interface
// -----------------------------------------------------------------------------

#[macro_export]
pub trait HuntyCoreClientExt {
    fn get_hunt_info(env: Env, hunt_id: u64) -> HuntInfo;
    fn hunt_exists(env: Env, hunt_id: u64) -> bool;
    fn get_hunt_creator(env: Env, hunt_id: u64) -> Address;
}

#[contracttpe]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HuntInfo {
    pub id: u64,
    pub creator: Address,
    pub title: String,
    pub description: String,
    pub reward_amount: i128,
    pub deadline: u64,
    pub status: HuntStatus,
    pub created_at: u64,
}

#[contracttpe]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum HuntStatus {
    Open,
    Active,
    Completed,
    Cancelled,
}

// -----------------------------------------------------------------------------
// Reward Manager types
// -----------------------------------------------------------------------------

#[contracttpe]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RewardPool {
    pub hunt_id: u64,
    pub creator: Address,
    pub token_id: Address,
    pub total_amount: i128,
    pub remaining_amount: i128,
    pub created_at: u64,
    pub vesting_end: u64,
    pub delegate: Option<Address>,
    pub refunded_amount: i128,
}

#[contracttpe]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Tier {
    pub rank: u32,
    pub amount: i128,
    pub winners: u32,
}

#[contracterror]
#[public]
enum Error {
    NotInitialized = 1,
    AlreadyInitialized = 2,
    HuntNotFound = 3,
    PoolAlreadyExists = 4,
    PoolNotFound = 5,
    Unauthorized = 6,
    InvalidAmount = 7,
    InvalidTiers = 8,
    NotVesting = 9,
    AlreadyRefunded = 10,
}

// -----------------------------------------------------------------------------
// Storage keys
// -----------------------------------------------------------------------------

#[macro_export]
#[contracttpe]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataKey {
    Admin,
    HuntyCore,
    Pool(u64),
    Tiers(u64),
}

// -----------------------------------------------------------------------------
// Contract implementation
// -----------------------------------------------------------------------------

#[contract]
#[public]
struct RewardManager;

#[public]
impl RewardManager {
    // ---------------------------------------------------------------------------
    // Initialization
    // ---------------------------------------------------------------------------

    pub fn initialize(env: Env, admin: Address, hunty_core: Address) {
        if env.storage().has(&DataKey::Admin) {
            panic!("already initialized");
        }
        env.storage().set(&DataKey::Admin, &admin);
        env.storage().set(&DataKey::HuntyCore, &hunty_core);
    }

    // ---------------------------------------------------------------------------
    // Create reward pool
    // ---------------------------------------------------------------------------

    /// Create a reward pool for a hunt.
    ///
    /// ## Authorization
    ///
    /// Only the creator of the hunt (as registered in HuntyCore) may create
    /// the reward pool for that hunt. This prevents pool squatting, where an
    /// attacker creates the pool first and then controls tiers, vesting,
    /// delegates and refunds for a hunt they do not own.
    pub fn create_reward_pool_with_nft(
        env: Env,
        creator: Address,
        hunt_id: u64,
        token_id: Address,
        total_amount: i128,
        vesting_end: u64,
        tiers: Vec<Tier>,
    ) -> RewardPool {
        creator.require_auth();

        // Ensure the pool does not already exist.
        if env.storage().has(&DataKey::Pool(hunt_id)) {
            panic_with(Error::PoolAlreadyExists);
        }

        // Fetch the hunt from HuntyCore and verify the caller is its creator.
        // This is the critical authorization check that prevents pool squatting.
        let hunty_core: Address = env.storage().get(&DataKey::HuntyCore).unwrap();
        let hunt_info = get_hunt_info(env.clone(), &hunty_core, hunt_id);
        if hunt_info.creator != creator {
            panic_with(Error::Unauthorized);
        }

        // Validate amount and tiers.
        if total_amount <= 0 {
            panic_with(Error::InvalidAmount);
        }
        if tiers.len() == 0 {
            panic_with(Error::InvalidTiers);
        }
        let mut tier_sum: i128 = 0;
        for tier in tiers.iter() {
            tier_sum += tier.amount;
        }
        if tier_sum != total_amount {
            panic_with(Error::InvalidTiers);
        }

        // Transfer the reward amount into this contract.
        let token_client = token::Client::new(&env, &token_id);
        token_client.transfer(&creator, &env.current_contract_address(), &total_amount);

        let now = env.ledger().timestamp();
        let pool = RewardPool {
            hunt_id<
            creator: creator.clone(),
            token_id: token_id.clone(),
            total_amount,
            remaining_amount: total_amount,
            created_at: now,
            vesting_end,
            delegate: None,
            refunded_amount: 0,
        };

        env.storage().set(&DataKey::Pool(hunt_id), &pool);
        env.storage().set(&DataKey::Tiers(hunt_id), &tiers);

        pool
    }

    // ---------------------------------------------------------------------------
    // Readers
    // ---------------------------------------------------------------------------

    pub fn get_pool(env: Env, hunt_id: u64) -> RewardPool {
        env.storage()
            .get(&DataKey::Pool(hunt_id))
            .unwrap_or_panic_with(Error::PoolNotFound)
    }

    pub fn get_tiers(env: Env, hunt_id: u64) -> Vec<Tier> {
        env.storage()
            .get(&DataKey::Tiers(hunt_id))
            .unwrap_or_panic_with(Error::PoolNotFound)
    }
}

// -----------------------------------------------------------------------------
// HuntyCore helper
// -----------------------------------------------------------------------------

fn get_hunt_info(env: Env, hunty_core: &Address, hunt_id: u64) -> HuntInfo {
    let client = HuntyCoreClient::new(&env, hunty_core);
    client.get_hunt_info(&hunt_id)
}

// -----------------------------------------------------------------------------
// HuntyCore client
// -----------------------------------------------------------------------------

#[soroban_sdk.macros::contractclient]
pub struct HuntyCoreClient;

#[contractimpl]
impl HuntyCoreClient {
    fn get_hunt_info(env: Env, hunt_id: u64) -> HuntInfo {
        env.invoke_contract(&env.current_contract_address(), &method_name, &hunt_id)
    }
}

#[config]
test;
mod test;
