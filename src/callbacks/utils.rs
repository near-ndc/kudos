use crate::utils::*;
use near_sdk::serde_json::{self, Value};
use near_sdk::{AccountId, PromiseError};
use std::collections::HashMap;

pub fn parse_kudos_and_verify_upvotes(
    res: Result<Value, PromiseError>,
    kudos_upvotes_path: String,
    required_min_number_of_upvotes: usize,
) -> Result<(), String> {
    let mut kudos_json =
        res.map_err(|e| format!("SocialDB::keys({kudos_upvotes_path}/*) call failure: {e:?}"))?;

    let upvotes_raw = remove_key_from_json(&mut kudos_json, &kudos_upvotes_path)
        .ok_or_else(|| format!("No upvotes found for kudos: {kudos_json:?}"))?;

    let upvoters = serde_json::from_value::<HashMap<AccountId, bool>>(upvotes_raw.clone())
        .map_err(|e| format!("Failed to parse kudos upvotes data `{upvotes_raw:?}`: {e:?}"))?;

    let number_of_upvotes = upvoters.keys().len();

    if number_of_upvotes < required_min_number_of_upvotes {
        Err(format!(
            "Minimum required number ({}) of upvotes has not been reached",
            required_min_number_of_upvotes
        ))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::build_kudos_upvotes_path;
    use crate::{IncrementalUniqueId, KudosId};
    use near_sdk::serde_json::json;
    use near_sdk::test_utils::accounts;

    #[test]
    fn test_parse_kudos_and_verify_upvotes() {
        let root_id = AccountId::new_unchecked("kudos.near".to_owned());
        let kudos_id = KudosId::from(IncrementalUniqueId::default().next());
        let receiver_id = accounts(0);
        let sender_id = accounts(1);
        let kudos_upvotes_path = build_kudos_upvotes_path(&root_id, &receiver_id, &kudos_id);

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
            assert_eq!(
                parse_kudos_and_verify_upvotes(test_case.input, kudos_upvotes_path.clone(), 3)
                    .unwrap_err()
                    .as_str(),
                test_case.output,
                "Test case `{} failure`",
                test_case.name
            );
        }
    }
}
