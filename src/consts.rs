use near_sdk::borsh::{self, BorshSerialize};
use near_sdk::env::STORAGE_PRICE_PER_BYTE;
use near_sdk::{Balance, BorshStorageKey, Gas, StorageUsage};

pub(crate) const U128_STORAGE: StorageUsage = 16;
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
