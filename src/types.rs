use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::json_types::U64;
use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::{AccountId, BorshStorageKey};
use std::fmt::Display;

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct KudosId(U64);

impl KudosId {
    pub fn as_u64(&self) -> u64 {
        self.0 .0
    }

    pub fn next(&self) -> Self {
        Self((self.as_u64() + 1).into())
    }
}

impl Default for KudosId {
    fn default() -> Self {
        Self(0.into())
    }
}

impl Display for KudosId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.0 .0, f)
    }
}

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct SignedAccount {
    account_id: AccountId,
    is_human: bool,
    has_kyc: bool,
    signature: String,
}

#[derive(BorshStorageKey, BorshSerialize)]
pub(crate) enum StorageKey {
    Accounts,
}

impl SignedAccount {
    pub async fn verify() {}
}
