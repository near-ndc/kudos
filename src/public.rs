use crate::consts::{EXCHANGE_KUDOS_COST, PROOF_OF_KUDOS_SBT_MINT_COST};
use crate::external_db::ext_db;
use crate::registry::{ext_sbtreg, TokenMetadata};
use crate::settings::Settings;
use crate::types::KudosId;
use crate::utils::*;
use crate::{Contract, ContractExt};
use near_sdk::json_types::Base64VecU8;
use near_sdk::serde_json::{self, json, Value};
use near_sdk::{env, near_bindgen, require, AccountId, Promise, PromiseError, PromiseOrValue};
use std::collections::HashMap;

#[near_bindgen]
impl Contract {
    /// Exchange upvoted Kudos for ProofOfKudos SBT
    #[payable]
    #[handle_result]
    pub fn exchange_kudos_for_sbt(
        &mut self,
        kudos_id: KudosId,
    ) -> Result<PromiseOrValue<Vec<u64>>, &'static str> {
        self.assert_contract_running();

        require!(
            env::attached_deposit() == EXCHANGE_KUDOS_COST,
            &display_deposit_requirement_in_near(EXCHANGE_KUDOS_COST)
        );

        if self.exchanged_kudos.contains(&kudos_id) {
            return Err("Kudos is already exchanged");
        }

        let external_db_id = self.external_db_id()?;
        let receiver_id = env::signer_account_id();
        let root_id = env::current_account_id();
        let kudos_upvotes_path = build_kudos_upvotes_path(&root_id, &receiver_id, &kudos_id);
        let collect_upvotes_req = [&kudos_upvotes_path, "/*"].concat();

