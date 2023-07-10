use crate::consts::{EXCHANGE_KUDOS_COST, EXCHANGE_KUDOS_STORAGE};
use crate::tests::utils::{build_default_context, promise_or_value_into_result, MAX_GAS};
use crate::types::MethodResult;
use crate::utils::{build_kudos_upvotes_path, display_deposit_requirement_in_near};
use crate::{Contract, IncrementalUniqueId, KudosId, PROOF_OF_KUDOS_SBT_MINT_COST};
use near_sdk::serde_json::{json, Value};
use near_sdk::test_utils::accounts;
use near_sdk::{
    env, serde_json, testing_env, AccountId, Gas, PromiseError, PromiseOrValue, ONE_NEAR, ONE_YOCTO,
};

#[test]
fn test_required_storage_to_exchange_kudos() {
    testing_env!(build_default_context(accounts(0), None, Some(Gas::ONE_TERA)).build());

    let mut kudos_contract = Contract::init(
        Some(accounts(0)),
        AccountId::new_unchecked("iah_registry.near".to_owned()),
    );

    let initial_storage = env::storage_usage();
    kudos_contract
        .exchanged_kudos
        .insert(IncrementalUniqueId::default().next().into());
    assert_eq!(
        env::storage_usage() - initial_storage,
        EXCHANGE_KUDOS_STORAGE
    );
}

#[test]
fn test_required_deposit_to_exchange_kudos() -> anyhow::Result<()> {
    let contract_id = AccountId::new_unchecked("kudos.near".to_owned());
    testing_env!(
        build_default_context(contract_id.clone(), None, Some(MAX_GAS),)
            .attached_deposit(EXCHANGE_KUDOS_COST)
            .build()
    );

    let initial_balance = env::account_balance();
    let mut kudos_contract = Contract::init(
        Some(contract_id.clone()),
        AccountId::new_unchecked("iah_registry.near".to_owned()),
    );

    let kudos_id = KudosId::from(IncrementalUniqueId::default().next());
    let receiver_id = accounts(0);
    let sender_id = accounts(1);
    let kudos_upvotes_path = build_kudos_upvotes_path(&contract_id, &receiver_id, &kudos_id);
    let _ = match kudos_contract.on_kudos_upvotes_acquired(
        sender_id,
        kudos_id,
        kudos_upvotes_path,
        Ok(json!({
            "kudos.near": {
              "kudos": {
                "alice": {
                  "1": {
                    "upvotes": {
                      "charlie": true,
                      "danny": true,
                      "eugene": true
                    }
                  }
                }
              }
            }
        })),
    ) {
        PromiseOrValue::Promise(_) => Ok(()),
        PromiseOrValue::Value(res) => Err(anyhow::Error::msg(format!("Unexpected result {res:?}"))),
    }?;
    assert_eq!(
        initial_balance - env::account_balance(),
        PROOF_OF_KUDOS_SBT_MINT_COST
    );

    Ok(())
}

#[test]
fn test_kudos_upvotes_acquire_errors() {
    let contract_id = AccountId::new_unchecked("kudos.near".to_owned());
    let context = build_default_context(contract_id.clone(), None, Some(MAX_GAS))
        .attached_deposit(EXCHANGE_KUDOS_COST)
        .build();

    let mut kudos_contract = Contract::init(
        Some(contract_id.clone()),
        AccountId::new_unchecked("iah_registry.near".to_owned()),
    );

    let kudos_id = KudosId::from(IncrementalUniqueId::default().next());
    let receiver_id = accounts(0);
    let sender_id = accounts(1);
    let kudos_upvotes_path = build_kudos_upvotes_path(&contract_id, &receiver_id, &kudos_id);

    struct TestCase<'a> {
        name: &'a str,
        input: Result<Value, PromiseError>,
        output: &'a str,
    }

    let test_cases = [
        TestCase {
            name: "Minimum upvotes requirement",
            input: Ok(json!({
                "kudos.near": {
                  "kudos": {
                    "alice": {
                      "1": {
                        "upvotes": {}
                      }
                    }
                  }
                }
            })),
            output: "Minimum required number (3) of upvotes has not been reached",
        },
        TestCase {
            name: "Upvotes parse failure",
            input: Ok(json!({
                "kudos.near": {
                  "kudos": {
                    "alice": {
                      "1": {
                        "upvotes": {
                          "test": "test"
                        }
                      }
                    }
                  }
                }
            })),
            output: "Failed to parse kudos upvotes data `Object {\"test\": String(\"test\")}`: Error(\"invalid type: string \\\"test\\\", expected a boolean\", line: 0, column: 0)",
        },
        TestCase {
            name: "Invalid response",
            input: Ok(json!({})),
            output: "No upvotes found for kudos: Object {}",
        },
        TestCase {
            name: "Promise error",
            input: Err(near_sdk::PromiseError::Failed),
            output: "SocialDB::keys(kudos.near/kudos/alice/1/upvotes/*) call failure: Failed",
        },
    ];

    for test_case in test_cases {
        testing_env!(context.clone());

        assert_eq!(
            promise_or_value_into_result(kudos_contract.on_kudos_upvotes_acquired(
                sender_id.clone(),
                kudos_id.clone(),
                kudos_upvotes_path.clone(),
                test_case.input,
            ))
            .unwrap_err()
            .as_str(),
            test_case.output,
            "Test case `{} failure`",
            test_case.name
        );
    }
}

#[test]
fn test_on_pok_sbt_mint() {
    let contract_id = AccountId::new_unchecked("kudos.near".to_owned());
    let context = build_default_context(contract_id.clone(), None, Some(MAX_GAS))
        .attached_deposit(EXCHANGE_KUDOS_COST)
        .build();

    let mut kudos_contract = Contract::init(
        Some(contract_id.clone()),
        AccountId::new_unchecked("iah_registry.near".to_owned()),
    );

    let sender_id = accounts(0);
    let kudos_id = KudosId::from(IncrementalUniqueId::default().next());

    struct TestCase<'a, T> {
        name: &'a str,
        input: Result<Vec<u64>, PromiseError>,
        output: MethodResult<T>,
    }

    let test_cases = [
        TestCase {
            name: "SBT mint successful",
            input: Ok(vec![1u64]),
            output: MethodResult::Success(vec![1u64]),
        },
        TestCase {
            name: "SBT mint failure",
            input: Ok(vec![]),
            output: MethodResult::Success(vec![]),
        },
        TestCase {
            name: "Promise error",
            input: Err(near_sdk::PromiseError::Failed),
            output: MethodResult::error("IAHRegistry::sbt_mint() call failure: Failed"),
        },
    ];

    for test_case in test_cases {
        testing_env!(context.clone());

        assert_eq!(
            kudos_contract.on_pok_sbt_mint(sender_id.clone(), kudos_id.clone(), test_case.input),
            test_case.output,
            "Test case `{} failure`",
            test_case.name
        );
    }
}
