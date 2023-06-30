use kudos_contract::{registry::TokenMetadata, KudosId};
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::json_types::Base64VecU8;
use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::serde_json::json;
use near_sdk::ONE_NEAR;
use near_sdk::{test_utils::VMContextBuilder, AccountId, Balance, Gas};
use near_units::parse_near;

pub const MAX_GAS: Gas = Gas(300_000_000_000_000);

#[derive(Deserialize, Serialize, Debug, PartialEq, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct OwnedToken {
    pub token: u64,
    pub metadata: TokenMetadata,
}

pub fn build_default_context(
    predecessor_account_id: AccountId,
    deposit: Option<Balance>,
    prepaid_gas: Option<Gas>,
) -> VMContextBuilder {
    let mut builder = VMContextBuilder::new();
    builder
        .signer_account_id(predecessor_account_id.clone())
        .predecessor_account_id(predecessor_account_id)
        .prepaid_gas(prepaid_gas.unwrap_or(MAX_GAS))
        .attached_deposit(deposit.unwrap_or_default());
    builder
}

pub async fn mint_fv_sbt(
    iah_registry_id: &workspaces::AccountId,
    issuer: &workspaces::Account,
    receiver_id: &workspaces::AccountId,
    issued_at: u64,  // SBT issued at in millis
    expires_at: u64, // SBT expires at in millis
) -> anyhow::Result<Vec<u64>> {
    let minted_tokens = issuer
        .call(iah_registry_id, "sbt_mint")
        .args_json(json!({
          "token_spec": [
            (receiver_id, [
              TokenMetadata {
                  class: 1, // FV SBT
                  issued_at: Some(issued_at),
                  expires_at: Some(expires_at),
                  reference: None,
                  reference_hash: None,
              }
            ])
          ]
        }))
        .deposit(parse_near!("0.006 N"))
        .max_gas()
        .transact()
        .await?
        .json()?;
    Ok(minted_tokens)
}

pub async fn verify_is_human(
    iah_registry_id: &workspaces::AccountId,
    issuer_id: &workspaces::AccountId,
    user_account: &workspaces::Account,
    tokens: &Vec<u64>,
) -> anyhow::Result<()> {
    let res = user_account
        .view(&iah_registry_id, "is_human")
        .args_json(json!({
          "account": user_account.id()
        }))
        .await?
        .json::<Vec<(AccountId, Vec<u64>)>>()?;

    match res.first() {
        Some((issuer_id_result, tokens_result))
            if issuer_id_result.as_str() == issuer_id.as_str() && tokens_result == tokens =>
        {
            Ok(())
        }
        _ => Err(anyhow::Error::msg("Not verified")),
    }
}

pub async fn give_kudos(
    kudos_contract_id: &workspaces::AccountId,
    sender: &workspaces::Account,
    receiver_id: &workspaces::AccountId,
    text: &str,
    hashtags: Vec<&str>,
) -> anyhow::Result<KudosId> {
    let kudos_id = sender
        .call(kudos_contract_id, "give_kudos")
        .args_json(json!({
            "receiver_id": receiver_id,
            "text": text,
            "hashtags": hashtags,
        }))
        .deposit(ONE_NEAR)
        .max_gas()
        .transact()
        .await?
        .json()?;
    Ok(kudos_id)
}

pub async fn upvote_kudos(
    kudos_contract_id: &workspaces::AccountId,
    sender: &workspaces::Account,
    receiver_id: &workspaces::AccountId,
    kudos_id: &KudosId,
) -> anyhow::Result<()> {
    let res = sender
        .call(kudos_contract_id, "upvote_kudos")
        .args_json(json!({
            "receiver_id": receiver_id,
            "kudos_id": kudos_id,
        }))
        .deposit(ONE_NEAR)
        .max_gas()
        .transact()
        .await?
        .json()?;
    Ok(res)
}

pub async fn leave_comment(
    kudos_contract_id: &workspaces::AccountId,
    sender: &workspaces::Account,
    receiver_id: &workspaces::AccountId,
    kudos_id: &KudosId,
    text: &str,
) -> anyhow::Result<()> {
    let res = sender
        .call(kudos_contract_id, "leave_comment")
        .args_json(json!({
            "receiver_id": receiver_id,
            "kudos_id": kudos_id,
            "text": text,
        }))
        .deposit(ONE_NEAR)
        .max_gas()
        .transact()
        .await?
        .json()?;
    Ok(res)
}

#[cfg(test)]
mod tests {}
