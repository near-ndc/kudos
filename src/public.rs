use std::collections::HashMap;

use crate::external_db::ext_db;
use crate::types::KudosId;
use crate::utils::build_add_kudos_request;
use crate::{Contract, ContractExt};
use near_sdk::serde_json::{self, json, Value};
use near_sdk::{env, near_bindgen, AccountId, PromiseError, PromiseOrValue};

#[near_bindgen]
impl Contract {
    #[payable]
    pub fn upvote_kudos(&mut self, kudos_id: KudosId, text: Option<String>) {
        todo!()
    }

    #[payable]
    #[handle_result]
    pub fn add_kudos(
        &mut self,
        receiver_id: AccountId,
        text: String,
    ) -> Result<PromiseOrValue<Result<KudosId, String>>, &'static str> {
        // TODO: check for minimum required deposit

        let external_db_id = self
            .external_db_id
            .as_ref()
            .ok_or("External db is not set")?;

        let sender_id = env::signer_account_id();
        let next_kudos_id = self.last_kudos_id.next();
        let root_id = env::current_account_id();
        let data =
            build_add_kudos_request(&root_id, &sender_id, &receiver_id, &next_kudos_id, &text)?;

        env::log_str(&format!("{data:?}"));

        Ok(ext_db::ext(external_db_id.clone())
            .with_attached_deposit(env::attached_deposit())
            //.with_static_gas(static_gas) TODO: use pre-computed static amount gas
            .set(data)
            .then(Self::ext(env::current_account_id()).on_kudos_saved(next_kudos_id))
            .into())
    }

    #[private]
    #[handle_result]
    pub fn on_kudos_saved(
        &mut self,
        kudos_id: KudosId,
        #[callback_result] callback_result: Result<(), PromiseError>,
    ) -> Result<KudosId, String> {
        callback_result
            .map_err(|e| format!("SocialDB::set() call failure: {:?}", e))
            .map(|_| kudos_id)
    }
}

#[cfg(test)]
mod tests {}
