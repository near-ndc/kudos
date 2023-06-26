mod utils;
mod workspaces;

use crate::workspaces::{build_contract, gen_user_account, transfer_near};
use kudos_contract::KudosId;
use near_sdk::{serde_json::json, ONE_NEAR};
use near_units::parse_near;

#[tokio::test]
async fn test_add_kudos() -> anyhow::Result<()> {
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

    // Register user1 & user2
    let user1_account = gen_user_account(&worker, "user1.test.near").await?;
    let _ = transfer_near(&worker, user1_account.id(), parse_near!("50 N")).await?;

    let user2_account = gen_user_account(&worker, "user2.test.near").await?;
    let _ = transfer_near(&worker, user1_account.id(), parse_near!("50 N")).await?;

    // User1 adds kudos to User2
    let kudos_id: KudosId = user1_account
        .call(kudos_contract.id(), "add_kudos")
        .args_json(json!({
            "receiver_id": user2_account.id(),
            "text": "blablabla",
        }))
        .max_gas()
        .deposit(ONE_NEAR)
        .transact()
        .await?
        .json()?;

    // Verify kudos on NEAR Social-DB contract
    let kudos_data: near_sdk::serde_json::Value = user2_account
        .view(&near_social_id, "get")
        .args_json(json!({
            "keys": [format!("{}/kudos/{}/**", kudos_contract.id(), user2_account.id())]
        }))
        .await?
        .json()?;
    assert_eq!(
        kudos_data.to_string(),
        format!(
            r#"{{"{}":{{"kudos":{{"{}":{{"{kudos_id}":{{"sender_id":"{}","text":"blablabla"}}}}}}}}}}"#,
            kudos_contract.id(),
            user2_account.id(),
            user1_account.id()
        )
    );

    Ok(())
}
