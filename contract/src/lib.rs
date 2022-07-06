mod storage;
mod storage_tracker;

pub use crate::storage::*;
use crate::storage_tracker::*;

use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::{LookupMap};
use near_sdk::json_types::U128;
use near_sdk::{
    assert_one_yocto, env, near_bindgen, AccountId, Balance, BorshStorageKey,
    PanicOnDefault, Promise};

#[derive(BorshSerialize, BorshStorageKey)]
#[allow(unused)]
enum StorageKey {
    Storage,
}

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct Contract {
    pub storage: LookupMap<AccountId, VStorage>,
}

#[near_bindgen]
impl Contract {
    #[init]
    pub fn new() -> Self {
        Self {
            storage: LookupMap::new(StorageKey::Storage),
        }
    }
}
