use crate::types::KudosId;
use crate::{Contract, ContractExt};
use near_sdk::{env, near_bindgen, AccountId};

#[near_bindgen]
impl Contract {
    pub fn upvote_kudos(&mut self, kudos_id: KudosId, text: Option<String>) {
        todo!()
    }

    pub fn leave_kudos(&mut self, receiver_id: AccountId, text: String) -> KudosId {
        let next_kudos_id = self.last_kudos_id.next();

        todo!()
    }
}
