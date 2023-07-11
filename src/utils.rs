use crate::consts::PROOF_OF_KUDOS_SBT_CLASS_ID;
use crate::registry::TokenMetadata;
use crate::types::{IncrementalUniqueId, KudosId};
use crate::{CommentId, Commentary, EncodedCommentary, EscapedMessage, Hashtag};
use near_sdk::env::STORAGE_PRICE_PER_BYTE;
use near_sdk::serde_json::{self, Value};
use near_sdk::{AccountId, Balance, Gas};

pub fn build_hashtags(
    receiver_id: &AccountId,
    kudos_id: &KudosId,
    hashtags: Option<&[Hashtag]>,
) -> Result<String, &'static str> {
    hashtags
        .map(|hashtags| {
            hashtags
                .into_iter()
                .map(|ht| {
                    serde_json::from_str::<Value>(&format!(
                        r#"{{
                          "{kudos_id}": "{receiver_id}"
                        }}"#
                    ))
                    .map(|v| (ht, v))
                })
                .collect::<Result<std::collections::BTreeMap<_, _>, _>>()
                .and_then(|map| serde_json::to_string(&map))
                .map_err(|_| "Internal serialization error")
        })
        .unwrap_or_else(|| Ok("{}".to_owned()))
}

pub fn hashtags_to_json_array(hashtags: &[Hashtag]) -> Result<String, &'static str> {
    serde_json::to_string(&hashtags)
        .map(|s| s.escape_default().to_string())
        .map_err(|_| "Internal hashtags serialization error")
}

pub fn build_give_kudos_request(
    root_id: &AccountId,
    sender_id: &AccountId,
    receiver_id: &AccountId,
    kudos_id: &KudosId,
    created_at: u64,
    message: &EscapedMessage,
    hashtags: Option<&[Hashtag]>,
) -> Result<Value, &'static str> {
    let hashtags_as_array_json = hashtags_to_json_array(hashtags.as_deref().unwrap_or(&[]))?;
    let hashtags_with_kudos = build_hashtags(receiver_id, kudos_id, hashtags)?;

    serde_json::from_str::<Value>(&format!(
        r#"{{
          "{root_id}": {{
            "kudos": {{
              "{receiver_id}": {{
                "{kudos_id}": {{
                  "created_at": "{created_at}",
                  "sender_id": "{sender_id}",
                  "message": "{message}",
                  "upvotes": {{}},
                  "comments": {{}},
                  "tags": "{hashtags_as_array_json}"
                }}
              }}
            }},
            "hashtags": {hashtags_with_kudos}
          }}
        }}"#
    ))
    .map_err(|_| "Internal serialization error")
}

pub fn build_upvote_kudos_request(
    root_id: &AccountId,
    sender_id: &AccountId,
    receiver_id: &AccountId,
    kudos_id: &KudosId,
) -> Result<Value, &'static str> {
    serde_json::from_str::<Value>(&format!(
        r#"{{
          "{root_id}": {{
            "kudos": {{
              "{receiver_id}": {{
                "{kudos_id}": {{
                  "upvotes": {{
                    "{sender_id}": ""
                  }}
                }}
              }}
            }}
          }}
        }}"#
    ))
    .map_err(|_| "Internal serialization error")
}

pub fn build_leave_comment_request(
    root_id: &AccountId,
    receiver_id: &AccountId,
    kudos_id: &KudosId,
    comment_id: &CommentId,
    comment: &EncodedCommentary,
) -> Result<Value, &'static str> {
    let comment = comment.as_str();
    let json = format!(
        r#"{{
          "{root_id}": {{
            "kudos": {{
              "{receiver_id}": {{
                "{kudos_id}": {{
                  "comments": {{
                    "{comment_id}": "{comment}"
                  }}
                }}
              }}
            }}
          }}
        }}"#
    );
    serde_json::from_str::<Value>(&json).map_err(|e| "Internal serialization error")
}

pub fn build_get_kudos_by_id_request(
    root_id: &AccountId,
    receiver_id: &AccountId,
    kudos_id: &KudosId,
) -> String {
    format!("{root_id}/kudos/{receiver_id}/{kudos_id}/*")
}

pub fn build_kudos_upvotes_path(
    root_id: &AccountId,
    receiver_id: &AccountId,
    kudos_id: &KudosId,
) -> String {
    format!("{root_id}/kudos/{receiver_id}/{kudos_id}/upvotes")
}

pub fn build_pok_sbt_metadata(issued_at: u64, expires_at: u64) -> TokenMetadata {
    TokenMetadata {
        class: PROOF_OF_KUDOS_SBT_CLASS_ID,
        issued_at: Some(issued_at),
        expires_at: Some(expires_at),
        reference: None,
        reference_hash: None,
    }
}

pub fn extract_kudos_id_sender_from_response(req: &str, mut res: Value) -> Option<AccountId> {
    remove_key_from_json(&mut res, &req.replace("*", "sender_id"))
        .and_then(|val| serde_json::from_value::<AccountId>(val).ok())
}

