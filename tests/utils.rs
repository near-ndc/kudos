use kudos_contract::{registry::TokenMetadata, KudosId, PROOF_OF_KUDOS_SBT_CLASS_ID};
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
    receivers: &[&workspaces::AccountId],
    issued_at: u64,  // SBT issued at in millis
    expires_at: u64, // SBT expires at in millis
) -> anyhow::Result<Vec<u64>> {
    let mut minted_tokens = Vec::with_capacity(receivers.len());

    for receiver_id in receivers {
        let tokens: Vec<u64> = issuer
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

        minted_tokens.extend(&tokens);
    }

    Ok(minted_tokens)
}

pub async fn verify_is_human(
    iah_registry_id: &workspaces::AccountId,
    issuer_id: &workspaces::AccountId,
    users_accounts: &[&workspaces::Account],
    tokens: &Vec<u64>,
) -> anyhow::Result<()> {
    for (i, &user_account) in users_accounts.into_iter().enumerate() {
        let res = user_account
            .view(&iah_registry_id, "is_human")
            .args_json(json!({
              "account": user_account.id()
            }))
            .await?
            .json::<Vec<(AccountId, Vec<u64>)>>()?;

        match res.first() {
            Some((issuer_id_result, tokens_result))
                if issuer_id_result.as_str() != issuer_id.as_str()
                    && tokens_result[0] != tokens[i] =>
            {
                return Err(anyhow::Error::msg(format!(
                    "User `{}` not verified",
                    user_account.id()
                )));
            }
            _ => (),
        };
    }

    Ok(())
}

pub async fn verify_kudos_sbt_tokens_by_owner(
    iah_registry_id: &workspaces::AccountId,
    issuer_id: &workspaces::AccountId,
    owner: &workspaces::Account,
    tokens_ids: &[u64],
) -> anyhow::Result<()> {
    let res = owner
        .view(&iah_registry_id, "sbt_tokens_by_owner")
        .args_json(json!({
          "account": owner.id(),
          "issuer": issuer_id,
        }))
        .await?
        .json::<Vec<(AccountId, Vec<OwnedToken>)>>()?;

    match res.first() {
        Some((issuer_id_result, tokens_result))
            if issuer_id_result.as_str() != issuer_id.as_str()
                && compare_slices(
                    &tokens_result
                        .into_iter()
                        .map(|token_res| token_res.token)
                        .collect::<Vec<_>>(),
                    tokens_ids,
                ) =>
        {
            Err(anyhow::Error::msg(format!(
                "User `{}` do not have ProofOfKudos SBT",
                owner.id()
            )))
        }
        _ => Ok(()),
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

pub async fn exchange_kudos_for_sbt(
    kudos_contract_id: &workspaces::AccountId,
    requestor: &workspaces::Account,
    kudos_id: &KudosId,
) -> anyhow::Result<Vec<u64>> {
    let res = requestor
        .call(kudos_contract_id, "exchange_kudos_for_sbt")
        .args_json(json!({
            "kudos_id": kudos_id,
        }))
        .deposit(ONE_NEAR)
        .max_gas()
        .transact()
        .await?
        .into_result();

    match res {
        Ok(res) => {
            //println!("Result: {res:?}");
            res.json::<Vec<u64>>().map_err(anyhow::Error::msg)
        }
        Err(e) => Err(anyhow::Error::msg(format!(
            "Exchange kudos failure. Error: {:?}",
            e
        ))),
    }
}

// TODO: pass iterators instead
fn compare_slices<T: PartialEq>(sl1: &[T], sl2: &[T]) -> bool {
    let count = sl1
        .iter()
        .zip(sl2)
        .filter(|&(item1, item2)| item1 == item2)
        .count();

    count == sl1.len() && count == sl2.len()
}

#[cfg(test)]
mod tests {}
