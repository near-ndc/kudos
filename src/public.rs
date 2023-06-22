use crate::{Contract, ContractExt};
use crate::types::KudoId;
use near_sdk::{env, near_bindgen, require, AccountId, PanicOnDefault};

#[near_bindgen]
impl Contract {
    pub fn upvote_kudo(&mut self, kudo_id: KudoId) {
        todo!()
    }
}
