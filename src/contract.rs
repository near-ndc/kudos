use crate::account::VAccount;
use crate::misc::RunningState;
use crate::types::{KudosId, StorageKey};
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::store::LookupMap;
use near_sdk::{env, near_bindgen, require, AccountId, PanicOnDefault};

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
}

#[near_bindgen]
impl Contract {
    /// Initializes contract
    #[init]
    pub fn init(owner_id: Option<AccountId>) -> Self {
        Self {
            owner_id: owner_id.unwrap_or_else(env::predecessor_account_id),
            running_state: RunningState::Running,
            last_kudos_id: KudosId::default(),
            accounts: LookupMap::new(StorageKey::Accounts),
            external_db_id: None,
        }
    }

    pub fn set_external_db(&mut self, external_db_id: AccountId) {
        self.assert_owner();
        require!(self.external_db_id == None, "External database already set");
        self.external_db_id = Some(external_db_id);
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
