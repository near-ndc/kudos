use crate::external_db::ext_db;
use crate::registry::{ext_sbtreg, TokenId, TokenMetadata, IS_HUMAN_GAS};
use crate::settings::Settings;
use crate::types::{CommentId, Commentary, KudosId, MethodResult, PromiseFunctionCall};
use crate::{consts::*, EscapedMessage, Hashtag};
use crate::{utils::*, GIVE_KUDOS_COST};
use crate::{Contract, ContractExt};
use near_sdk::json_types::Base64VecU8;
use near_sdk::serde_json::{self, json, Value};
use near_sdk::{
    env, near_bindgen, require, AccountId, Balance, Gas, Promise, PromiseError, PromiseOrValue,
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

        let minimum_gas_requirement = EXCHANGE_KUDOS_FOR_SBT_RESERVED_GAS
            + IS_HUMAN_GAS
            + ACQUIRE_NUMBER_OF_UPVOTES_RESERVED_GAS
            + SOCIAL_DB_REQUEST_MIN_RESERVED_GAS
            + KUDOS_UPVOTES_ACQUIRED_CALLBACK_GAS
            + PROOF_OF_KUDOS_SBT_MINT_GAS
            + PROOF_OF_KUDOS_SBT_MINT_CALLBACK_GAS;
        require!(
            env::prepaid_gas() >= minimum_gas_requirement,
            display_gas_requirement_in_tgas(minimum_gas_requirement)
        );

        let attached_deposit = env::attached_deposit();
        require!(
            attached_deposit == EXCHANGE_KUDOS_COST,
            &display_deposit_requirement_in_near(EXCHANGE_KUDOS_COST)
        );

        if self.exchanged_kudos.contains(&kudos_id) {
            return Err("Kudos is already exchanged");
        }

        let predecessor_account_id = env::predecessor_account_id();
        let external_db_id = self.external_db_id()?.clone();

        let gas_available = env::prepaid_gas()
            - (env::used_gas() + IS_HUMAN_GAS + EXCHANGE_KUDOS_FOR_SBT_RESERVED_GAS);

        Ok(ext_sbtreg::ext(self.iah_registry.clone())
            .with_static_gas(IS_HUMAN_GAS)
            .is_human(env::signer_account_id())
            .then(
                Self::ext(env::current_account_id())
                    .with_static_gas(gas_available)
                    .acquire_number_of_upvotes(
                        predecessor_account_id.clone(),
                        attached_deposit,
                        external_db_id,
                        kudos_id,
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

        let comment_id = CommentId::from(self.last_incremental_id.inc());
        let composed_comment = Commentary {
            sender_id: &sender_id,
            message: &EscapedMessage::new(
                &message,
                Settings::from(&self.settings).commentary_message_max_length as usize,
            )?,
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

        let minimum_gas_requirement = GIVE_KUDOS_RESERVED_GAS
            + IS_HUMAN_GAS
            + SAVE_KUDOS_RESERVED_GAS
            + SOCIAL_DB_REQUEST_MIN_RESERVED_GAS
            + KUDOS_SAVED_CALLBACK_GAS;
        require!(
            env::prepaid_gas() >= minimum_gas_requirement,
            display_gas_requirement_in_tgas(minimum_gas_requirement)
        );

        let attached_deposit = env::attached_deposit();
        require!(
            attached_deposit == GIVE_KUDOS_COST,
            &display_deposit_requirement_in_near(GIVE_KUDOS_COST)
        );

        let settings = Settings::from(&self.settings);
        let hashtags = settings.validate_hashtags(hashtags.as_deref())?;
        let message =
            EscapedMessage::new(&message, settings.commentary_message_max_length as usize)?;

        let external_db_id = self.external_db_id()?.clone();

        let gas_available =
            env::prepaid_gas() - (env::used_gas() + IS_HUMAN_GAS + GIVE_KUDOS_RESERVED_GAS);

        Ok(ext_sbtreg::ext(self.iah_registry.clone())
            .with_static_gas(IS_HUMAN_GAS)
            .is_human(sender_id.clone())
            .then(
                Self::ext(env::current_account_id())
                    .with_static_gas(gas_available)
                    .save_kudos(
                        predecessor_account_id.clone(),
                        attached_deposit,
                        external_db_id,
                        receiver_id,
                        message,
                        hashtags,
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
    pub fn acquire_number_of_upvotes(
        &mut self,
        predecessor_account_id: AccountId,
        attached_deposit: Balance,
        external_db_id: AccountId,
        kudos_id: KudosId,
        #[callback_result] callback_result: Result<Vec<(AccountId, Vec<TokenId>)>, PromiseError>,
    ) -> PromiseOrValue<MethodResult<KudosId>> {
        let result = callback_result
            .map_err(|e| format!("IAHRegistry::is_human() call failure: {e:?}"))
            .and_then(|tokens| {
                if tokens.is_empty() {
                    return Err("IAHRegistry::is_human() returns result: Not a human".to_owned());
                }

                let receiver_id = env::signer_account_id();
                let root_id = env::current_account_id();
                let kudos_upvotes_path =
                    build_kudos_upvotes_path(&root_id, &receiver_id, &kudos_id);
                let acquire_upvotes_req = [&kudos_upvotes_path, "/*"].concat();

                let upvotes_acquired_callback_gas = KUDOS_UPVOTES_ACQUIRED_CALLBACK_GAS
                    + PROOF_OF_KUDOS_SBT_MINT_GAS
                    + PROOF_OF_KUDOS_SBT_MINT_CALLBACK_GAS;

                let acquire_upvotes_gas = env::prepaid_gas()
                    - (ACQUIRE_NUMBER_OF_UPVOTES_RESERVED_GAS + upvotes_acquired_callback_gas);

                Ok(ext_db::ext(external_db_id)
                    .with_static_gas(acquire_upvotes_gas)
                    .keys(vec![acquire_upvotes_req], None)
                    .then(
                        Self::ext(env::current_account_id())
                            .with_static_gas(upvotes_acquired_callback_gas)
                            .on_kudos_upvotes_acquired(
                                predecessor_account_id.clone(),
                                attached_deposit,
                                kudos_id,
                                kudos_upvotes_path,
                            ),
                    ))
            });

        match result {
            Ok(promise) => promise.into(),
            Err(e) => {
                Promise::new(predecessor_account_id).transfer(attached_deposit);
                PromiseOrValue::Value(MethodResult::Error(e))
            }
        }
    }

    #[private]
    pub fn on_kudos_upvotes_acquired(
        &mut self,
        predecessor_account_id: AccountId,
        attached_deposit: Balance,
        kudos_id: KudosId,
        kudos_upvotes_path: String,
        #[callback_result] kudos_result: Result<Value, PromiseError>,
    ) -> PromiseOrValue<MethodResult<Vec<u64>>> {
        let settings = Settings::from(&self.settings);

        let result = kudos_result
            .map_err(|e| format!("SocialDB::keys({kudos_upvotes_path}/*) call failure: {e:?}"))
            .and_then(|mut kudos_json| {
                let upvotes_raw = remove_key_from_json(&mut kudos_json, &kudos_upvotes_path)
                    .ok_or_else(|| format!("No upvotes found for kudos: {kudos_json:?}"))?;

                let upvoters =
                    serde_json::from_value::<HashMap<AccountId, bool>>(upvotes_raw.clone())
                        .map_err(|e| {
                            format!("Failed to parse kudos upvotes data `{upvotes_raw:?}`: {e:?}")
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
                            .with_static_gas(PROOF_OF_KUDOS_SBT_MINT_CALLBACK_GAS)
                            .on_pok_sbt_mint(
                                predecessor_account_id.clone(),
                                attached_deposit,
                                kudos_id,
                            ),
                    )
                    .into();
            }
            Err(e) => {
                // Return leave comment deposit back to sender if failed
                Promise::new(predecessor_account_id).transfer(attached_deposit);

                PromiseOrValue::Value(MethodResult::Error(e))
            }
        }
    }

    #[private]
    #[handle_result]
    pub fn on_pok_sbt_mint(
        &mut self,
        predecessor_account_id: AccountId,
        attached_deposit: Balance,
        kudos_id: KudosId,
        #[callback_result] callback_result: Result<Vec<u64>, PromiseError>,
    ) -> Result<MethodResult<Vec<u64>>, &'static str> {
        match callback_result {
            Ok(minted_tokens_ids) if minted_tokens_ids.is_empty() => {
                // If IAHRegistry contract succeeds but returns an empty tokens list,
                // we treat is an unexpected failure and panic. No user deposit returns for this case.
                Err("IAHRegistry::sbt_mint() responses with an empty tokens array")
            }
            Ok(minted_tokens_ids) => Ok(MethodResult::Success(minted_tokens_ids)),
            Err(e) => {
                // If tokens weren't minted, remove kudos from exchanged table
                self.exchanged_kudos.remove(&kudos_id);

                // Return deposit back to sender if IAHRegistry::sbt_mint fails
                Promise::new(predecessor_account_id).transfer(attached_deposit);

                Ok(MethodResult::Error(format!(
                    "IAHRegistry::sbt_mint() call failure: {:?}",
                    e
                )))
            }
        }
    }

    #[private]
    pub fn save_kudos(
        &mut self,
        predecessor_account_id: AccountId,
        attached_deposit: Balance,
        external_db_id: AccountId,
        receiver_id: AccountId,
        message: EscapedMessage,
        hashtags: Option<Vec<Hashtag>>,
        #[callback_result] callback_result: Result<Vec<(AccountId, Vec<TokenId>)>, PromiseError>,
    ) -> PromiseOrValue<MethodResult<KudosId>> {
        let result = callback_result
            .map_err(|e| format!("IAHRegistry::is_human() call failure: {e:?}"))
            .and_then(|tokens| {
                if tokens.is_empty() {
                    return Err("IAHRegistry::is_human() returns result: Not a human".to_owned());
                }

                let sender_id = env::signer_account_id();
                let root_id = env::current_account_id();
                let created_at = env::block_timestamp_ms();
                let kudos_id = KudosId::from(self.last_incremental_id.inc());
                let hashtags = build_hashtags(&receiver_id, &kudos_id, hashtags)?;
                let kudos_json = build_give_kudos_request(
                    &root_id,
                    &sender_id,
                    &receiver_id,
                    &kudos_id,
                    created_at,
                    &message,
                    &hashtags,
                )?;

                let save_kudos_gas =
                    env::prepaid_gas() - (SAVE_KUDOS_RESERVED_GAS + KUDOS_SAVED_CALLBACK_GAS);

                Ok(ext_db::ext(external_db_id)
                    .with_static_gas(save_kudos_gas)
                    .with_attached_deposit(attached_deposit)
                    .set(kudos_json)
                    .then(
                        Self::ext(env::current_account_id())
                            .with_static_gas(KUDOS_SAVED_CALLBACK_GAS)
                            .on_kudos_saved(
                                predecessor_account_id.clone(),
                                attached_deposit,
                                kudos_id,
                            ),
                    ))
            });

        match result {
            Ok(promise) => promise.into(),
            Err(e) => {
                Promise::new(predecessor_account_id).transfer(attached_deposit);
                PromiseOrValue::Value(MethodResult::Error(e))
            }
        }
    }

    #[private]
    pub fn on_kudos_saved(
        &mut self,
        predecessor_account_id: AccountId,
        attached_deposit: Balance,
        kudos_id: KudosId,
        #[callback_result] callback_result: Result<(), PromiseError>,
    ) -> MethodResult<KudosId> {
        match callback_result {
            Ok(_) => MethodResult::Success(kudos_id),
            Err(e) => {
                // Return deposit back to sender if NEAR SocialDb write failure
                Promise::new(predecessor_account_id).transfer(attached_deposit);

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
    pub fn on_humanity_verified(
        &mut self,
        predecessor_account_id: AccountId,
        promise: PromiseFunctionCall,
        callback_promise: PromiseFunctionCall,
        #[callback_result] callback_result: Result<Vec<(AccountId, Vec<TokenId>)>, PromiseError>,
    ) -> PromiseOrValue<Option<KudosId>> {
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
