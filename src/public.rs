use crate::external_db::ext_db;
use crate::registry::ext_sbtreg;
use crate::settings::Settings;
use crate::types::KudosId;
use crate::utils::{
    build_collect_kudos_upvotes_request, build_give_kudos_request, build_hashtags,
    build_leave_comment_request, build_upvote_kudos_request, build_verify_kudos_id_request,
    verify_kudos_id_response,
};
use crate::{Contract, ContractExt};
use near_sdk::json_types::Base64VecU8;
use near_sdk::serde_json::{self, json, Value};
use near_sdk::{env, near_bindgen, AccountId, Promise, PromiseError, PromiseOrValue};
use std::collections::HashMap;

#[near_bindgen]
impl Contract {
    /// Exchange upvoted Kudos for ProofOfKudos SBT
    #[payable]
    #[handle_result]
    pub fn exchange_kudos_for_sbt(
        &mut self,
        kudos_id: KudosId,
    ) -> Result<PromiseOrValue<bool>, &'static str> {
        self.assert_contract_running();
        // TODO: check for minimum required deposit

        if self.exchanged_kudos.contains(&kudos_id) {
            return Err("Kudos is already exchanged");
        }

        let external_db_id = self.external_db_id()?;
        let receiver_id = env::signer_account_id();
        let root_id = env::current_account_id();
        let collect_req = build_collect_kudos_upvotes_request(&root_id, &receiver_id, &kudos_id);

