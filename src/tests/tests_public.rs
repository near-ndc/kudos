use crate::consts::{EXCHANGE_KUDOS_COST, EXCHANGE_KUDOS_STORAGE};
use crate::tests::utils::{build_default_context, promise_or_value_into_result, MAX_GAS};
use crate::utils::{build_kudos_upvotes_path, display_deposit_requirement_in_near};
use crate::{Contract, KudosId, PROOF_OF_KUDOS_SBT_MINT_COST};
use near_sdk::serde_json::json;
use near_sdk::test_utils::accounts;
use near_sdk::{
    env, serde_json, testing_env, AccountId, Gas, PromiseError, PromiseOrValue, ONE_NEAR, ONE_YOCTO,
};

// #[test]
// fn test_required_storage_to_exchange_kudos() {
//     testing_env!(build_default_context(accounts(0), None, Some(Gas::ONE_TERA)).build());

//     let mut kudos_contract = Contract::init(
//         Some(accounts(0)),
//         AccountId::new_unchecked("iah_registry.near".to_owned()),
//     );

//     let initial_storage = env::storage_usage();
//     kudos_contract
//         .exchanged_kudos
//         .insert(KudosId::default().next());
//     assert_eq!(
//         env::storage_usage() - initial_storage,
//         EXCHANGE_KUDOS_STORAGE
//     );
// }

// #[test]
// fn test_required_deposit_to_exchange_kudos() -> anyhow::Result<()> {
//     let contract_id = AccountId::new_unchecked("kudos.near".to_owned());
//     testing_env!(
//         build_default_context(contract_id.clone(), None, Some(MAX_GAS),)
//             .attached_deposit(EXCHANGE_KUDOS_COST)
//             .build()
//     );

//     let initial_balance = env::account_balance();
//     let mut kudos_contract = Contract::init(
//         Some(contract_id.clone()),
//         AccountId::new_unchecked("iah_registry.near".to_owned()),
//     );

//     let kudos_id = KudosId::default().next();
//     let user_id = accounts(0);
//     let kudos_upvotes_path = build_kudos_upvotes_path(&contract_id, &user_id, &kudos_id);
//     let _ = kudos_contract
//         .send_sbt_mint_request(
//             kudos_id,
//             kudos_upvotes_path,
//             Ok(json!({
//                 "kudos.near": {
//                   "kudos": {
//                     "alice": {
//                       "1": {
//                         "upvotes": {
//                           "charlie": true,
//                           "danny": true,
//                           "eugene": true
//                         }
//                       }
//                     }
//                   }
//                 }
//             })),
//         )
//         .map_err(anyhow::Error::msg)?;
//     assert_eq!(
//         initial_balance - env::account_balance(),
//         PROOF_OF_KUDOS_SBT_MINT_COST
//     );

//     Ok(())
// }

// #[test]
// fn test_send_sbt_mint_request_errors() {
//     let contract_id = AccountId::new_unchecked("kudos.near".to_owned());
//     let context = build_default_context(contract_id.clone(), None, Some(MAX_GAS))
//         .attached_deposit(EXCHANGE_KUDOS_COST)
//         .build();

//     let mut kudos_contract = Contract::init(
//         Some(contract_id.clone()),
//         AccountId::new_unchecked("iah_registry.near".to_owned()),
//     );

//     let kudos_id = KudosId::default().next();
//     let user_id = accounts(0);
//     let kudos_upvotes_path = build_kudos_upvotes_path(&contract_id, &user_id, &kudos_id);

//     struct TestCase<'a> {
//         name: &'a str,
//         input: Result<serde_json::Value, PromiseError>,
//         output: &'a str,
//     }

//     let test_cases = [
//         TestCase {
//             name: "Minimum upvotes requirement",
//             input: Ok(json!({
//                 "kudos.near": {
//                   "kudos": {
//                     "alice": {
//                       "1": {
//                         "upvotes": {}
//                       }
//                     }
//                   }
//                 }
//             })),
//             output: "Minimum required number (3) of upvotes is not reached",
//         },
//         TestCase {
//             name: "Upvotes parse failure",
//             input: Ok(json!({
//                 "kudos.near": {
//                   "kudos": {
//                     "alice": {
//                       "1": {
//                         "upvotes": {
//                           "test": "test"
//                         }
//                       }
//                     }
//                   }
//                 }
//             })),
//             output: "Failed to parse upvotes data `Object {\"test\": String(\"test\")}`: Error(\"invalid type: string \\\"test\\\", expected a boolean\", line: 0, column: 0)",
//         },
//         TestCase {
//             name: "Invalid response",
//             input: Ok(json!({})),
//             output: "SocialDB::keys(kudos.near/kudos/alice/1/upvotes/*) invalid response Object {}",
//         },
//         TestCase {
//             name: "Promise error",
//             input: Err(near_sdk::PromiseError::Failed),
//             output: "SocialDB::keys(kudos.near/kudos/alice/1/upvotes/*) call failure: Failed",
//         },
//     ];

//     for test_case in test_cases {
//         testing_env!(context.clone());

//         assert_eq!(
//             kudos_contract
//                 .send_sbt_mint_request(
//                     kudos_id.clone(),
//                     kudos_upvotes_path.clone(),
//                     test_case.input,
//                 )
//                 .and_then(promise_or_value_into_result)
//                 .unwrap_err()
//                 .as_str(),
//             test_case.output,
//             "Test case `{} failure`",
//             test_case.name
//         );
//     }
// }

// #[test]
// fn test_on_pok_sbt_mint() {
//     let contract_id = AccountId::new_unchecked("kudos.near".to_owned());
//     let context = build_default_context(contract_id.clone(), None, Some(MAX_GAS))
//         .attached_deposit(EXCHANGE_KUDOS_COST)
//         .build();

//     let mut kudos_contract = Contract::init(
//         Some(contract_id.clone()),
//         AccountId::new_unchecked("iah_registry.near".to_owned()),
//     );

//     let kudos_id = KudosId::default().next();

//     struct TestCase<'a, T> {
//         name: &'a str,
//         input: Result<Vec<u64>, PromiseError>,
//         output: PromiseOrValue<T>,
//     }

//     let test_cases = [
//         TestCase {
//             name: "SBT mint successful",
//             input: Ok(vec![1u64]),
//             output: PromiseOrValue::Value(vec![1u64]),
//         },
//         TestCase {
//             name: "SBT mint failure",
//             input: Ok(vec![]),
//             output: PromiseOrValue::Value(vec![]),
//         },
//         TestCase {
//             name: "Promise error",
//             input: Err(near_sdk::PromiseError::Failed),
//             output: PromiseOrValue::Value(vec![]),
//         },
//     ];

//     for test_case in test_cases {
//         testing_env!(context.clone());

//         assert_eq!(
//             promise_or_value_into_result(
//                 kudos_contract.on_pok_sbt_mint(kudos_id.clone(), test_case.input,)
//             ),
//             promise_or_value_into_result(test_case.output),
//             "Test case `{} failure`",
//             test_case.name
//         );
//     }
// }
