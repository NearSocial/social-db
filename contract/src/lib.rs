mod account;
mod api;
mod node;
mod permission;
mod storage_tracker;

pub use crate::account::*;
pub use crate::api::*;
pub use crate::node::*;
pub use crate::permission::*;
use crate::storage_tracker::*;

use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::{LookupMap, UnorderedMap};
use near_sdk::json_types::U128;
use near_sdk::{
    assert_one_yocto, env, near_bindgen, AccountId, Balance, BorshStorageKey, PanicOnDefault,
    Promise,
};

#[derive(BorshSerialize, BorshStorageKey)]
#[allow(unused)]
enum StorageKey {
    Account,
    Nodes,
    Node { node_id: NodeId },
    Permissions { node_id: NodeId },
}

pub type NodeId = u32;

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct Contract {
    pub accounts: LookupMap<NodeId, VAccount>,
    pub root_node: Node,
    pub nodes: LookupMap<NodeId, VNode>,
    pub node_count: NodeId,
}

#[near_bindgen]
impl Contract {
    #[init]
    pub fn new() -> Self {
        Self {
            accounts: LookupMap::new(StorageKey::Account),
            root_node: Node::new(0, None),
            nodes: LookupMap::new(StorageKey::Nodes),
            node_count: 1,
        }
    }
}

impl Contract {
    pub fn create_node_id(&mut self) -> NodeId {
        let node_id = self.node_count;
        self.node_count += 1;
        node_id
    }
}
