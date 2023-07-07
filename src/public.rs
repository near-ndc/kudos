use crate::consts::*;
use crate::external_db::ext_db;
use crate::registry::{ext_sbtreg, TokenId, TokenMetadata, IS_HUMAN_GAS};
use crate::settings::Settings;
use crate::types::{CommentId, Commentary, KudosId, MethodResult, PromiseFunctionCall};
use crate::{utils::*, GIVE_KUDOS_COST};
use crate::{Contract, ContractExt};
use near_sdk::json_types::Base64VecU8;
use near_sdk::serde_json::{self, json, Value};
use near_sdk::{
    env, near_bindgen, require, AccountId, Gas, Promise, PromiseError, PromiseOrValue,
    PromiseResult,
};
use std::collections::HashMap;

#[near_bindgen]
impl Contract {
    /// Exchange upvoted Kudos for ProofOfKudos SBT
    #[payable]
    #[handle_result]
    pub fn exchange_kudos_for_sbt(&mut self, kudos_id: KudosId) -> Result<Promise, &'static str> {
        self.assert_contract_running();

        require!(
            env::attached_deposit() == EXCHANGE_KUDOS_COST,
            &display_deposit_requirement_in_near(EXCHANGE_KUDOS_COST)
        );

        // TODO: check for minimum required gas

        if self.exchanged_kudos.contains(&kudos_id) {
            return Err("Kudos is already exchanged");
        }

        let predecessor_account_id = env::predecessor_account_id();
        let external_db_id = self.external_db_id()?;
        let receiver_id = env::signer_account_id();
        let root_id = env::current_account_id();
        let kudos_upvotes_path = build_kudos_upvotes_path(&root_id, &receiver_id, &kudos_id);
        let collect_upvotes_req = [&kudos_upvotes_path, "/*"].concat();

        let gas_available = env::prepaid_gas()
            - (env::used_gas() + IS_HUMAN_GAS + EXCHANGE_KUDOS_FOR_SBT_RESERVED_GAS);
        let collect_upvotes_gas =
            gas_available - (HUMANITY_VERIFIED_RESERVED_GAS + KUDOS_UPVOTES_CONFIRMED_CALLBACK_GAS);

