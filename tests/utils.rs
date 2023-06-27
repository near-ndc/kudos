use near_sdk::serde_json::Value;
use near_sdk::{test_utils::VMContextBuilder, AccountId, Balance, Gas};

pub const MAX_GAS: Gas = Gas(300_000_000_000_000);

pub fn build_default_context(
    predecessor_account_id: AccountId,
    deposit: Option<Balance>,
    prepaid_gas: Option<Gas>,
) -> VMContextBuilder {
    let mut builder = VMContextBuilder::new();
    builder
        .signer_account_id(predecessor_account_id.clone())
        .predecessor_account_id(predecessor_account_id)
        .prepaid_gas(prepaid_gas.unwrap_or(MAX_GAS))
        .attached_deposit(deposit.unwrap_or_default());
    builder
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

#[cfg(test)]
mod tests {
    use super::remove_key_from_json;
    use near_sdk::serde_json::json;

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
}
