use crate::*;
use near_sdk::{require, BlockHeight};

pub const EMPTY_KEY: &str = "";
pub const ERR_PERMISSION_DENIED: &str = "Permission Denied";

#[derive(BorshSerialize, BorshDeserialize, Serialize)]
#[serde(crate = "near_sdk::serde")]
pub struct ValueAtHeight {
    pub value: String,
    pub block_height: BlockHeight,
}

#[derive(BorshSerialize, BorshDeserialize, Serialize)]
#[serde(crate = "near_sdk::serde")]
pub enum NodeValue {
    Value(ValueAtHeight),
    Node(NodeId),
    DeletedEntry(BlockHeight),
}

mod unordered_map_expensive {
    use super::*;
    use near_sdk::borsh::{BorshDeserialize, BorshSerialize};
    use near_sdk::serde::Serializer;

    pub fn serialize<S, K, V>(map: &UnorderedMap<K, V>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
        K: Serialize + BorshDeserialize + BorshSerialize,
        V: Serialize + BorshDeserialize + BorshSerialize,
    {
        serializer.collect_seq(map.iter())
    }
}

#[derive(BorshSerialize, BorshDeserialize, Serialize)]
#[serde(crate = "near_sdk::serde")]
pub struct Node {
    #[borsh_skip]
    pub node_id: NodeId,
    pub block_height: BlockHeight,
    #[serde(with = "unordered_map_expensive")]
    pub children: UnorderedMap<String, NodeValue>,
}

#[derive(BorshSerialize, BorshDeserialize)]
pub enum VNode {
    Current(Node),
}

impl From<VNode> for Node {
    fn from(v: VNode) -> Self {
        match v {
            VNode::Current(c) => c,
        }
    }
}

impl From<Node> for VNode {
    fn from(c: Node) -> Self {
        VNode::Current(c)
    }
}

impl Node {
    pub fn new(node_id: NodeId, value: Option<NodeValue>) -> Self {
        let mut children = UnorderedMap::new(StorageKey::Node { node_id });
        if let Some(value) = value {
            require!(
                !matches!(value, NodeValue::Node(_)),
                "Invariant: empty key value can't be a node"
            );
            children.insert(&EMPTY_KEY.to_string(), &value);
        }
        Self {
            node_id,
            block_height: env::block_height(),
            children,
        }
    }

    pub fn set(&mut self, key: &String, value: &str) {
        let prev_value = self.children.insert(
            &key,
            &NodeValue::Value(ValueAtHeight {
                value: value.to_string(),
                block_height: env::block_height(),
            }),
        );
        require!(
            !matches!(prev_value, Some(NodeValue::Node(_))),
            "Internal error, the replaced value was a node"
        );
    }
}

impl Contract {
    pub fn internal_get_node(&self, node_id: NodeId) -> Option<Node> {
        self.nodes.get(&node_id).map(|o| {
            let mut node: Node = o.into();
            node.node_id = node_id;
            node
        })
    }

    pub fn internal_unwrap_node(&self, node_id: NodeId) -> Node {
        self.internal_get_node(node_id).expect("Node is missing")
    }

    pub fn internal_set_node(&mut self, mut node: Node) {
        let node_id = node.node_id;
        node.block_height = env::block_height();
        self.nodes.insert(&node_id, &node.into());
    }
}

#[near_bindgen]
impl Contract {
    pub fn debug_nodes(self) -> Vec<Option<Node>> {
        let mut nodes = vec![None];
        for node_id in 1..self.node_count {
            nodes.push(self.internal_get_node(node_id));
        }
        nodes[0].replace(self.root_node);
        nodes
    }
}
