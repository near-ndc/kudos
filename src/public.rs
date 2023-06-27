use crate::external_db::ext_db;
use crate::types::KudosId;
use crate::utils::{
    build_give_kudos_request, build_hashtags, build_upvote_kudos_request,
    build_verify_kudos_id_request,
};
use crate::{Contract, ContractExt};
use near_sdk::serde_json::{self, json, Value};
use near_sdk::{env, near_bindgen, AccountId, Promise, PromiseError, PromiseOrValue};

#[near_bindgen]
impl Contract {
    #[payable]
    #[handle_result]
    pub fn leave_comment(&mut self, receiver_id: AccountId, kudos_id: KudosId, text: String) {
        todo!()
    }

    #[payable]
    #[handle_result]
    pub fn upvote_kudos(
        &mut self,
        receiver_id: AccountId,
        kudos_id: KudosId,
    ) -> Result<PromiseOrValue<()>, &'static str> {
        // TODO: check for minimum required deposit

        let external_db_id = self
            .external_db_id
            .as_ref()
            .ok_or("External db is not set")?;

        let sender_id = env::signer_account_id();
        let root_id = env::current_account_id();
        let upvote_req = build_upvote_kudos_request(&root_id, &sender_id, &receiver_id, &kudos_id)?;

        let verify_req = build_verify_kudos_id_request(&root_id, &receiver_id, &kudos_id);

        Ok(ext_db::ext(external_db_id.clone())
            //.with_static_gas(static_gas) TODO: use pre-computed static amount gas
            .get(vec![verify_req], None)
            .then(
                Self::ext(env::current_account_id())
                    .on_kudos_verified(external_db_id.clone(), upvote_req),
            )
            .into())
    }

    #[payable]
    #[handle_result]
    pub fn give_kudos(
        &mut self,
        receiver_id: AccountId,
        text: String,
        hashtags: Option<Vec<String>>,
    ) -> Result<PromiseOrValue<Result<KudosId, String>>, &'static str> {
        // TODO: check for minimum required deposit

        let external_db_id = self
            .external_db_id
            .as_ref()
            .ok_or("External db is not set")?;

        let sender_id = env::signer_account_id();
        let next_kudos_id = self.last_kudos_id.next();
        let root_id = env::current_account_id();
        let created_at = env::block_timestamp_ms();
        let hashtags = build_hashtags(&receiver_id, &next_kudos_id, hashtags)?;
        let data = build_give_kudos_request(
            &root_id,
            &sender_id,
            &receiver_id,
            &next_kudos_id,
            created_at,
            &text,
            &hashtags,
        )?;

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

    #[private]
    #[handle_result]
    pub fn on_kudos_verified(
        &mut self,
        external_db_id: AccountId,
        upvote_req: Value,
        #[callback_result] callback_result: Result<Value, PromiseError>,
    ) -> Result<Promise, String> {
        // TODO: Verify result response
        let result =
            callback_result.map_err(|e| format!("SocialDB::get() call failure: {:?}", e))?;

        Ok(ext_db::ext(external_db_id)
            .with_attached_deposit(env::attached_deposit())
            //.with_static_gas(static_gas) TODO: use pre-computed static amount gas
            .set(upvote_req)
            .then(Self::ext(env::current_account_id()).on_upvote_kudos_saved())
            .into())
    }

    #[private]
    #[handle_result]
    pub fn on_upvote_kudos_saved(
        &mut self,
        #[callback_result] callback_result: Result<(), PromiseError>,
    ) -> Result<(), String> {
        callback_result.map_err(|e| format!("SocialDB::set() call failure: {:?}", e))
    }
}

#[cfg(test)]
mod tests {}