        Ok(ext_sbtreg::ext(self.iah_registry.clone())
            .with_static_gas(IS_HUMAN_GAS)
            .is_human(receiver_id.clone())
            .then(
                Self::ext(env::current_account_id())
                    .with_static_gas(gas_available)
                    .on_humanity_verified(
                        predecessor_account_id.clone(),
                        PromiseFunctionCall {
                            contract_id: external_db_id.clone(),
                            function_name: "keys".to_owned(),
                            arguments: json!({
                                "keys": [collect_upvotes_req],
                            })
                            .to_string()
                            .into_bytes(),
                            attached_deposit: Some(env::attached_deposit()),
                            static_gas: collect_upvotes_gas,
                        },
                        PromiseFunctionCall {
                            contract_id: env::current_account_id(),
                            function_name: "send_sbt_mint_request".to_owned(),
                            arguments: json!({
                                "predecessor_account_id": predecessor_account_id,
                                "kudos_id": kudos_id,
                                "kudos_upvotes_path": kudos_upvotes_path,
                            })
                            .to_string()
                            .into_bytes(),
                            attached_deposit: None,
                            static_gas: KUDOS_UPVOTES_CONFIRMED_CALLBACK_GAS,
                        },
                    ),
            ))
    }

    #[payable]
    #[handle_result]
    pub fn leave_comment(
        &mut self,
        receiver_id: AccountId,
        kudos_id: KudosId,
        message: String,
    ) -> Result<Promise, &'static str> {
        self.assert_contract_running();

        let predecessor_account_id = env::predecessor_account_id();
        let sender_id = env::signer_account_id();
        require!(
            receiver_id != sender_id,
            "User is not eligible to leave a comment for this kudos"
        );

        // TODO: check for minimum required deposit
        // TODO: check for minimum required gas

        Settings::from(&self.settings).validate_commentary_message(&message);

        let comment_id = CommentId::from(self.last_incremental_id.inc());
        let composed_comment = Commentary {
            sender_id: &sender_id,
            message: &message,
            timestamp: env::block_timestamp_ms().into(),
        }
        .compose()
        .unwrap_or_else(|e| env::panic_str(&e));
        let external_db_id = self.external_db_id()?;
        let root_id = env::current_account_id();
        let leave_comment_req = build_leave_comment_request(
            &root_id,
            &receiver_id,
            &kudos_id,
            &comment_id,
            composed_comment.as_str(),
        )?;
        let get_kudos_by_id_req = build_get_kudos_by_id_request(&root_id, &receiver_id, &kudos_id);

        let gas_available =
            env::prepaid_gas() - (env::used_gas() + IS_HUMAN_GAS + LEAVE_COMMENT_RESERVED_GAS);
        let get_kudos_by_id_gas = (gas_available
            - (HUMANITY_VERIFIED_RESERVED_GAS
                + VERIFY_KUDOS_RESERVED_GAS
                + COMMENT_SAVED_CALLBACK_GAS))
            / 2;
        let get_kudos_by_id_callback_gas =
            get_kudos_by_id_gas + VERIFY_KUDOS_RESERVED_GAS + COMMENT_SAVED_CALLBACK_GAS;

        Ok(ext_sbtreg::ext(self.iah_registry.clone())
            .with_static_gas(IS_HUMAN_GAS)
            .is_human(sender_id.clone())
            .then(
                Self::ext(env::current_account_id())
                    .with_static_gas(gas_available)
                    .on_humanity_verified(
                        predecessor_account_id.clone(),
                        PromiseFunctionCall {
                            contract_id: external_db_id.clone(),
                            function_name: "get".to_owned(),
                            arguments: json!({
                                "keys": [&get_kudos_by_id_req],
                            })
                            .to_string()
                            .into_bytes(),
                            attached_deposit: Some(env::attached_deposit()),
                            static_gas: get_kudos_by_id_gas,
                        },
                        PromiseFunctionCall {
                            contract_id: env::current_account_id(),
                            function_name: "send_verified_leave_comment_request".to_owned(),
                            arguments: json!({
                                "predecessor_account_id": predecessor_account_id,
                                "external_db_id": external_db_id.clone(),
                                "get_kudos_by_id_req": get_kudos_by_id_req,
                                "leave_comment_req": leave_comment_req,
                                "comment_id": comment_id,
                            })
                            .to_string()
                            .into_bytes(),
                            attached_deposit: None,
                            static_gas: get_kudos_by_id_callback_gas,
                        },
                    ),
            ))
    }

    #[payable]
    #[handle_result]
    pub fn upvote_kudos(
        &mut self,
        receiver_id: AccountId,
        kudos_id: KudosId,
    ) -> Result<Promise, &'static str> {
        self.assert_contract_running();

        let predecessor_account_id = env::predecessor_account_id();
        let sender_id = env::signer_account_id();
        require!(
            receiver_id != sender_id,
            "User is not eligible to upvote this kudos"
        );

        // TODO: check for minimum required deposit
        // TODO: check for minimum required gas

        let external_db_id = self.external_db_id()?;
        let root_id = env::current_account_id();
        let upvote_req = build_upvote_kudos_request(&root_id, &sender_id, &receiver_id, &kudos_id)?;
        let get_kudos_by_id_req = build_get_kudos_by_id_request(&root_id, &receiver_id, &kudos_id);

        let gas_available =
            env::prepaid_gas() - (env::used_gas() + IS_HUMAN_GAS + UPVOTE_KUDOS_RESERVED_GAS);
        let get_kudos_by_id_gas = (gas_available
            - (HUMANITY_VERIFIED_RESERVED_GAS
                + VERIFY_KUDOS_RESERVED_GAS
                + UPVOTE_KUDOS_SAVED_CALLBACK_GAS))
            / 2;
        let get_kudos_by_id_callback_gas =
            get_kudos_by_id_gas + VERIFY_KUDOS_RESERVED_GAS + UPVOTE_KUDOS_SAVED_CALLBACK_GAS;

        Ok(ext_sbtreg::ext(self.iah_registry.clone())
            .with_static_gas(IS_HUMAN_GAS)
            .is_human(sender_id.clone())
            .then(
                Self::ext(env::current_account_id())
                    .with_static_gas(gas_available)
                    .on_humanity_verified(
                        predecessor_account_id.clone(),
                        PromiseFunctionCall {
                            contract_id: external_db_id.clone(),
                            function_name: "get".to_owned(),
                            arguments: json!({
                                "keys": [&get_kudos_by_id_req],
                            })
                            .to_string()
                            .into_bytes(),
                            attached_deposit: Some(env::attached_deposit()),
                            static_gas: get_kudos_by_id_gas,
                        },
                        PromiseFunctionCall {
                            contract_id: env::current_account_id(),
                            function_name: "send_verified_upvote_request".to_owned(),
                            arguments: json!({
                                "predecessor_account_id": predecessor_account_id,
                                "external_db_id": external_db_id.clone(),
                                "get_kudos_by_id_req": get_kudos_by_id_req,
                                "upvote_req": upvote_req,
                            })
                            .to_string()
                            .into_bytes(),
                            attached_deposit: None,
                            static_gas: get_kudos_by_id_callback_gas,
                        },
                    ),
            ))
    }

    #[payable]
    #[handle_result]
    pub fn give_kudos(
        &mut self,
        receiver_id: AccountId,
        message: String,
        hashtags: Option<Vec<String>>,
    ) -> Result<Promise, &'static str> {
        self.assert_contract_running();

        let predecessor_account_id = env::predecessor_account_id();
        let sender_id = env::signer_account_id();
        require!(
            receiver_id != sender_id,
            "User is not eligible to upvote this kudos"
        );

        require!(
            env::attached_deposit() == GIVE_KUDOS_COST,
            &display_deposit_requirement_in_near(GIVE_KUDOS_COST)
        );

        // TODO: check for minimum required gas

        let settings = Settings::from(&self.settings);
        settings.validate_commentary_message(&message);
        if let Some(hashtags) = hashtags.as_ref() {
            settings.validate_hashtags(hashtags);
        }

        let kudos_id = KudosId::from(self.last_incremental_id.inc());

        let external_db_id = self.external_db_id()?;
        let root_id = env::current_account_id();
        let created_at = env::block_timestamp_ms();
        // TODO: move hashtags & kudos objects build after receive IAHRegistry::is_human response
        // to prevent generating Kudos id for not a human accounts
        let hashtags = build_hashtags(&receiver_id, &kudos_id, hashtags)?;
        let data = build_give_kudos_request(
            &root_id,
            &sender_id,
            &receiver_id,
            &kudos_id,
            created_at,
            &message,
            &hashtags,
        )?;

        let gas_available =
            env::prepaid_gas() - (env::used_gas() + IS_HUMAN_GAS + GIVE_KUDOS_RESERVED_GAS);
        let save_kudos_gas =
            gas_available - (HUMANITY_VERIFIED_RESERVED_GAS + KUDOS_SAVED_CALLBACK_GAS);

        Ok(ext_sbtreg::ext(self.iah_registry.clone())
            .with_static_gas(IS_HUMAN_GAS)
            .is_human(sender_id.clone())
            .then(
                Self::ext(env::current_account_id())
                    .with_static_gas(gas_available)
                    .on_humanity_verified(
                        predecessor_account_id.clone(),
                        PromiseFunctionCall {
                            contract_id: external_db_id.clone(),
                            function_name: "set".to_owned(),
                            arguments: json!({
                                "data": data,
                            })
                            .to_string()
                            .into_bytes(),
                            attached_deposit: Some(env::attached_deposit()),
                            static_gas: save_kudos_gas,
                        },
                        PromiseFunctionCall {
                            contract_id: env::current_account_id(),
                            function_name: "on_kudos_saved".to_owned(),
                            arguments: json!({
                                "predecessor_account_id": predecessor_account_id,
                                "kudos_id": kudos_id,
                            })
                            .to_string()
                            .into_bytes(),
                            attached_deposit: None,
                            static_gas: KUDOS_SAVED_CALLBACK_GAS,
                        },
                    ),
            ))
    }

    #[private]
    pub fn send_verified_upvote_request(
        &mut self,
        predecessor_account_id: AccountId,
        external_db_id: AccountId,
        get_kudos_by_id_req: String,
        upvote_req: Value,
        #[callback_result] callback_result: Result<Value, PromiseError>,
    ) -> PromiseOrValue<MethodResult<u64>> {
        let method_result = match callback_result
            .map_err(|e| format!("SocialDB::get({get_kudos_by_id_req}) call failure: {e:?}"))
            .and_then(|kudos_by_id_res| {
                extract_kudos_id_sender_from_response(&get_kudos_by_id_req, kudos_by_id_res)
                    .ok_or_else(|| format!("Unable to acquire a Kudos sender account id"))
            }) {
            Err(e) => MethodResult::Error(e),
            Ok(sender_id) if sender_id == env::signer_account_id() => {
                MethodResult::error("User is not eligible to upvote this kudos")
            }
            Ok(_) => {
                let gas_available = env::prepaid_gas()
                    - (VERIFY_KUDOS_RESERVED_GAS + UPVOTE_KUDOS_SAVED_CALLBACK_GAS);

                return ext_db::ext(external_db_id)
                    .with_attached_deposit(env::attached_deposit())
                    .with_static_gas(gas_available)
                    .set(upvote_req)
                    .then(
                        Self::ext(env::current_account_id())
                            .with_static_gas(UPVOTE_KUDOS_SAVED_CALLBACK_GAS)
                            .on_social_db_data_saved(predecessor_account_id.clone()),
                    )
                    .into();
            }
        };

        // Return upvote deposit back to sender if failed
        Promise::new(predecessor_account_id).transfer(env::attached_deposit());

        PromiseOrValue::Value(method_result)
    }

    #[private]
    pub fn send_verified_leave_comment_request(
        &mut self,
        predecessor_account_id: AccountId,
        external_db_id: AccountId,
        get_kudos_by_id_req: String,
        leave_comment_req: Value,
        comment_id: CommentId,
        #[callback_result] callback_result: Result<Value, PromiseError>,
    ) -> PromiseOrValue<MethodResult<CommentId>> {
        let method_result = match callback_result
            .map_err(|e| {
                MethodResult::Error(format!(
                    "SocialDB::get({get_kudos_by_id_req}) call failure: {e:?}"
                ))
            })
            .and_then(|kudos_by_id_res| {
                extract_kudos_id_sender_from_response(&get_kudos_by_id_req, kudos_by_id_res)
                    .ok_or_else(|| {
                        MethodResult::error("Unable to acquire a Kudos sender account id")
                    })
            }) {
            Err(e) => e,
            Ok(_) => {
                let gas_left =
                    env::prepaid_gas() - (VERIFY_KUDOS_RESERVED_GAS + COMMENT_SAVED_CALLBACK_GAS);

                return ext_db::ext(external_db_id)
                    .with_attached_deposit(env::attached_deposit())
                    .with_static_gas(gas_left)
                    .set(leave_comment_req)
                    .then(
                        Self::ext(env::current_account_id())
                            .with_static_gas(COMMENT_SAVED_CALLBACK_GAS)
                            .on_commentary_saved(predecessor_account_id.clone(), comment_id),
                    )
                    .into();
            }
        };

        // Return leave comment deposit back to sender if failed
        Promise::new(predecessor_account_id).transfer(env::attached_deposit());

        PromiseOrValue::Value(method_result)
    }

    #[private]
    pub fn on_kudos_saved(
        &mut self,
        predecessor_account_id: AccountId,
        kudos_id: KudosId,
        #[callback_result] callback_result: Result<(), PromiseError>,
    ) -> MethodResult<KudosId> {
        match callback_result {
            Ok(_) => MethodResult::Success(kudos_id),
            Err(e) => {
                // Return deposit back to sender if NEAR SocialDb write failure
                Promise::new(predecessor_account_id).transfer(env::attached_deposit());

                MethodResult::Error(format!("SocialDB::set() call failure: {e:?}"))
            }
        }
    }

    #[private]
    pub fn on_commentary_saved(
        &mut self,
        predecessor_account_id: AccountId,
        comment_id: CommentId,
        #[callback_result] callback_result: Result<(), PromiseError>,
    ) -> MethodResult<CommentId> {
        match callback_result {
            Ok(_) => MethodResult::Success(comment_id),
            Err(e) => {
                // Return deposit back to sender if NEAR SocialDb write failure
                Promise::new(predecessor_account_id).transfer(env::attached_deposit());

                MethodResult::Error(format!("SocialDB::set() call failure: {e:?}"))
            }
        }
    }

    #[private]
    pub fn on_social_db_data_saved(
        &mut self,
        predecessor_account_id: AccountId,
        #[callback_result] callback_result: Result<(), PromiseError>,
    ) -> MethodResult<u64> {
        match callback_result {
            Ok(_) => MethodResult::Success(env::block_timestamp_ms()),
            Err(e) => {
                // Return deposit back to sender if NEAR SocialDb write failure
                Promise::new(predecessor_account_id).transfer(env::attached_deposit());

                MethodResult::Error(format!("SocialDB::set() call failure: {e:?}"))
            }
        }
    }

    #[private]
    pub fn send_sbt_mint_request(
        &mut self,
        predecessor_account_id: AccountId,
        kudos_id: KudosId,
        kudos_upvotes_path: String,
        #[callback_result] kudos_result: Result<Value, PromiseError>,
    ) -> PromiseOrValue<MethodResult<Vec<u64>>> {
        let settings = Settings::from(&self.settings);

        let result = kudos_result
            .map_err(|e| format!("SocialDB::keys({kudos_upvotes_path}/*) call failure: {e:?}"))
            .and_then(|mut kudos_json| {
                let upvotes_raw = remove_key_from_json(&mut kudos_json, &kudos_upvotes_path)
                    .ok_or_else(|| {
                        format!("No upvotes information found for kudos. Response: {kudos_json:?}")
                    })?;

                let upvoters =
                    serde_json::from_value::<HashMap<AccountId, bool>>(upvotes_raw.clone())
                        .map_err(|e| {
                            format!("Failed to parse upvotes data `{upvotes_raw:?}`: {e:?}")
                        })?;

                let number_of_upvotes = upvoters.keys().len();

                if !settings.verify_number_of_upvotes_to_exchange_kudos(number_of_upvotes) {
                    return Err(format!(
                        "Minimum required number ({}) of upvotes has not been reached",
                        settings.min_number_of_upvotes_to_exchange_kudos
                    ));
                }

                let issued_at = env::block_timestamp_ms();
                let expires_at = settings.acquire_pok_sbt_expire_at_ts(issued_at)?;

                Ok(build_pok_sbt_metadata(issued_at, expires_at))
            });

        match result {
            Ok(metadata) => {
                self.exchanged_kudos.insert(kudos_id.clone());

                return ext_sbtreg::ext(self.iah_registry.clone())
                    .with_attached_deposit(PROOF_OF_KUDOS_SBT_MINT_COST)
                    .with_static_gas(PROOF_OF_KUDOS_SBT_MINT_GAS)
                    .sbt_mint(vec![(env::signer_account_id(), vec![metadata])])
                    .then(
                        Self::ext(env::current_account_id())
                            .with_static_gas(COMMENT_SAVED_CALLBACK_GAS)
                            .on_pok_sbt_mint(predecessor_account_id.clone(), kudos_id),
                    )
                    .into();
            }
            Err(e) => {
                // Return leave comment deposit back to sender if failed
                Promise::new(predecessor_account_id).transfer(env::attached_deposit());

                PromiseOrValue::Value(MethodResult::Error(e))
            }
        }
    }

    #[private]
    pub fn on_pok_sbt_mint(
        &mut self,
        predecessor_account_id: AccountId,
        kudos_id: KudosId,
        #[callback_result] callback_result: Result<Vec<u64>, PromiseError>,
    ) -> MethodResult<Vec<u64>> {
        match callback_result {
            Ok(minted_tokens_ids) => MethodResult::Success(minted_tokens_ids),
            Err(e) => {
                // If tokens weren't minted, remove kudos from exchanged table
                self.exchanged_kudos.remove(&kudos_id);

                // Return deposit back to sender if IAHRegistry::sbt_mint fails
                Promise::new(predecessor_account_id).transfer(env::attached_deposit());

                MethodResult::Error(format!("IAHRegistry::sbt_mint() call failure: {:?}", e))
            }
        }
    }

    #[private]
    pub fn on_humanity_verified(
        &mut self,
        predecessor_account_id: AccountId,
        promise: PromiseFunctionCall,
        callback_promise: PromiseFunctionCall,
        #[callback_result] callback_result: Result<Vec<(AccountId, Vec<TokenId>)>, PromiseError>,
    ) -> PromiseOrValue<Option<KudosId>> {
        //env::log_str(&format!("IAHRegistry::is_human(): {callback_result:?}"));
        let promise: Option<Promise> = match callback_result {
            Ok(res) if res.is_empty() => {
                env::log_str("IAHRegistry::is_human() returns result: Not a human");
                None
            }
            Ok(_) => Promise::new(promise.contract_id)
                .function_call(
                    promise.function_name,
                    promise.arguments,
                    promise.attached_deposit.unwrap_or_default(),
                    promise.static_gas,
                )
                .then(Promise::new(callback_promise.contract_id).function_call(
                    callback_promise.function_name,
                    callback_promise.arguments,
                    callback_promise.attached_deposit.unwrap_or_default(),
                    callback_promise.static_gas,
                ))
                .into(),
            Err(e) => {
                env::log_str(&format!("IAHRegistry::is_human() call failure: {e:?}"));
                None
            }
        };

        promise.map(PromiseOrValue::from).unwrap_or_else(|| {
            Promise::new(predecessor_account_id).transfer(env::attached_deposit());
            PromiseOrValue::Value(None)
        })
    }
}
