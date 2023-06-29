use crate::account::VAccount;
use crate::consts::{DEFAULT_COMMENTARY_TEXT_MAX_LENGTH, DEFAULT_HASHTAG_MAX_LENGTH};
use crate::external_db::ext_db;
use crate::misc::RunningState;
use crate::types::{KudosId, StorageKey};
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::store::LookupMap;
use near_sdk::{
    env, near_bindgen, require, AccountId, PanicOnDefault, Promise, PromiseError, ONE_YOCTO,
};

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct Contract {
    /// The contract's owner account id
    pub(crate) owner_id: AccountId,
    /// Contract's state, e.g. running, paused
    pub(crate) running_state: RunningState,
    /// Last Kudos unique identifier used to get next incremented unique id
    pub(crate) last_kudos_id: KudosId,
    /// User versioned accounts data keyed by AccountId
    pub(crate) accounts: LookupMap<AccountId, VAccount>,
    pub(crate) external_db_id: Option<AccountId>,
    pub(crate) commentary_text_max_length: u16,
    pub(crate) hashtag_max_length: u8,
    pub(crate) iah_registry: AccountId,
}

#[near_bindgen]
impl Contract {
    /// Initializes contract
    #[init]
    pub fn init(owner_id: Option<AccountId>, iah_registry: AccountId) -> Self {
        Self {
            owner_id: owner_id.unwrap_or_else(env::predecessor_account_id),
            running_state: RunningState::Running,
            last_kudos_id: KudosId::default(),
            accounts: LookupMap::new(StorageKey::Accounts),
            external_db_id: None,
            commentary_text_max_length: DEFAULT_COMMENTARY_TEXT_MAX_LENGTH,
            hashtag_max_length: DEFAULT_HASHTAG_MAX_LENGTH,
            iah_registry,
        }
    }

    #[payable]
    pub fn set_external_db(&mut self, external_db_id: AccountId) -> Promise {
        self.assert_owner();
        require!(self.external_db_id == None, "External database already set");

        // Grant write permission to IAH Registry to be able to use `IAHRegistry::is_human_call`,
        // because SocialDB checks for a predecessor_id.
        // This will require a minimum amount of deposit to register a user for Kudos contract.
        // Minimum amount of deposit required could be priorly acquired by calling a view method
        // `storage_balance_bounds` to Social-Db contract
        ext_db::ext(external_db_id.clone())
            .with_attached_deposit(env::attached_deposit())
            //.with_static_gas(static_gas) TODO: use pre-computed static amount gas
            .grant_write_permission(
                Some(self.iah_registry.clone()),
                None,
                vec![format!("{}", env::current_account_id())],
            )
            .then(
                Self::ext(env::current_account_id())
                    .on_ext_db_write_permission_granted(external_db_id),
            )
            .into()
    }

    #[private]
    #[handle_result]
    pub fn on_ext_db_write_permission_granted(
        &mut self,
        external_db_id: AccountId,
        #[callback_result] callback_result: Result<(), PromiseError>,
    ) -> Result<(), String> {
        callback_result
            .map_err(|e| format!("SocialDB::grant_write_permission() call failure: {:?}", e))?;
        self.external_db_id = Some(external_db_id);
        Ok(())
    }
}

impl Contract {
    /// Checks if contract is at running state
    pub(crate) fn assert_contract_running(&self) {
        require!(
            self.running_state == RunningState::Running,
            "Contract paused"
        );
    }

    /// Asserts if the caller is not an owner of the contract
    pub(crate) fn assert_owner(&self) {
        require!(self.is_owner(&env::predecessor_account_id()), "Not allowed");
    }

    /// Checks ifn the caller is an owner of the contract
    pub(crate) fn is_owner(&self, account_id: &AccountId) -> bool {
        account_id == &self.owner_id
    }
}