pub fn remove_key_from_json(json: &mut Value, key: &str) -> Option<Value> {
    let mut json = Some(json);
    let mut keys = key.split("/").peekable();

    while let Some(key) = keys.next() {
        match json {
            Some(Value::Object(obj)) if keys.peek().is_none() => {
                return obj.remove(key);
            }
            Some(Value::Object(obj)) => json = obj.get_mut(key),
            _ => break,
        }
    }

    None
}

/// Checks if provided value of type T is equal to T::default()
pub(crate) fn is_default<T: Default + PartialEq>(t: &T) -> bool {
    t == &T::default()
}

pub(crate) fn opt_default<T>() -> Option<T> {
    Option::<T>::None
}

pub(crate) fn display_gas_requirement_in_tgas(gas: Gas) -> String {
    format!(
        "Requires minimum amount of attached gas {} TGas",
        gas.0 / Gas::ONE_TERA.0
    )
}

pub(crate) fn display_deposit_requirement_in_near(value: Balance) -> String {
    format!(
        "Requires exact amount of attached deposit {} NEAR",
        (value / STORAGE_PRICE_PER_BYTE) as f64 / 100_000f64
    )
}

pub fn display_deposit_in_near(value: Balance) -> String {
    format!(
        "{} NEAR",
        (value / STORAGE_PRICE_PER_BYTE) as f64 / 100_000f64
    )
}

#[cfg(test)]
mod tests {
    use crate::EncodedCommentary;

    use super::*;
    use near_sdk::json_types::U64;
    use near_sdk::serde_json::json;
    use near_units::parse_near;

    #[test]
    fn test_build_hashtags() {
        let receiver_id = AccountId::new_unchecked("test2.near".to_owned());
        let next_kudos_id = KudosId::from(IncrementalUniqueId::default().next());

        let json_text = super::build_hashtags(
            &receiver_id,
            &next_kudos_id,
            Some(&vec![
                Hashtag::try_from("hashtaga").unwrap(),
                Hashtag::try_from("hashtagb").unwrap(),
                Hashtag::try_from("hashtagc").unwrap(),
            ]),
        )
        .unwrap();

        assert_eq!(
            json_text,
            r#"{"hashtaga":{"1":"test2.near"},"hashtagb":{"1":"test2.near"},"hashtagc":{"1":"test2.near"}}"#
        );
    }

