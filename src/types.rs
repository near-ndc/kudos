use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::json_types::U64;
use near_sdk::serde::{Deserialize, Serialize};

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
