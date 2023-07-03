mod utils;
mod workspaces;

use crate::utils::*;
use crate::workspaces::{
    build_contract, gen_user_account, get_block_timestamp, load_contract, transfer_near,
};
use kudos_contract::PROOF_OF_KUDOS_SBT_CLASS_ID;
use kudos_contract::{utils::*, KudosId};
use near_contract_standards::storage_management::StorageBalanceBounds;
use near_sdk::serde_json::{self, json, Value};
use near_sdk::{AccountId, ONE_NEAR, ONE_YOCTO};
use near_units::parse_near;
use std::collections::BTreeMap;

#[tokio::test]
async fn test_give_kudos() -> anyhow::Result<()> {
    let worker_mainnet = ::workspaces::mainnet_archival().await?;
    let near_social_id = "social.near".parse()?;
    let worker = ::workspaces::sandbox().await?;

    let admin_account = worker.root_account()?;

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

    // Initialize NDC i-am-human registry contract
    let iah_registry_id = "registry.i-am-human.near".parse()?;
    let iah_registry = worker
        .import_contract(&iah_registry_id, &worker_mainnet)
        .initial_balance(parse_near!("10000000 N"))
        .block_height(95_309_837)
        .transact()
        .await?;
    let _ = iah_registry
        .call("new")
        .args_json(json!({
          "authority": admin_account.id(),
          "iah_issuer": admin_account.id(),
          "iah_classes": [1]
        }))
        .max_gas()
        .transact()
        .await?
        .into_result()?;
    let _ = admin_account
        .call(&iah_registry_id, "admin_add_sbt_issuer")
        .args_json(json!({
          "issuer": admin_account.id()
        }))
        .max_gas()
        .transact()
        .await?
        .into_result()?;

    // Setup NDC Kudos Contract
    let kudos_contract = build_contract(
        &worker,
        "./",
        "init",
        json!({ "iah_registry": iah_registry_id }),
    )
    .await?;
    let balance_bounds: StorageBalanceBounds = near_social
        .view("storage_balance_bounds")
        .args_json(json!({}))
        .await?
        .json()?;
    let _ = kudos_contract
        .call("set_external_db")
        .args_json(json!({
            "external_db_id": near_social.id()
        }))
        .deposit(balance_bounds.min.0)
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

    let now_ms = get_block_timestamp(&worker).await? / 1_000_000;

    // Mint FV SBT for users & verify
    let minted_tokens: Vec<u64> = mint_fv_sbt(
        &iah_registry_id,
        &admin_account,
        &vec![user1_account.id(), user2_account.id(), user3_account.id()],
        now_ms,
        now_ms + 86_400_000,
    )
    .await?;
    assert!(verify_is_human(
        &iah_registry_id,
        admin_account.id(),
        &vec![&user1_account, &user2_account, &user3_account],
        &minted_tokens
    )
    .await
    .is_ok());

    /*
    // Test deposit BEGIN
    let balance_1: near_contract_standards::storage_management::StorageBalance = kudos_contract
        .as_account()
        .view(near_social.id(), "storage_balance_of")
        .args_json(json!({"account_id": kudos_contract.id()}))
        .await?
        .json()?;
    println!(
        "avail: {} total: {}",
        display_deposit_in_near(balance_1.available.0), display_deposit_in_near(balance_1.total.0)
    );

    let hashtags = (0..10)
        .map(|n| format!("{}{n}", "a".repeat(31)))
        .collect::<Vec<_>>();
    let kudos_text = "a".repeat(1000);
    let test1_account =
        gen_user_account(&worker, &[&"a".repeat(54), ".test.near"].concat()).await?;
    let _ = transfer_near(&worker, test1_account.id(), parse_near!("10 N")).await?;
    let test2_account =
        gen_user_account(&worker, &[&"b".repeat(54), ".test.near"].concat()).await?;
    let _ = transfer_near(&worker, test2_account.id(), parse_near!("10 N")).await?;

    // Mint FV SBT for users & verify
    let _ = mint_fv_sbt(
        &iah_registry_id,
        &admin_account,
        &vec![test1_account.id(), test2_account.id()],
        now_ms,
        now_ms + 86_400_000,
    )
    .await?;
    let res = give_kudos(
        kudos_contract.id(),
        &test1_account,
        test2_account.id(),
        &kudos_text,
        hashtags.iter().map(|s| s.as_str()).collect(),
    )
    .await;
    println!("{res:?}");

    let balance_2: near_contract_standards::storage_management::StorageBalance = kudos_contract
        .as_account()
        .view(near_social.id(), "storage_balance_of")
        .args_json(json!({"account_id": kudos_contract.id()}))
        .await?
        .json()?;
    println!(
        "avail: {} total: {}",
        display_deposit_in_near(balance_2.available.0), display_deposit_in_near(balance_2.total.0)
    );
    // Test deposit END
    */
    // User1 gives kudos to User2
    let hashtags = (0..3).map(|n| format!("ht{n}")).collect::<Vec<_>>();
    let kudos_text = "blablabla blablabla";
    let kudos_id = give_kudos(
        kudos_contract.id(),
        &user1_account,
        user2_account.id(),
        &kudos_text,
        hashtags.iter().map(|s| s.as_str()).collect(),
    )
    .await?;

    let get_kudos_by_id_req = build_get_kudos_by_id_request(
        &AccountId::new_unchecked(kudos_contract.id().to_string()),
        &AccountId::new_unchecked(user2_account.id().to_string()),
        &kudos_id,
    );

    let hashtags_req = format!("{}/hashtags/**", kudos_contract.id());

    // Verify kudos on NEAR Social-DB contract
    let mut kudos_data: near_sdk::serde_json::Value = user2_account
        .view(&near_social_id, "get")
        .args_json(json!({ "keys": [get_kudos_by_id_req, hashtags_req] }))
        .await?
        .json()?;
    // remove `created_at` nested key to be able compare with static stringified json and verify that removed key were exist
    assert!(remove_key_from_json(
        &mut kudos_data,
        &get_kudos_by_id_req.replace("*", "created_at")
    )
    .is_some());
    let extracted_hashtags = remove_key_from_json(
        &mut kudos_data,
        &format!("{}/hashtags", kudos_contract.id()),
    )
    .and_then(|val| serde_json::from_value::<BTreeMap<String, Value>>(val).ok())
    .map(|map| map.keys().cloned().collect::<Vec<_>>());
    assert_eq!(extracted_hashtags, Some(hashtags));

    // kudos referenced by id and account of User2
    //let kudos_reference = format!(r#"{{"{}":"{}"}}"#, kudos_id, user2_account.id());
    assert_eq!(
        kudos_data.to_string(),
        format!(
            r#"{{"{}":{{"kudos":{{"{}":{{"{kudos_id}":{{"sender_id":"{}","text":"{kudos_text}"}}}}}}}}}}"#,
            kudos_contract.id(),
            user2_account.id(),
            user1_account.id()
        )
    );

    // User3 upvotes kudos given to User2 by User1
    let _ = upvote_kudos(
        kudos_contract.id(),
        &user3_account,
        user2_account.id(),
        &kudos_id,
    )
    .await?;

    // Verify upvoted kudos on NEAR Social-DB contract
    let mut kudos_data: near_sdk::serde_json::Value = user2_account
        .view(&near_social_id, "get")
        .args_json(json!({
            "keys": [get_kudos_by_id_req.replace("*", "upvotes/**")]
        }))
        .await?
        .json()?;

    // remove `/upvotes` nested key and check for it's value, which should contain User3 who upvoted kudos
    let upvotes_json = remove_key_from_json(
        &mut kudos_data,
        &get_kudos_by_id_req.replace("*", "upvotes"),
    )
    .unwrap()
    .to_string();
    assert_eq!(upvotes_json, format!(r#"{{"{}":""}}"#, user3_account.id()));

    // User3 leaves a comment to kudos given to User2 by User1
    let _ = leave_comment(
        kudos_contract.id(),
        &user3_account,
        user2_account.id(),
        &kudos_id,
        "amazing",
    )
    .await?;

    // Verify comment left for kudos on NEAR Social-DB contract
    let mut kudos_data: near_sdk::serde_json::Value = user2_account
        .view(&near_social_id, "get")
        .args_json(json!({
            "keys": [get_kudos_by_id_req.replace("*", "comments/**")]
        }))
        .await?
        .json()?;

    // remove `/comments` nested key and check for it's value, which should contain User3 who left a comment and a text for kudos
    let upvotes_json = remove_key_from_json(
        &mut kudos_data,
        &get_kudos_by_id_req.replace("*", "comments"),
    )
    .unwrap()
    .to_string();
    assert_eq!(
        upvotes_json,
        format!(r#"{{"{}":"amazing"}}"#, user3_account.id())
    );

    Ok(())
}

#[tokio::test]
async fn test_mint_proof_of_kudos_sbt() -> anyhow::Result<()> {
    let worker_mainnet = ::workspaces::mainnet_archival().await?;
    let near_social_id = "social.near".parse()?;
    let worker = ::workspaces::sandbox().await?;

    let admin_account = worker.root_account()?;
    let iah_registry_id = "registry.i-am-human.near".parse()?;

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
    let kudos_contract = build_contract(
        &worker,
        "./",
        "init",
        json!({ "iah_registry": iah_registry_id }),
    )
    .await?;
    let balance_bounds: StorageBalanceBounds = near_social
        .view("storage_balance_bounds")
        .args_json(json!({}))
        .await?
        .json()?;
    let _ = kudos_contract
        .call("set_external_db")
        .args_json(json!({
            "external_db_id": near_social.id()
        }))
        .deposit(balance_bounds.min.0)
        .max_gas()
        .transact()
        .await?
        .into_result()?;

    // Initialize NDC i-am-human registry contract
    let iah_registry = worker
        .import_contract(&iah_registry_id, &worker_mainnet)
        .initial_balance(parse_near!("10000000 N"))
        .block_height(95_309_837)
        .transact()
        .await?;
    let _ = iah_registry
        .call("new")
        .args_json(json!({
          "authority": admin_account.id(),
          "iah_issuer": admin_account.id(),
          "iah_classes": [1]
        }))
        .max_gas()
        .transact()
        .await?
        .into_result()?;
    let _ = admin_account
        .call(&iah_registry_id, "admin_add_sbt_issuer")
        .args_json(json!({
          "issuer": admin_account.id()
        }))
        .max_gas()
        .transact()
        .await?
        .into_result()?;
    // Set Kudos contract as an SBT issuer
    let _ = admin_account
        .call(&iah_registry_id, "admin_add_sbt_issuer")
        .args_json(json!({
          "issuer": kudos_contract.id()
        }))
        .max_gas()
        .transact()
        .await?
        .into_result()?;

    // Register users' accounts
    let user1_account = gen_user_account(&worker, "user1.test.near").await?;
    let _ = transfer_near(&worker, user1_account.id(), parse_near!("10 N")).await?;

    let user2_account = gen_user_account(&worker, "user2.test.near").await?;
    let _ = transfer_near(&worker, user2_account.id(), parse_near!("10 N")).await?;

    let user3_account = gen_user_account(&worker, "user3.test.near").await?;
    let _ = transfer_near(&worker, user3_account.id(), parse_near!("10 N")).await?;

    let user4_account = gen_user_account(&worker, "user4.test.near").await?;
    let _ = transfer_near(&worker, user4_account.id(), parse_near!("10 N")).await?;

    let user5_account = gen_user_account(&worker, "user5.test.near").await?;
    let _ = transfer_near(&worker, user5_account.id(), parse_near!("10 N")).await?;

    let now_ms = get_block_timestamp(&worker).await? / 1_000_000;

    // Mint FV SBT for users
    let _ = mint_fv_sbt(
        &iah_registry_id,
        &admin_account,
        &vec![
            user1_account.id(),
            user2_account.id(),
            user3_account.id(),
            user4_account.id(),
            user5_account.id(),
        ],
        now_ms,
        now_ms + 86_400_000,
    )
    .await?;

    // User2 gives kudos to User1
    let kudos_id = give_kudos(
        kudos_contract.id(),
        &user2_account,
        user1_account.id(),
        "blablabla",
        vec!["hta", "htb"],
    )
    .await?;

    // User3 upvotes kudos for User1
    let _ = upvote_kudos(
        kudos_contract.id(),
        &user3_account,
        user1_account.id(),
        &kudos_id,
    )
    .await?;

    // User4 upvotes kudos for User1
    let _ = upvote_kudos(
        kudos_contract.id(),
        &user4_account,
        user1_account.id(),
        &kudos_id,
    )
    .await?;

    // User5 upvotes kudos for User1
    let _ = upvote_kudos(
        kudos_contract.id(),
        &user5_account,
        user1_account.id(),
        &kudos_id,
    )
    .await?;

    // User1 exchanges his Kudos for ProofOfKudos SBT
    let tokens_ids = exchange_kudos_for_sbt(kudos_contract.id(), &user1_account, &kudos_id).await?;
    assert_eq!(tokens_ids, vec![PROOF_OF_KUDOS_SBT_CLASS_ID]);

    let _ = verify_kudos_sbt_tokens_by_owner(
        &iah_registry_id,
        kudos_contract.id(),
        &user1_account,
        &tokens_ids,
    )
    .await?;

    Ok(())
}