    #[test]
    fn test_hashtags_to_json_array() {
        assert_eq!(
            hashtags_to_json_array(&[
                Hashtag::try_from("a1").unwrap(),
                Hashtag::try_from("b1").unwrap(),
                Hashtag::try_from("c1").unwrap(),
            ])
            .unwrap(),
            r#"[\"a1\",\"b1\",\"c1\"]"#
        );
        assert_eq!(hashtags_to_json_array(&[]).unwrap(), r#"[]"#);
    }

    #[test]
    fn test_build_kudos_request() {
        let root_id = AccountId::new_unchecked("kudos.near".to_owned());
        let sender_id = AccountId::new_unchecked("test1.near".to_owned());
        let receiver_id = AccountId::new_unchecked("test2.near".to_owned());
        let next_kudos_id = KudosId::from(IncrementalUniqueId::default().next());
        let message = EscapedMessage::new(r#""a","b":{"t":"multi\nline"},"#, 1000).unwrap();

        let json_text = serde_json::to_string(
            &super::build_give_kudos_request(
                &root_id,
                &sender_id,
                &receiver_id,
                &next_kudos_id,
                1234567890u64,
                &message,
                Some(&[
                    Hashtag::try_from("abc").unwrap(),
                    Hashtag::try_from("def").unwrap(),
                ]),
            )
            .unwrap(),
        )
        .unwrap();

        assert_eq!(
            json_text,
            r#"{"kudos.near":{"hashtags":{"abc":{"1":"test2.near"},"def":{"1":"test2.near"}},"kudos":{"test2.near":{"1":{"comments":{},"created_at":"1234567890","message":"\"a\",\"b\":{\"t\":\"multi\\nline\"},","sender_id":"test1.near","tags":"[\"abc\",\"def\"]","upvotes":{}}}}}}"#
        );
    }

    #[test]
    fn test_build_upvote_kudos_request() {
        let root_id = AccountId::new_unchecked("kudos.near".to_owned());
        let sender_id = AccountId::new_unchecked("test1.near".to_owned());
        let receiver_id = AccountId::new_unchecked("test2.near".to_owned());
        let next_kudos_id = KudosId::from(IncrementalUniqueId::default().next());

        let json_text = serde_json::to_string(
            &super::build_upvote_kudos_request(&root_id, &sender_id, &receiver_id, &next_kudos_id)
                .unwrap(),
        )
        .unwrap();

        assert_eq!(
            json_text,
            r#"{"kudos.near":{"kudos":{"test2.near":{"1":{"upvotes":{"test1.near":""}}}}}}"#
        );
    }

    #[test]
    fn test_build_leave_comment_request() {
        let root_id = AccountId::new_unchecked("kudos.near".to_owned());
        let sender_id = AccountId::new_unchecked("test1.near".to_owned());
        let receiver_id = AccountId::new_unchecked("test2.near".to_owned());
        let mut unique_id = IncrementalUniqueId::default();
        let kudos_id = KudosId::from(unique_id.inc());
        let comment_id = CommentId::from(unique_id.inc());

        let json_text = serde_json::to_string(
            &super::build_leave_comment_request(
                &root_id,
                &receiver_id,
                &kudos_id,
                &comment_id,
                &EncodedCommentary::try_from(&Commentary {
                    sender_id: &sender_id,
                    message: &EscapedMessage::new("some commentary text", 1000).unwrap(),
                    timestamp: U64(1234567890),
                })
                .unwrap(),
            )
            .unwrap(),
        )
        .unwrap();

        assert_eq!(
            json_text,
            r#"{"kudos.near":{"kudos":{"test2.near":{"1":{"comments":{"2":"eyJtIjoic29tZSBjb21tZW50YXJ5IHRleHQiLCJzIjoidGVzdDEubmVhciIsInQiOiIxMjM0NTY3ODkwIn0="}}}}}}"#
        );
    }

    #[test]
    fn test_build_verify_kudos_id_request() {
        let root_id = AccountId::new_unchecked("kudos.near".to_owned());
        let receiver_id = AccountId::new_unchecked("test2.near".to_owned());
        let next_kudos_id = KudosId::from(IncrementalUniqueId::default().next());
        assert_eq!(
            &super::build_get_kudos_by_id_request(&root_id, &receiver_id, &next_kudos_id),
            "kudos.near/kudos/test2.near/1/*"
        );
    }

    #[test]
    fn test_verify_kudos_id_response() {
        // valid kudos response
        assert_eq!(
            super::extract_kudos_id_sender_from_response(
                "test.near/kudos/user1.near/1/*",
                json!({
                    "test.near": {
                      "kudos": {
                        "user1.near": {
                          "1": {
                            "sender_id": "user2.near"
                          }
                        }
                      }
                    }
                })
            ),
            Some(AccountId::new_unchecked("user2.near".to_owned()))
        );
        // invalid kudos response
        assert!(super::extract_kudos_id_sender_from_response(
            "test.near/kudos/user1.near/1/*",
            json!({
                "test.near": {
                  "kudos": {
                    "user1.near": {
                      "1": {}
                    }
                  }
                }
            })
        )
        .is_none());
        // different kudos root id
        assert!(super::extract_kudos_id_sender_from_response(
            "test.near/kudos/user1.near/1/*",
            json!({
                "test1.near": {
                  "kudos": {
                    "user1.near": {
                      "1": {
                        "sender_id": "user2.near"
                      }
                    }
                  }
                }
            })
        )
        .is_none());
        // no response
        assert!(super::extract_kudos_id_sender_from_response(
            "test.near/kudos/user1.near/1/*",
            json!({})
        )
        .is_none());
    }

    #[test]
    fn test_remove_key_from_json() {
        let mut json = json!({
            "abc": "test",
            "remove_me": "test2",
            "test": {
                "remove_me": "test3",
                "test1": {
                    "remove_me": "test4"
                }
            }
        });

        // key not exist or nothing to remove
        assert!(remove_key_from_json(&mut json, "").is_none());
        assert!(remove_key_from_json(&mut json, "testtest").is_none());
        assert!(remove_key_from_json(&mut json, "test_abc/test_def").is_none());
        assert_eq!(
            json.to_string(),
            r#"{"abc":"test","remove_me":"test2","test":{"remove_me":"test3","test1":{"remove_me":"test4"}}}"#
        );
        // remove key from root
        assert_eq!(
            remove_key_from_json(&mut json, "remove_me"),
            Some(json!("test2"))
        );
        assert_eq!(
            json.to_string(),
            r#"{"abc":"test","test":{"remove_me":"test3","test1":{"remove_me":"test4"}}}"#
        );
        // remove nested key
        assert_eq!(
            remove_key_from_json(&mut json, "test/remove_me"),
            Some(json!("test3"))
        );
        assert_eq!(
            json.to_string(),
            r#"{"abc":"test","test":{"test1":{"remove_me":"test4"}}}"#
        );
        // remove deeply nested key
        assert_eq!(
            remove_key_from_json(&mut json, "test/test1/remove_me"),
            Some(json!("test4"))
        );
        assert_eq!(json.to_string(), r#"{"abc":"test","test":{"test1":{}}}"#);
    }

    #[test]
    fn test_display_deposit_requirement_in_near() {
        assert_eq!(
            display_deposit_requirement_in_near(parse_near!("0.0005 NEAR")).as_str(),
            "Requires exact amount of attached deposit 0.0005 NEAR"
        );
        assert_eq!(
            display_deposit_requirement_in_near(parse_near!("0.00051 NEAR")).as_str(),
            "Requires exact amount of attached deposit 0.00051 NEAR"
        );
        assert_eq!(
            display_deposit_requirement_in_near(parse_near!("0.000553 NEAR")).as_str(),
            "Requires exact amount of attached deposit 0.00055 NEAR"
        );
    }
}
