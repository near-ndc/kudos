use crate::types::MethodResult;
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::json_types::Base64VecU8;
use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::serde_json::json;
use near_sdk::{test_utils::VMContextBuilder, AccountId, Balance, Gas};
use near_sdk::{PromiseOrValue, ONE_NEAR};
use near_units::parse_near;

pub const MAX_GAS: Gas = Gas(300_000_000_000_000);

pub fn build_default_context(
    predecessor_account_id: AccountId,
    deposit: Option<Balance>,
    prepaid_gas: Option<Gas>,
) -> VMContextBuilder {
    let mut builder = VMContextBuilder::new();
    builder
        .signer_account_id(predecessor_account_id.clone())
        .predecessor_account_id(predecessor_account_id)
        .prepaid_gas(prepaid_gas.unwrap_or(MAX_GAS))
        .attached_deposit(deposit.unwrap_or_default());
    builder
}

pub fn promise_or_value_into_result<T: std::fmt::Debug>(
    value: PromiseOrValue<MethodResult<T>>,
) -> Result<String, String> {
    match value {
        PromiseOrValue::Promise(promise) => near_sdk::serde_json::to_string(&promise)
            .map_err(|e| format!("Failed to serialize Promise: {e:?}")),
        PromiseOrValue::Value(MethodResult::Success(res)) => Ok(format!("{res:?}")),
        PromiseOrValue::Value(MethodResult::Error(e)) => Err(e),
    }
}
