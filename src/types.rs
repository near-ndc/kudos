use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::json_types::U64;
use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::{serde_json, AccountId, BorshStorageKey};
use std::fmt::Display;

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone, Debug)]
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

#[derive(BorshStorageKey, BorshSerialize)]
pub(crate) enum StorageKey {
    Kudos,
}

#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct PromiseFunctionCall {
    pub contract_id: AccountId,
    pub function_name: String,
    pub arguments: Vec<u8>,
    pub attached_deposit: Option<near_sdk::Balance>,
    pub static_gas: near_sdk::Gas,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(crate = "near_sdk::serde", tag = "status", content = "result")]
pub enum MethodResult<T> {
    Success(T),
    Error(String),
}

impl<T> MethodResult<T> {
    pub fn error<E: ToString>(err: E) -> Self {
        Self::Error(err.to_string())
    }
}
