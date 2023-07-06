use kudos_contract::Commentary;
use near_sdk::serde::{self, Deserialize};
use near_sdk::{serde_json, AccountId};

#[derive(Debug, PartialEq)]
pub struct CommentaryRaw {
    pub sender_id: AccountId,
    pub text: String,
}

impl<'a> From<&'a CommentaryRaw> for Commentary<'a> {
    fn from(value: &'a CommentaryRaw) -> Self {
        Self {
            sender_id: &value.sender_id,
            text: &value.text,
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
                let sender_id = map
                    .remove("s")
                    .and_then(|s| serde_json::from_value::<AccountId>(s).ok())?;
                let text = map
                    .remove("t")
                    .and_then(|s| serde_json::from_value::<String>(s).ok())?;

                Some(Self { sender_id, text })
            })
            .ok_or_else(|| serde::de::Error::custom("Failure to deserialize commentary from json"))
    }
}