        Ok(ext_sbtreg::ext(self.iah_registry.clone())
            //.with_static_gas(static_gas) TODO: use pre-computed static amount gas
            .is_human_call(
                receiver_id,
                external_db_id.clone(),
                "keys".to_owned(),
                Base64VecU8::try_from(
                    json!({
                        "keys": [collect_req.clone()],
                    })
                    .to_string()
                    .into_bytes(),
                )
                .map_err(|_| "Internal serialization error")?,
            )
            .then(Self::ext(env::current_account_id()).send_sbt_mint_request(kudos_id))
            .into())
    }

    #[payable]
    #[handle_result]
    pub fn leave_comment(
        &mut self,
        receiver_id: AccountId,
        kudos_id: KudosId,
        text: String,
    ) -> Result<PromiseOrValue<()>, &'static str> {
        self.assert_contract_running();
        // TODO: check for minimum required deposit

        Settings::from(&self.settings).validate_commentary_text(&text);

        let external_db_id = self.external_db_id()?;
        let sender_id = env::signer_account_id();
        let root_id = env::current_account_id();
        let leave_comment_req =
            build_leave_comment_request(&root_id, &sender_id, &receiver_id, &kudos_id, &text)?;
        let verify_req = build_verify_kudos_id_request(&root_id, &receiver_id, &kudos_id);

        Ok(ext_sbtreg::ext(self.iah_registry.clone())
            //.with_static_gas(static_gas) TODO: use pre-computed static amount gas
            .is_human_call(
                sender_id,
                external_db_id.clone(),
                "keys".to_owned(),
                Base64VecU8::try_from(
                    json!({
                        "keys": [verify_req.clone()],
                    })
                    .to_string()
                    .into_bytes(),
                )
                .map_err(|_| "Internal serialization error")?,
            )
            .then(
                Self::ext(env::current_account_id()).send_verified_leave_comment_request(
                    external_db_id.clone(),
                    verify_req,
                    leave_comment_req,
                ),
            )
            .into())
    }

    #[payable]
    #[handle_result]
    pub fn upvote_kudos(
        &mut self,
        receiver_id: AccountId,
        kudos_id: KudosId,
    ) -> Result<PromiseOrValue<()>, &'static str> {
        self.assert_contract_running();
        // TODO: check for minimum required deposit

        let external_db_id = self.external_db_id()?;
        let sender_id = env::signer_account_id();
        let root_id = env::current_account_id();
        let upvote_req = build_upvote_kudos_request(&root_id, &sender_id, &receiver_id, &kudos_id)?;
        let verify_req = build_verify_kudos_id_request(&root_id, &receiver_id, &kudos_id);

        Ok(ext_sbtreg::ext(self.iah_registry.clone())
            //.with_static_gas(static_gas) TODO: use pre-computed static amount gas
            .is_human_call(
                sender_id,
                external_db_id.clone(),
                "keys".to_owned(),
                Base64VecU8::try_from(
                    json!({
                        "keys": [verify_req.clone()],
                    })
                    .to_string()
                    .into_bytes(),
                )
                .map_err(|_| "Internal serialization error")?,
            )
            .then(
                Self::ext(env::current_account_id()).send_verified_upvote_request(
                    external_db_id.clone(),
                    verify_req,
                    upvote_req,
                ),
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
        self.assert_contract_running();
        // TODO: check for minimum required deposit

        let settings = Settings::from(&self.settings);
        settings.validate_commentary_text(&text);
        if let Some(hashtags) = hashtags.as_ref() {
            settings.validate_hashtags(hashtags);
        }

        let external_db_id = self.external_db_id()?;
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

        Ok(ext_sbtreg::ext(self.iah_registry.clone())
            .with_attached_deposit(env::attached_deposit())
            //.with_static_gas(static_gas) TODO: use pre-computed static amount gas
            .is_human_call(
                sender_id,
                external_db_id.clone(),
                "set".to_owned(),
                Base64VecU8::try_from(
                    json!({
                        "data": data,
                    })
                    .to_string()
                    .into_bytes(),
                )
                .map_err(|_| "Internal serialization error")?,
            )
            .then(Self::ext(env::current_account_id()).on_kudos_saved(next_kudos_id))
            .into())
    }

    #[private]
    #[handle_result]
    pub fn send_verified_upvote_request(
        &mut self,
        external_db_id: AccountId,
        verify_req: String,
        upvote_req: Value,
        #[callback_result] callback_result: Result<Value, PromiseError>,
    ) -> Result<Promise, String> {
        let verify_res = callback_result
            .map_err(|e| format!("SocialDB::keys({verify_req}) call failure: {:?}", e))?;
        if !verify_kudos_id_response(&verify_req, verify_res) {
            return Err("Invalid kudos to upvote".to_owned());
        }

        Ok(ext_db::ext(external_db_id)
            .with_attached_deposit(env::attached_deposit())
            //.with_static_gas(static_gas) TODO: use pre-computed static amount gas
            .set(upvote_req)
            .then(Self::ext(env::current_account_id()).on_upvote_kudos_saved())
            .into())
    }

    #[private]
    #[handle_result]
    pub fn send_verified_leave_comment_request(
        &mut self,
        external_db_id: AccountId,
        verify_req: String,
        leave_comment_req: Value,
        #[callback_result] callback_result: Result<Value, PromiseError>,
    ) -> Result<Promise, String> {
        let verify_res = callback_result
            .map_err(|e| format!("SocialDB::keys({verify_req}) call failure: {:?}", e))?;
        if !verify_kudos_id_response(&verify_req, verify_res.clone()) {
            return Err(format!(
                "Invalid kudos to leave comment: {:?} ({})",
                verify_res,
                env::promise_results_count()
            ));
        }

        Ok(ext_db::ext(external_db_id)
            .with_attached_deposit(env::attached_deposit())
            //.with_static_gas(static_gas) TODO: use pre-computed static amount gas
            .set(leave_comment_req)
            .then(Self::ext(env::current_account_id()).on_upvote_kudos_saved())
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
    pub fn on_upvote_kudos_saved(
        &mut self,
        #[callback_result] callback_result: Result<(), PromiseError>,
    ) -> Result<(), String> {
        callback_result.map_err(|e| format!("SocialDB::set() call failure: {:?}", e))
    }

    #[private]
    pub fn send_sbt_mint_request(
        &mut self,
        kudos_id: KudosId,
        #[callback_result] callback_result: Result<HashMap<AccountId, bool>, PromiseError>,
    ) -> PromiseOrValue<bool> {
        let settings = Settings::from(&self.settings);

        match callback_result {
            Err(_) => return PromiseOrValue::Value(false),
            Ok(upvoters)
                if !settings.verify_number_of_upvotes_to_exchange_kudos(upvoters.keys().len()) =>
            {
                return PromiseOrValue::Value(false);
            }
            Ok(_) => (),
        };

        self.exchanged_kudos.insert(kudos_id);

        todo!()
    }
}

#[cfg(test)]
mod tests {}
