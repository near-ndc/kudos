use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::json_types::U64;
use near_sdk::serde::{self, Deserialize, Serialize};
use near_sdk::serde_json::Value;
use near_sdk::{
    serde_json::{self, json},
    AccountId, BorshStorageKey,
};
use std::fmt::Display;
use std::hash::{Hash, Hasher};

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone, Debug)]
#[serde(crate = "near_sdk::serde")]
pub struct IncrementalUniqueId(U64);

impl IncrementalUniqueId {
    pub fn as_u64(&self) -> u64 {
        self.0 .0
    }

    pub fn inc(&mut self) -> &Self {
        self.0 = self.next().0;
        self
    }

    pub fn next(&self) -> Self {
        Self((self.as_u64() + 1).into())
    }
}

impl Default for IncrementalUniqueId {
    fn default() -> Self {
        Self(0.into())
    }
}

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone, Debug)]
#[serde(crate = "near_sdk::serde")]
pub struct KudosId(U64);

impl From<IncrementalUniqueId> for KudosId {
    fn from(value: IncrementalUniqueId) -> Self {
        Self(value.0)
    }
}

impl From<&IncrementalUniqueId> for KudosId {
    fn from(value: &IncrementalUniqueId) -> Self {
        Self(value.0)
    }
}

impl Display for KudosId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.0 .0, f)
    }
}

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone, Debug, Eq, PartialEq)]
#[serde(crate = "near_sdk::serde")]
pub struct CommentId(U64);

impl Hash for CommentId {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0 .0.hash(state);
    }
}

impl From<IncrementalUniqueId> for CommentId {
    fn from(value: IncrementalUniqueId) -> Self {
        Self(value.0)
    }
}

impl From<&IncrementalUniqueId> for CommentId {
    fn from(value: &IncrementalUniqueId) -> Self {
        Self(value.0)
    }
}

impl Display for CommentId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.0 .0, f)
    }
}

#[derive(BorshStorageKey, BorshSerialize)]
pub(crate) enum StorageKey {
    Kudos,
}

#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct PromiseFunctionCall {
    pub contract_id: AccountId,
    pub function_name: String,
    pub arguments: Vec<u8>,
    pub attached_deposit: Option<near_sdk::Balance>,
    pub static_gas: near_sdk::Gas,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(crate = "near_sdk::serde", tag = "status", content = "result")]
pub enum MethodResult<T> {
    Success(T),
    Error(String),
}

impl<T> MethodResult<T> {
    pub fn error<E: ToString>(err: E) -> Self {
        Self::Error(err.to_string())
    }
}

#[derive(Debug, PartialEq)]
pub struct Commentary<'a> {
    pub message: &'a str,
    pub sender_id: &'a AccountId,
    pub timestamp: U64,
}

impl<'a> Commentary<'_> {
    pub fn compose(&self) -> Result<String, String> {
        serde_json::to_value(&self)
            .and_then(|val| {
                val.as_str()
                    .map(str::to_string)
                    .ok_or(serde::ser::Error::custom("Not a string"))
            })
            .map_err(|e| format!("Unable to compose commentary. Error: {e}"))
    }
}

impl Serialize for Commentary<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: near_sdk::serde::Serializer,
    {
        let encoded = near_sdk::base64::encode(
            json!({
                "m": self.message,
                "s": self.sender_id,
                "t": self.timestamp
            })
            .to_string(),
        );

        serializer.serialize_str(&encoded)
    }
}

#[derive(
    BorshDeserialize,
    BorshSerialize,
    Serialize,
    Deserialize,
    Clone,
    Debug,
    Ord,
    PartialOrd,
    Eq,
    PartialEq,
)]
#[serde(crate = "near_sdk::serde")]
pub struct Hashtag(String);

impl TryFrom<&String> for Hashtag {
    type Error = &'static str;

    fn try_from(value: &String) -> Result<Self, Self::Error> {
        Self::try_from(value.as_str())
    }
}

impl TryFrom<&str> for Hashtag {
    type Error = &'static str;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        if value.contains(|c: char| !c.is_ascii_alphanumeric()) {
            return Err("Non-alphanumeric characters are not allowed in hashtag");
        }

        Ok(Self(value.to_owned()))
    }
}

#[cfg(test)]
mod tests {
    use crate::{Commentary, Hashtag};
    use near_sdk::json_types::U64;
    use near_sdk::AccountId;

    #[test]
    fn test_commentary_ser() {
        let comment = Commentary {
            sender_id: &AccountId::new_unchecked("user.near".to_owned()),
            message: "commentary test",
            timestamp: U64(1234567890),
        }
        .compose()
        .unwrap();
        assert_eq!(
            comment.as_str(),
            "eyJtIjoiY29tbWVudGFyeSB0ZXN0IiwicyI6InVzZXIubmVhciIsInQiOiIxMjM0NTY3ODkwIn0="
        );
    }

    #[test]
    fn test_hashtag_from_str() {
        assert!(Hashtag::try_from("validhashtag").is_ok());
        assert!(Hashtag::try_from("val1dhAshta9").is_ok());
        assert!(Hashtag::try_from("invalid_hashtag").is_err());
        assert!(Hashtag::try_from("invalidha$ht@g").is_err());
    }
}
