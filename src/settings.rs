use crate::utils::opt_default;
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::require;
use near_sdk::serde::{Deserialize, Serialize};

#[derive(BorshDeserialize, BorshSerialize, Debug, Clone)]
pub struct Settings {
    pub commentary_text_max_length: u16,
    pub max_number_of_hashtags_per_kudos: u8,
    pub hashtag_text_max_length: u8,
    pub min_number_of_upvotes_to_exchange_kudos: u8,
    pub pok_sbt_ttl: u64,
}

#[derive(BorshDeserialize, BorshSerialize, Debug)]
pub enum VSettings {
    // Add old versions here, keep ordering, the oldest on top, most recent at bottom
    // e.g. V0(SettingsV0),
    Current(Settings), // most recent version
}

/// View JSON serializable representation of `Settings` data struct
#[derive(Default, Deserialize, Serialize, Clone)]
#[serde(crate = "near_sdk::serde", rename_all = "camelCase")]
pub struct SettingsView {
    #[serde(default = "opt_default", skip_serializing_if = "Option::is_none")]
    pub commentary_text_max_length: Option<u16>,
    #[serde(default = "opt_default", skip_serializing_if = "Option::is_none")]
    pub max_number_of_hashtags_per_kudos: Option<u8>,
    #[serde(default = "opt_default", skip_serializing_if = "Option::is_none")]
    pub hashtag_text_max_length: Option<u8>,
    #[serde(default = "opt_default", skip_serializing_if = "Option::is_none")]
    pub min_number_of_upvotes_to_exchange_kudos: Option<u8>,
    #[serde(default = "opt_default", skip_serializing_if = "Option::is_none")]
    pub pok_sbt_ttl: Option<u64>,
}

impl Settings {
    /// Apply optionally provided changes to settings
    fn apply_changes(mut self, settings_json: SettingsView) -> Self {
        if let Some(commentary_text_max_length) = settings_json.commentary_text_max_length {
            self.commentary_text_max_length = commentary_text_max_length;
        }

        if let Some(max_number_of_hashtags_per_kudos) =
            settings_json.max_number_of_hashtags_per_kudos
        {
            self.max_number_of_hashtags_per_kudos = max_number_of_hashtags_per_kudos;
        }

        if let Some(hashtag_text_max_length) = settings_json.hashtag_text_max_length {
            self.hashtag_text_max_length = hashtag_text_max_length;
        }

        if let Some(min_number_of_upvotes_to_exchange_kudos) =
            settings_json.min_number_of_upvotes_to_exchange_kudos
        {
            self.min_number_of_upvotes_to_exchange_kudos = min_number_of_upvotes_to_exchange_kudos;
        }

        if let Some(pok_sbt_ttl) = settings_json.pok_sbt_ttl {
            self.pok_sbt_ttl = pok_sbt_ttl;
        }

        self
    }

    pub(crate) fn validate_hashtags(&self, hashtags: &[String]) {
        require!(
            hashtags.len() <= self.max_number_of_hashtags_per_kudos as usize,
            "Maximum number of hashtags per Kudos exceeded"
        );

        for hashtag in hashtags {
            require!(
                hashtag.len() <= self.hashtag_text_max_length as usize,
                "Hashtag max text length exceeded"
            );
        }
    }

    pub(crate) fn validate_commentary_text(&self, text: &str) {
        require!(
            text.len() <= self.commentary_text_max_length as usize,
            "Commentary text max length exceeded"
        );
    }

    pub(crate) fn verify_number_of_upvotes_to_exchange_kudos(&self, upvotes: usize) -> bool {
        upvotes >= self.min_number_of_upvotes_to_exchange_kudos as usize
    }

    pub(crate) fn acquire_pok_sbt_expire_at_ts(&self, issued_at: u64) -> Result<u64, &'static str> {
        issued_at
            .checked_add(self.pok_sbt_ttl)
            .ok_or("ProofOfKudos SBT expiration date overflow")
    }
}

impl VSettings {
    /// Helper function to migrate settings to the current version and apply changes
    pub(crate) fn apply_changes(&self, settings_json: SettingsView) -> Self {
        Settings::from(self).apply_changes(settings_json).into()
    }
}

fn default_commentary_text_max_length() -> u16 {
    1000
}

fn default_max_number_of_hashtags_per_kudos() -> u8 {
    10
}

fn default_hashtag_text_max_length() -> u8 {
    32
}

fn default_min_number_of_upvotes_to_exchange_kudos() -> u8 {
    3
}

fn default_pok_sbt_ttl() -> u64 {
    365 * 86_400_000
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            commentary_text_max_length: default_commentary_text_max_length(),
            max_number_of_hashtags_per_kudos: default_max_number_of_hashtags_per_kudos(),
            hashtag_text_max_length: default_hashtag_text_max_length(),
            min_number_of_upvotes_to_exchange_kudos:
                default_min_number_of_upvotes_to_exchange_kudos(),
            pok_sbt_ttl: default_pok_sbt_ttl(),
        }
    }
}

impl From<&VSettings> for Settings {
    fn from(v_settings: &VSettings) -> Self {
        match v_settings {
            VSettings::Current(settings) => settings.clone(),
            // TODO: add any migration stuff below
            // e.g. VSettings::V0(settings_v0) => Settings::from(settings_v0),
        }
    }
}

// TODO: impl From<&OLD_VERSION_STRUCT> for CURRENT_VERSION_STRUCT
// e.g. impl From<&SettingsV0> for Settings

impl From<Settings> for VSettings {
    fn from(settings: Settings) -> Self {
        Self::Current(settings)
    }
}

impl From<Settings> for SettingsView {
    fn from(settings: Settings) -> Self {
        Self {
            commentary_text_max_length: Some(settings.commentary_text_max_length),
            max_number_of_hashtags_per_kudos: Some(settings.max_number_of_hashtags_per_kudos),
            hashtag_text_max_length: Some(settings.hashtag_text_max_length),
            min_number_of_upvotes_to_exchange_kudos: Some(
                settings.min_number_of_upvotes_to_exchange_kudos,
            ),
            pok_sbt_ttl: Some(settings.pok_sbt_ttl),
        }
    }
}
