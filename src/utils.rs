use crate::types::KudosId;
use near_sdk::serde_json::{self, Value};
use near_sdk::AccountId;

pub fn build_hashtags(
    receiver_id: &AccountId,
    kudos_id: &KudosId,
    hashtags: Option<Vec<String>>,
) -> Result<String, &'static str> {
    hashtags
        .map(|hashtags| {
            hashtags
                .into_iter()
                .map(|ht| {
                    // TODO: verify hashtag for valid symbols (a-z)
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

pub fn build_give_kudos_request(
    root_id: &AccountId,
    sender_id: &AccountId,
    receiver_id: &AccountId,
    kudos_id: &KudosId,
    created_at: u64,
    text: &str,
    hashtags: &str,
) -> Result<Value, &'static str> {
    serde_json::from_str::<Value>(&format!(
        r#"{{
          "{root_id}": {{
            "kudos": {{
              "{receiver_id}": {{
                "{kudos_id}": {{
                  "created_at": "{created_at}",
                  "sender_id": "{sender_id}",
                  "text": "{text}",
                  "upvotes": {{}},
                  "comments": {{}}
                }}
              }}
            }},
            "hashtags": {hashtags}
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
    sender_id: &AccountId,
    receiver_id: &AccountId,
    kudos_id: &KudosId,
    text: &str,
) -> Result<Value, &'static str> {
    serde_json::from_str::<Value>(&format!(
        r#"{{
          "{root_id}": {{
            "kudos": {{
              "{receiver_id}": {{
                "{kudos_id}": {{
                  "comments": {{
                    "{sender_id}": "{text}"
                  }}
                }}
              }}
            }}
          }}
        }}"#
    ))
    .map_err(|_| "Internal serialization error")
}

pub fn build_verify_kudos_id_request(
    root_id: &AccountId,
    receiver_id: &AccountId,
    kudos_id: &KudosId,
) -> String {
    format!("{root_id}/kudos/{receiver_id}/{kudos_id}")
}

#[cfg(test)]
mod tests {
    use crate::KudosId;
    use near_sdk::serde_json;
    use near_sdk::AccountId;

    #[test]
    fn test_build_hashtags() {
        let receiver_id = AccountId::new_unchecked("test2.near".to_owned());
        let next_kudos_id = KudosId::default().next();

        let json_text = super::build_hashtags(
            &receiver_id,
            &next_kudos_id,
            Some(vec![
                "hashtaga".to_owned(),
                "hashtagb".to_owned(),
                "hashtagc".to_owned(),
            ]),
        )
        .unwrap();

        assert_eq!(
            json_text,
            r#"{"hashtaga":{"1":"test2.near"},"hashtagb":{"1":"test2.near"},"hashtagc":{"1":"test2.near"}}"#
        );
    }

    #[test]
    fn test_build_kudos_request() {
        let root_id = AccountId::new_unchecked("kudos.near".to_owned());
        let sender_id = AccountId::new_unchecked("test1.near".to_owned());
        let receiver_id = AccountId::new_unchecked("test2.near".to_owned());
        let next_kudos_id = KudosId::default().next();
        let text = "blablabla";

        let json_text = serde_json::to_string(
            &super::build_give_kudos_request(
                &root_id,
                &sender_id,
                &receiver_id,
                &next_kudos_id,
                1234567890u64,
                text,
                "{}",
            )
            .unwrap(),
        )
        .unwrap();

        assert_eq!(
            json_text,
            r#"{"kudos.near":{"hashtags":{},"kudos":{"test2.near":{"1":{"comments":{},"created_at":"1234567890","sender_id":"test1.near","text":"blablabla","upvotes":{}}}}}}"#
        );
    }
}
