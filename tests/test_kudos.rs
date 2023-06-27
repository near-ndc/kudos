mod utils;
mod workspaces;

use crate::utils::remove_key_from_json;
use crate::workspaces::{build_contract, gen_user_account, transfer_near};
use kudos_contract::utils::build_verify_kudos_id_request;
use kudos_contract::KudosId;
use near_sdk::{serde_json::json, AccountId, ONE_NEAR};
use near_units::parse_near;

#[tokio::test]
async fn test_give_kudos() -> anyhow::Result<()> {
    let worker_mainnet = ::workspaces::mainnet_archival().await?;
    let near_social_id = "social.near".parse()?;
    let worker = ::workspaces::sandbox().await?;

    // Setup NEAR Social-DB contract
    let near_social = worker
        .import_contract(&near_social_id, &worker_mainnet)
        .initial_balance(parse_near!("10000000 N"))
        .block_height(94_000_000)
        .transact()
        .await?;
    let _ = near_social
        .call("new")
        .args_json(json!({}))
        .max_gas()
        .transact()
        .await?
        .into_result()?;
    let _ = near_social
        .call("set_status")
        .args_json(json!({"status": "Live"}))
        .max_gas()
        .transact()
        .await?
        .into_result()?;

    // Setup NDC Kudos Contract
    let kudos_contract = build_contract(&worker, "./", "init", json!({})).await?;
    let _ = kudos_contract
        .call("set_external_db")
        .args_json(json!({
            "external_db_id": near_social.id()
        }))
        .max_gas()
        .transact()
        .await?
        .into_result()?;

    // Register users' accounts
    let user1_account = gen_user_account(&worker, "user1.test.near").await?;
    let _ = transfer_near(&worker, user1_account.id(), parse_near!("50 N")).await?;

    let user2_account = gen_user_account(&worker, "user2.test.near").await?;
    let _ = transfer_near(&worker, user2_account.id(), parse_near!("50 N")).await?;

    let user3_account = gen_user_account(&worker, "user3.test.near").await?;
    let _ = transfer_near(&worker, user3_account.id(), parse_near!("50 N")).await?;

    // User1 gives kudos to User2
    let kudos_id: KudosId = user1_account
        .call(kudos_contract.id(), "give_kudos")
        .args_json(json!({
            "receiver_id": user2_account.id(),
            "text": "blablabla",
            "hashtags": vec!["hta","htb"]
        }))
        .max_gas()
        .deposit(ONE_NEAR)
        .transact()
        .await?
        .json()?;

    let kudos_root_prefix = build_verify_kudos_id_request(
        &AccountId::new_unchecked(kudos_contract.id().to_string()),
        &AccountId::new_unchecked(user2_account.id().to_string()),
        &kudos_id,
    );

    let hashtags_req = format!("{}/hashtags/**", kudos_contract.id());

    // Verify kudos on NEAR Social-DB contract
    let mut kudos_data: near_sdk::serde_json::Value = user2_account
        .view(&near_social_id, "get")
        .args_json(json!({ "keys": [format!("{kudos_root_prefix}/**"), hashtags_req] }))
        .await?
        .json()?;
    // remove `created_at` nested key to be able compare with static stringified json and verify that removed key were exist
    assert!(
        remove_key_from_json(&mut kudos_data, &format!("{kudos_root_prefix}/created_at")).is_some()
    );
    // kudos referenced by id and account of User2
    let kudos_reference = format!(r#"{{"{}":"{}"}}"#, kudos_id, user2_account.id());
    assert_eq!(
        kudos_data.to_string(),
        format!(
            r#"{{"{}":{{"hashtags":{{"hta":{kudos_reference},"htb":{kudos_reference}}},"kudos":{{"{}":{{"{kudos_id}":{{"sender_id":"{}","text":"blablabla"}}}}}}}}}}"#,
            kudos_contract.id(),
            user2_account.id(),
            user1_account.id()
        )
    );

    // User3 upvotes kudos given to User2 by User1
    let _ = user3_account
        .call(kudos_contract.id(), "upvote_kudos")
        .args_json(json!({
            "receiver_id": user2_account.id(),
            "kudos_id": kudos_id,
        }))
        .max_gas()
        .deposit(ONE_NEAR)
        .transact()
        .await?
        .into_result()?;

    // Verify upvoted kudos on NEAR Social-DB contract
    let mut kudos_data: near_sdk::serde_json::Value = user2_account
        .view(&near_social_id, "get")
        .args_json(json!({
            "keys": [format!("{kudos_root_prefix}/upvotes/**")]
        }))
        .await?
        .json()?;

    // remove `/upvotes` nested key and check for it's value, which should contain User3 who upvoted kudos
    let upvotes_json =
        remove_key_from_json(&mut kudos_data, &format!("{kudos_root_prefix}/upvotes"))
            .unwrap()
            .to_string();
    assert_eq!(upvotes_json, format!(r#"{{"{}":""}}"#, user3_account.id()));

    Ok(())
}
