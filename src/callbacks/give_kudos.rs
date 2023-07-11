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
                let kudos_json = build_give_kudos_request(
                    &root_id,
                    &sender_id,
                    &receiver_id,
                    &kudos_id,
                    created_at,
                    &message,
                    hashtags.as_deref(),
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
}