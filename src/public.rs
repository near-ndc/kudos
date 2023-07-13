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
            + PROOF_OF_KUDOS_SBT_MINT_CALLBACK_GAS
            + FAILURE_CALLBACK_GAS;
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
    ) -> Result<Promise, String> {
        self.assert_contract_running();

        let predecessor_account_id = env::predecessor_account_id();
        let sender_id = env::signer_account_id();
        require!(
            receiver_id != sender_id,
            "User is not eligible to leave a comment for this kudos"
        );

        let minimum_gas_requirement = LEAVE_COMMENT_RESERVED_GAS
            + IS_HUMAN_GAS
            + ACQUIRE_KUDOS_INFO_RESERVED_GAS
            + SOCIAL_DB_REQUEST_MIN_RESERVED_GAS
            + KUDOS_INFO_ACQUIRED_CALLBACK_GAS
            + SOCIAL_DB_REQUEST_MIN_RESERVED_GAS
            + KUDOS_COMMENT_SAVED_CALLBACK_GAS
            + FAILURE_CALLBACK_GAS;
        require!(
            env::prepaid_gas() >= minimum_gas_requirement,
            display_gas_requirement_in_tgas(minimum_gas_requirement)
        );

        let attached_deposit = env::attached_deposit();
        // TODO: check for minimum required deposit

        let external_db_id = self.external_db_id()?.clone();
        let comment = EncodedCommentary::try_from(&Commentary {
            sender_id: &sender_id,
            message: &EscapedMessage::new(
                &message,
                Settings::from(&self.settings).commentary_message_max_length as usize,
            )?,
            timestamp: env::block_timestamp_ms().into(),
        })?;

        let gas_available =
            env::prepaid_gas() - (env::used_gas() + IS_HUMAN_GAS + LEAVE_COMMENT_RESERVED_GAS);

        Ok(ext_sbtreg::ext(self.iah_registry.clone())
            .with_static_gas(IS_HUMAN_GAS)
            .is_human(env::signer_account_id())
            .then(
                Self::ext(env::current_account_id())
                    .with_static_gas(gas_available)
                    .acquire_kudos_info(
                        predecessor_account_id.clone(),
                        attached_deposit,
                        external_db_id,
                        receiver_id,
                        kudos_id,
                        comment,
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

        let minimum_gas_requirement = UPVOTE_KUDOS_RESERVED_GAS
            + IS_HUMAN_GAS
            + ACQUIRE_KUDOS_SENDER_RESERVED_GAS
            + SOCIAL_DB_REQUEST_MIN_RESERVED_GAS
            + KUDOS_SENDER_ACQUIRED_CALLBACK_GAS
            + SOCIAL_DB_REQUEST_MIN_RESERVED_GAS
            + KUDOS_UPVOTE_SAVED_CALLBACK_GAS
            + FAILURE_CALLBACK_GAS;
        require!(
            env::prepaid_gas() >= minimum_gas_requirement,
            display_gas_requirement_in_tgas(minimum_gas_requirement)
        );

        let attached_deposit = env::attached_deposit();
        // TODO: check for minimum required deposit
        let external_db_id = self.external_db_id()?.clone();

        let gas_available =
            env::prepaid_gas() - (env::used_gas() + IS_HUMAN_GAS + UPVOTE_KUDOS_RESERVED_GAS);

        Ok(ext_sbtreg::ext(self.iah_registry.clone())
            .with_static_gas(IS_HUMAN_GAS)
            .is_human(env::signer_account_id())
            .then(
                Self::ext(env::current_account_id())
                    .with_static_gas(gas_available)
                    .acquire_kudos_sender(
                        predecessor_account_id.clone(),
                        attached_deposit,
                        external_db_id,
                        receiver_id,
                        kudos_id,
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
            + KUDOS_SAVED_CALLBACK_GAS
            + FAILURE_CALLBACK_GAS;
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
}
