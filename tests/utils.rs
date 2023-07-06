use kudos_contract::registry::{OwnedToken, TokenMetadata};
use kudos_contract::{CommentId, KudosId, MethodResult, EXCHANGE_KUDOS_COST, GIVE_KUDOS_COST};
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::json_types::Base64VecU8;
use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::serde_json::json;
use near_sdk::{AccountId, ONE_NEAR};
use near_units::parse_near;

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
    let res = sender
        .call(kudos_contract_id, "give_kudos")
        .args_json(json!({
            "receiver_id": receiver_id,
            "text": text,
            "hashtags": hashtags,
        }))
        .deposit(GIVE_KUDOS_COST)
        .max_gas()
        .transact()
        .await?
        .into_result()
        .map_err(|e| anyhow::Error::msg(format!("Give kudos failure: {e:?}")));

    res.and_then(|res| match res.json::<MethodResult<_>>() {
        Ok(MethodResult::Success(kudos_id)) => Ok(kudos_id),
        Ok(MethodResult::Error(e)) => Err(anyhow::Error::msg(format!(
            "Failed to give kudos. Error: {e}. Receipts: {:?}",
            res.receipt_outcomes(),
        ))),
        Err(e) => Err(anyhow::Error::msg(format!(
            "Failed to deserialize give kudos response: {e:?}. Receipts: {:?}",
            res.receipt_outcomes()
        ))),
    })
}

pub async fn upvote_kudos(
    kudos_contract_id: &workspaces::AccountId,
    sender: &workspaces::Account,
    receiver_id: &workspaces::AccountId,
    kudos_id: &KudosId,
) -> anyhow::Result<u64> {
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
        .into_result()
        .map_err(|e| anyhow::Error::msg(format!("Upvote kudos failure: {e:?}")));

    res.and_then(|res| match res.json::<MethodResult<_>>() {
        Ok(MethodResult::Success(created_at)) => Ok(created_at),
        Ok(MethodResult::Error(e)) => Err(anyhow::Error::msg(format!(
            "Failed to upvotes kudos. Error: {e}. Receipts: {:?}",
            res.receipt_outcomes(),
        ))),
        Err(e) => Err(anyhow::Error::msg(format!(
            "Failed to deserialize upvote kudos response: {e:?}. Receipts: {:?}",
            res.receipt_outcomes()
        ))),
    })
}

pub async fn leave_comment(
    kudos_contract_id: &workspaces::AccountId,
    sender: &workspaces::Account,
    receiver_id: &workspaces::AccountId,
    kudos_id: &KudosId,
    text: &str,
) -> anyhow::Result<CommentId> {
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
        .into_result()
        .map_err(|e| anyhow::Error::msg(format!("Leave comment failure: {e:?}")));

    res.and_then(|res| match res.json::<MethodResult<_>>() {
        Ok(MethodResult::Success(created_at)) => Ok(created_at),
        Ok(MethodResult::Error(e)) => Err(anyhow::Error::msg(format!(
            "Failed to leave a comment for kudos. Error: {e}. Receipts: {:?}",
            res.receipt_outcomes(),
        ))),
        Err(e) => Err(anyhow::Error::msg(format!(
            "Failed to deserialize leave comment response: {e:?}. Receipts: {:?}",
            res.receipt_outcomes()
        ))),
    })
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
        .deposit(EXCHANGE_KUDOS_COST)
        .max_gas()
        .transact()
        .await?
        .into_result()
        .map_err(|e| anyhow::Error::msg(format!("Exchange kudos failure: {e:?}")));

    res.and_then(|res| match res.json::<MethodResult<_>>() {
        Ok(MethodResult::Success(tokens)) => Ok(tokens),
        Ok(MethodResult::Error(e)) => Err(anyhow::Error::msg(format!(
            "Failed to exchange kudos. Error: {e}. Receipts: {:?}",
            res.receipt_outcomes(),
        ))),
        Err(e) => Err(anyhow::Error::msg(format!(
            "Failed to deserialize exchange kudos response: {e:?}. Receipts: {:?}",
            res.receipt_outcomes()
        ))),
    })
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
