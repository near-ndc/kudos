use crate::types::KudosId;
use near_sdk::serde_json::{self, Value};
use near_sdk::AccountId;

pub fn build_add_kudos_request(
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
                "sender_id": "{sender_id}",
                "text": "{text}",
                "upvotes": {{}}
              }}
            }}
          }}
        }}
      }}"#
    ))
    .map_err(|_| "Internal serialization error")
}

#[cfg(test)]
mod tests {
    use crate::KudosId;
    use near_sdk::serde_json;
    use near_sdk::AccountId;

    #[test]
    fn test_build_kudos_request() {
        let root_id = AccountId::new_unchecked("kudos.near".to_owned());
        let sender_id = AccountId::new_unchecked("test1.near".to_owned());
        let receiver_id = AccountId::new_unchecked("test2.near".to_owned());
        let next_kudos_id = KudosId::default().next();
        let text = "blablabla";

        let json_text = serde_json::to_string(
            &super::build_add_kudos_request(
                &root_id,
                &sender_id,
                &receiver_id,
                &next_kudos_id,
                text,
            )
            .unwrap(),
        )
        .unwrap();

        assert_eq!(
            json_text,
            r#"{"kudos.near":{"kudos":{"test2.near":{"1":{"sender_id":"test1.near","text":"blablabla","upvotes":{}}}}}}"#
        );
    }
}
