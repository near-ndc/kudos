use anyhow::anyhow;
use kudos_contract::registry::{OwnedToken, TokenMetadata};
use kudos_contract::{
    CommentId, KudosId, EXCHANGE_KUDOS_COST, GIVE_KUDOS_COST, LEAVE_COMMENT_COST, UPVOTE_KUDOS_COST,
};
use near_sdk::json_types::U64;
use near_sdk::serde_json::json;
use near_sdk::AccountId;
use near_units::parse_near;
use workspaces::result::ExecutionOutcome;

pub async fn mint_fv_sbt(
    iah_registry_id: &workspaces::AccountId,
    issuer: &workspaces::Account,
    receivers: &[&workspaces::AccountId],
    issued_at: u64,  // SBT issued at in millis
    expires_at: u64, // SBT expires at in millis
) -> anyhow::Result<Vec<u64>> {
    let minted_tokens = issuer
        .call(iah_registry_id, "sbt_mint")
        .args_json(json!({
          "token_spec": receivers.into_iter().map(|receiver_id| (receiver_id, [
              TokenMetadata {
                  class: 1, // FV SBT
                  issued_at: Some(issued_at),
                  expires_at: Some(expires_at),
                  reference: None,
                  reference_hash: None,
              }
            ])
            ).collect::<Vec<_>>()
        }))
        .deposit(parse_near!("0.006 N") * receivers.len() as u128)
        .max_gas()
        .transact()
        .await?
        .json()?;

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
    message: &str,
    hashtags: Vec<&str>,
) -> anyhow::Result<KudosId> {
    let res = sender
        .call(kudos_contract_id, "give_kudos")
        .args_json(json!({
            "receiver_id": receiver_id,
            "message": message,
            "hashtags": hashtags,
        }))
        .deposit(GIVE_KUDOS_COST)
        .max_gas()
        .transact()
        .await?
        .into_result()
        .map_err(|e| {
            anyhow::Error::msg(format!(
                "Give kudos failure: {:?}",
                extract_error(e.outcomes().into_iter())
            ))
        });

    res.and_then(|res| {
        println!("gas burnt: {}", res.total_gas_burnt);
        res.json().map_err(|e| {
            anyhow::Error::msg(format!(
                "Failed to deserialize give kudos response: {e:?}. Receipts: {:?}",
                res.receipt_outcomes()
            ))
        })
    })
}

pub async fn upvote_kudos(
    kudos_contract_id: &workspaces::AccountId,
    sender: &workspaces::Account,
    receiver_id: &workspaces::AccountId,
    kudos_id: &KudosId,
) -> anyhow::Result<U64> {
    let res = sender
        .call(kudos_contract_id, "upvote_kudos")
        .args_json(json!({
            "receiver_id": receiver_id,
            "kudos_id": kudos_id,
        }))
        .deposit(UPVOTE_KUDOS_COST)
        .max_gas()
        .transact()
        .await?
        .into_result()
        .map_err(|e| {
            anyhow::Error::msg(format!(
                "Upvote kudos failure: {:?}",
                extract_error(e.outcomes().into_iter())
            ))
        });

    res.and_then(|res| {
        println!("gas burnt: {}", res.total_gas_burnt);
        res.json().map_err(|e| {
            anyhow::Error::msg(format!(
                "Failed to deserialize upvote kudos response: {e:?}. Receipts: {:?}",
                res.receipt_outcomes()
            ))
        })
    })
}

pub async fn leave_comment(
    kudos_contract_id: &workspaces::AccountId,
    sender: &workspaces::Account,
    receiver_id: &workspaces::AccountId,
    kudos_id: &KudosId,
    message: &str,
) -> anyhow::Result<CommentId> {
    let res = sender
        .call(kudos_contract_id, "leave_comment")
        .args_json(json!({
            "receiver_id": receiver_id,
            "kudos_id": kudos_id,
            "message": message,
        }))
        .deposit(LEAVE_COMMENT_COST)
        .max_gas()
        .transact()
        .await?
        .into_result()
        .map_err(|e| {
            anyhow::Error::msg(format!(
                "Leave comment failure: {:?}",
                extract_error(e.outcomes().into_iter())
            ))
        });

    res.and_then(|res| {
        println!("gas burnt: {}", res.total_gas_burnt);
        res.json().map_err(|e| {
            anyhow::Error::msg(format!(
                "Failed to deserialize leave comment response: {e:?}. Receipts: {:?}",
                res.receipt_outcomes()
            ))
        })
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
        .map_err(|e| {
            anyhow::Error::msg(format!(
                "Exchange kudos failure: {:?}",
                extract_error(e.outcomes().into_iter())
            ))
        });

    res.and_then(|res| {
        res.json().map_err(|e| {
            anyhow::Error::msg(format!(
                "Failed to deserialize exchange kudos response: {e:?}. Receipts: {:?}",
                res.receipt_outcomes()
            ))
        })
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

pub fn extract_error<'a, I>(mut outcomes: I) -> anyhow::Error
where
    I: Iterator<Item = &'a ExecutionOutcome>,
{
    outcomes
        .find(|&outcome| outcome.is_failure())
        //.and_then(|outcome| outcome.clone().into_result().err())
        .map(|outcome| {
            outcome
                .clone()
                .into_result()
                .map_err(|e| anyhow!(e.into_inner().unwrap()))
                .unwrap_err()
        })
        .unwrap()
}
