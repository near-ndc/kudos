use crate::external_db::ext_db;
use crate::registry::{ext_sbtreg, TokenId, TokenMetadata, IS_HUMAN_GAS};
use crate::settings::Settings;
use crate::types::{CommentId, Commentary, KudosId, MethodResult, PromiseFunctionCall};
use crate::{consts::*, EncodedCommentary, EscapedMessage, Hashtag};
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
    #[private]
    pub fn acquire_kudos_info(
        &mut self,
        predecessor_account_id: AccountId,
        attached_deposit: Balance,
        external_db_id: AccountId,
        receiver_id: AccountId,
        kudos_id: KudosId,
        comment: EncodedCommentary,
        #[callback_result] callback_result: Result<Vec<(AccountId, Vec<TokenId>)>, PromiseError>,
    ) -> PromiseOrValue<MethodResult<KudosId>> {
        let result = callback_result
            .map_err(|e| format!("IAHRegistry::is_human() call failure: {e:?}"))
            .and_then(|tokens| {
                if tokens.is_empty() {
                    return Err("IAHRegistry::is_human() returns result: Not a human".to_owned());
                }

                let root_id = env::current_account_id();
                let comment_id = CommentId::from(self.last_incremental_id.inc());
                let leave_comment_req = build_leave_comment_request(
                    &root_id,
                    &receiver_id,
                    &kudos_id,
                    &comment_id,
                    &comment,
                )?;
                let get_kudos_by_id_req =
                    build_get_kudos_by_id_request(&root_id, &receiver_id, &kudos_id);

                let get_kudos_by_id_gas = (env::prepaid_gas()
                    - (ACQUIRE_KUDOS_INFO_RESERVED_GAS
                        + KUDOS_INFO_ACQUIRED_CALLBACK_GAS
                        + KUDOS_COMMENT_SAVED_CALLBACK_GAS))
                    / 2;
                let get_kudos_by_id_callback_gas = get_kudos_by_id_gas
                    + KUDOS_INFO_ACQUIRED_CALLBACK_GAS
                    + KUDOS_COMMENT_SAVED_CALLBACK_GAS;

                Ok(ext_db::ext(external_db_id.clone())
                    .with_static_gas(get_kudos_by_id_gas)
                    .get(vec![get_kudos_by_id_req.clone()], None)
                    .then(
                        Self::ext(env::current_account_id())
                            .with_static_gas(get_kudos_by_id_callback_gas)
                            .on_kudos_info_acquired(
                                predecessor_account_id.clone(),
                                attached_deposit,
                                external_db_id,
                                get_kudos_by_id_req,
                                leave_comment_req,
                                comment_id,
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
    pub fn on_kudos_info_acquired(
        &mut self,
        predecessor_account_id: AccountId,
        attached_deposit: Balance,
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
                let gas_left = env::prepaid_gas()
                    - (KUDOS_INFO_ACQUIRED_CALLBACK_GAS + KUDOS_COMMENT_SAVED_CALLBACK_GAS);

                return ext_db::ext(external_db_id)
                    .with_attached_deposit(attached_deposit)
                    .with_static_gas(gas_left)
                    .set(leave_comment_req)
                    .then(
                        Self::ext(env::current_account_id())
                            .with_static_gas(KUDOS_COMMENT_SAVED_CALLBACK_GAS)
                            .on_commentary_saved(
                                predecessor_account_id.clone(),
                                attached_deposit,
                                comment_id,
                            ),
                    )
                    .into();
            }
        };

        // Return leave comment deposit back to sender if failed
        Promise::new(predecessor_account_id).transfer(attached_deposit);

        PromiseOrValue::Value(method_result)
    }

    #[private]
    pub fn on_commentary_saved(
        &mut self,
        predecessor_account_id: AccountId,
        attached_deposit: Balance,
        comment_id: CommentId,
        #[callback_result] callback_result: Result<(), PromiseError>,
    ) -> MethodResult<CommentId> {
        match callback_result {
            Ok(_) => MethodResult::Success(comment_id),
            Err(e) => {
                // Return deposit back to sender if NEAR SocialDb write failure
                Promise::new(predecessor_account_id).transfer(attached_deposit);

                MethodResult::Error(format!("SocialDB::set() call failure: {e:?}"))
            }
        }
    }
}