        Ok(ext_sbtreg::ext(self.iah_registry.clone())
            //.with_static_gas(static_gas) TODO: use pre-computed static amount gas
            .is_human_call(
                receiver_id,
                external_db_id.clone(),
                "keys".to_owned(),
                Base64VecU8::try_from(
                    json!({
                        "keys": [collect_upvotes_req],
                    })
                    .to_string()
                    .into_bytes(),
                )
                .map_err(|_| "Internal serialization error")?,
            )
            .then(
                Self::ext(env::current_account_id())
                    .send_sbt_mint_request(kudos_id, kudos_upvotes_path),
            )
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

        let sender_id = env::signer_account_id();
        require!(
            receiver_id != sender_id,
            "User is not eligible to leave a comment for this kudos"
        );

        // TODO: check for minimum required deposit

        Settings::from(&self.settings).validate_commentary_text(&text);

        let external_db_id = self.external_db_id()?;
        let root_id = env::current_account_id();
        let leave_comment_req =
            build_leave_comment_request(&root_id, &sender_id, &receiver_id, &kudos_id, &text)?;
        let get_kudos_by_id_req = build_get_kudos_by_id_request(&root_id, &receiver_id, &kudos_id);

        Ok(ext_sbtreg::ext(self.iah_registry.clone())
            //.with_static_gas(static_gas) TODO: use pre-computed static amount gas
            .is_human_call(
                sender_id,
                external_db_id.clone(),
                "get".to_owned(),
                Base64VecU8::try_from(
                    json!({
                        "keys": [&get_kudos_by_id_req],
                    })
                    .to_string()
                    .into_bytes(),
                )
                .map_err(|_| "Internal serialization error")?,
            )
            .then(
                Self::ext(env::current_account_id()).send_verified_leave_comment_request(
                    external_db_id.clone(),
                    get_kudos_by_id_req,
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

        let sender_id = env::signer_account_id();
        require!(
            receiver_id != sender_id,
            "User is not eligible to upvote this kudos"
        );

        // TODO: check for minimum required deposit

        let external_db_id = self.external_db_id()?;
        let root_id = env::current_account_id();
        let upvote_req = build_upvote_kudos_request(&root_id, &sender_id, &receiver_id, &kudos_id)?;
        let get_kudos_by_id_req = build_get_kudos_by_id_request(&root_id, &receiver_id, &kudos_id);

        Ok(ext_sbtreg::ext(self.iah_registry.clone())
            //.with_static_gas(static_gas) TODO: use pre-computed static amount gas
            .is_human_call(
                sender_id,
                external_db_id.clone(),
                "get".to_owned(),
                Base64VecU8::try_from(
                    json!({
                        "keys": [&get_kudos_by_id_req],
                    })
                    .to_string()
                    .into_bytes(),
                )
                .map_err(|_| "Internal serialization error")?,
            )
            .then(
                Self::ext(env::current_account_id()).send_verified_upvote_request(
                    external_db_id.clone(),
                    get_kudos_by_id_req,
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

        let sender_id = env::signer_account_id();
        require!(
            receiver_id != sender_id,
            "User is not eligible to upvote this kudos"
        );
        // TODO: check for minimum required deposit

        let settings = Settings::from(&self.settings);
        settings.validate_commentary_text(&text);
        if let Some(hashtags) = hashtags.as_ref() {
            settings.validate_hashtags(hashtags);
        }

        let external_db_id = self.external_db_id()?;
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
        get_kudos_by_id_req: String,
        upvote_req: Value,
        #[callback_result] callback_result: Result<Value, PromiseError>,
    ) -> Result<Promise, String> {
        let kudos_by_id_res = callback_result
            .map_err(|e| format!("SocialDB::get({get_kudos_by_id_req}) call failure: {:?}", e))?;

        match extract_kudos_id_sender_from_response(&get_kudos_by_id_req, kudos_by_id_res.clone()) {
            None => {
                return Err(format!(
                    "Invalid kudos to upvote Req: {get_kudos_by_id_req:?} Res: {kudos_by_id_res:?}"
                ));
            }
            Some(sender_id) if sender_id == env::signer_account_id() => {
                return Err("User is not eligible to upvote this kudos".to_owned());
            }
            Some(_) => (),
        };

        Ok(ext_db::ext(external_db_id)
            .with_attached_deposit(env::attached_deposit())
            //.with_static_gas(static_gas) TODO: use pre-computed static amount gas
            .set(upvote_req)
            .then(Self::ext(env::current_account_id()).on_social_db_data_saved())
            .into())
    }

    #[private]
    #[handle_result]
    pub fn send_verified_leave_comment_request(
        &mut self,
        external_db_id: AccountId,
        get_kudos_by_id_req: String,
        leave_comment_req: Value,
        #[callback_result] callback_result: Result<Value, PromiseError>,
    ) -> Result<Promise, String> {
        let kudos_by_id_res = callback_result
            .map_err(|e| format!("SocialDB::get({get_kudos_by_id_req}) call failure: {:?}", e))?;

        if extract_kudos_id_sender_from_response(&get_kudos_by_id_req, kudos_by_id_res).is_none() {
            return Err("Invalid kudos to leave a comment for".to_owned());
        }

        Ok(ext_db::ext(external_db_id)
            .with_attached_deposit(env::attached_deposit())
            //.with_static_gas(static_gas) TODO: use pre-computed static amount gas
            .set(leave_comment_req)
            .then(Self::ext(env::current_account_id()).on_social_db_data_saved())
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
    pub fn on_social_db_data_saved(
        &mut self,
        #[callback_result] callback_result: Result<(), PromiseError>,
    ) -> Result<(), String> {
        callback_result.map_err(|e| format!("SocialDB::set() call failure: {:?}", e))
    }

    #[private]
    #[handle_result]
    pub fn send_sbt_mint_request(
        &mut self,
        kudos_id: KudosId,
        kudos_upvotes_path: String,
        #[callback_result] callback_result: Result<Value, PromiseError>,
    ) -> Result<PromiseOrValue<Vec<u64>>, String> {
        let mut result = callback_result.map_err(|e| {
            format!(
                "SocialDB::keys({kudos_upvotes_path}/*) call failure: {:?}",
                e
            )
        })?;

        let upvoters = remove_key_from_json(&mut result, &kudos_upvotes_path)
            .ok_or_else(|| {
                format!("SocialDB::keys({kudos_upvotes_path}/*) invalid response {result:?}")
            })
            .and_then(|upvotes| {
                serde_json::from_value::<HashMap<AccountId, bool>>(upvotes.clone())
                    .map_err(|e| format!("Failed to parse upvotes data `{upvotes:?}`: {e:?}"))
            })?;

        let number_of_upvotes = upvoters.keys().len();
        let settings = Settings::from(&self.settings);

        if !settings.verify_number_of_upvotes_to_exchange_kudos(number_of_upvotes) {
            return Err(format!(
                "Minimum required number ({}) of upvotes is not reached",
                settings.min_number_of_upvotes_to_exchange_kudos
            ));
        }

        let issued_at = env::block_timestamp_ms();
        let expires_at = settings.acquire_pok_sbt_expire_at_ts(issued_at)?;

        self.exchanged_kudos.insert(kudos_id.clone());

        Ok(ext_sbtreg::ext(self.iah_registry.clone())
            .with_attached_deposit(PROOF_OF_KUDOS_SBT_MINT_COST)
            //.with_static_gas(static_gas) TODO: use pre-computed static amount gas
            .sbt_mint(vec![(
                env::signer_account_id(),
                vec![build_pok_sbt_metadata(issued_at, expires_at)],
            )])
            .then(Self::ext(env::current_account_id()).on_pok_sbt_mint(kudos_id))
            .into())
    }

    #[private]
    pub fn on_pok_sbt_mint(
        &mut self,
        kudos_id: KudosId,
        #[callback_result] callback_result: Result<Vec<u64>, PromiseError>,
    ) -> PromiseOrValue<Vec<u64>> {
        let minted_tokens_ids = callback_result.unwrap_or_else(|e| {
            env::log_str(&format!("IAHRegistry::sbt_mint() call failure: {:?}", e));
            vec![]
        });

        if minted_tokens_ids.is_empty() {
            // If tokens weren't minted, remove kudos from exchanged table
            self.exchanged_kudos.remove(&kudos_id);
        }

        PromiseOrValue::Value(minted_tokens_ids)
    }
}
