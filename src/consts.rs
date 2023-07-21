use near_sdk::env::STORAGE_PRICE_PER_BYTE;
use near_sdk::{Balance, Gas, StorageUsage};

pub(crate) const U64_STORAGE: StorageUsage = 8;
pub(crate) const U8_STORAGE: StorageUsage = 1;

/// Every contract storage key/value entry always uses 40 bytes when stored via `env::storage_write`
/// - key len as u64,
/// - key ptr as u64,
/// - value len as u64,
/// - value ptr as u64,
/// - register as u64
pub(crate) const STORAGE_ENTRY: StorageUsage = 5 * U64_STORAGE;

/// enum::StorageKey size [1 byte]
const ENUM_STORAGE_KEY: StorageUsage = U8_STORAGE;

pub const PROOF_OF_KUDOS_SBT_CLASS_ID: u64 = 1;

pub const PROOF_OF_KUDOS_SBT_MINT_COST: Balance = 6_000_000_000_000_000_000_000;

pub const EXCHANGE_KUDOS_STORAGE: StorageUsage = STORAGE_ENTRY + ENUM_STORAGE_KEY + U64_STORAGE;

/// Deposit required to exchange upvoted Kudos for ProofOfKudos SBT
pub const EXCHANGE_KUDOS_COST: Balance =
    EXCHANGE_KUDOS_STORAGE as Balance * STORAGE_PRICE_PER_BYTE + PROOF_OF_KUDOS_SBT_MINT_COST;

pub const GIVE_KUDOS_COST: Balance = 100_000_000_000_000_000_000_000; // 0.1 NEAR (0.09936)
pub const LEAVE_COMMENT_COST: Balance = 17_000_000_000_000_000_000_000; // 0.017 NEAR (0.01653)
pub const UPVOTE_KUDOS_COST: Balance = 4_000_000_000_000_000_000_000; // 0.004 NEAR (0.00311)

pub const FAILURE_CALLBACK_GAS: Gas = Gas(5 * Gas::ONE_TERA.0);

pub const SAVE_KUDOS_RESERVED_GAS: Gas = Gas(15 * Gas::ONE_TERA.0);
pub const KUDOS_SAVED_CALLBACK_GAS: Gas = Gas(5 * Gas::ONE_TERA.0);
pub const GIVE_KUDOS_RESERVED_GAS: Gas = Gas(15 * Gas::ONE_TERA.0);

pub const VERIFY_KUDOS_RESERVED_GAS: Gas = Gas(15 * Gas::ONE_TERA.0);

pub const ACQUIRE_KUDOS_SENDER_RESERVED_GAS: Gas = Gas(15 * Gas::ONE_TERA.0);
pub const KUDOS_SENDER_ACQUIRED_CALLBACK_GAS: Gas = Gas(15 * Gas::ONE_TERA.0);
pub const KUDOS_UPVOTE_SAVED_CALLBACK_GAS: Gas = Gas(10 * Gas::ONE_TERA.0);
pub const UPVOTE_KUDOS_RESERVED_GAS: Gas = Gas(15 * Gas::ONE_TERA.0);

pub const ACQUIRE_KUDOS_INFO_RESERVED_GAS: Gas = Gas(15 * Gas::ONE_TERA.0);
pub const KUDOS_INFO_ACQUIRED_CALLBACK_GAS: Gas = Gas(15 * Gas::ONE_TERA.0);
pub const KUDOS_COMMENT_SAVED_CALLBACK_GAS: Gas = Gas(5 * Gas::ONE_TERA.0);
pub const LEAVE_COMMENT_RESERVED_GAS: Gas = Gas(15 * Gas::ONE_TERA.0);

pub const ACQUIRE_NUMBER_OF_UPVOTES_RESERVED_GAS: Gas = Gas(15 * Gas::ONE_TERA.0);
pub const KUDOS_UPVOTES_ACQUIRED_CALLBACK_GAS: Gas = Gas(10 * Gas::ONE_TERA.0);
pub const PROOF_OF_KUDOS_SBT_MINT_GAS: Gas = Gas(10 * Gas::ONE_TERA.0);
pub const PROOF_OF_KUDOS_SBT_MINT_CALLBACK_GAS: Gas = Gas(5 * Gas::ONE_TERA.0);
pub const EXCHANGE_KUDOS_FOR_SBT_RESERVED_GAS: Gas = Gas(10 * Gas::ONE_TERA.0);

pub const SOCIAL_DB_REQUEST_MIN_RESERVED_GAS: Gas = Gas(10 * Gas::ONE_TERA.0);
