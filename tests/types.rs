use kudos_contract::{Commentary, EscapedMessage};
use near_sdk::json_types::U64;
use near_sdk::serde::{self, Deserialize};
use near_sdk::{serde_json, AccountId};

#[derive(Debug, PartialEq)]
pub struct CommentaryRaw {
    pub message: EscapedMessage,
    pub sender_id: AccountId,
    pub timestamp: U64,
}

impl<'a> From<&'a CommentaryRaw> for Commentary<'a> {
    fn from(value: &'a CommentaryRaw) -> Self {
        Self {
            message: &value.message,
            sender_id: &value.sender_id,
            timestamp: value.timestamp,
        }
    }
}

impl<'de> Deserialize<'de> for CommentaryRaw {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let encoded = <String as Deserialize>::deserialize(deserializer)?;

        let raw = near_sdk::base64::decode(&encoded).map_err(|e| {
            serde::de::Error::custom(format!(
                "Unable to deserialize commentary from base64 encoded data: {encoded}. {e:?}"
            ))
        })?;

        serde_json::from_slice::<serde_json::Value>(&raw)
            .map_err(|e| {
                serde::de::Error::custom(format!(
                    "Unable to deserialize commentary json from decoded data: {encoded}. {e:?}"
                ))
            })?
            .as_object_mut()
            .and_then(|map| {
                let message = map
                    .remove("m")
                    .and_then(|v| serde_json::from_value::<String>(v).ok())?;
                let sender_id = map
                    .remove("s")
                    .and_then(|v| serde_json::from_value::<AccountId>(v).ok())?;
                let timestamp = map
                    .remove("t")
                    .and_then(|v| serde_json::from_value::<U64>(v).ok())?;

                Some(Self {
                    sender_id,
                    message: EscapedMessage::new_unchecked(&message),
                    timestamp,
                })
            })
            .ok_or_else(|| serde::de::Error::custom("Failure to deserialize commentary from json"))
    }
}
