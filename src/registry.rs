use near_sdk::json_types::Base64VecU8;
use near_sdk::{ext_contract, AccountId, PromiseOrValue};

#[ext_contract(ext_sbtreg)]
pub trait ExtSbtRegistry {
    fn is_human_call(
        &mut self,
        account: AccountId,
        ctr: AccountId,
        function: String,
        args: Base64VecU8,
    ) -> PromiseOrValue<bool>;
}
