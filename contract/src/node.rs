use crate::*;
use near_sdk::{require, BlockHeight};

pub const EMPTY_KEY: &str = "";
pub const ERR_PERMISSION_DENIED: &str = "Permission Denied";

#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct ValueAtHeight {
    pub value: String,
    pub block_height: BlockHeight,
}

#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub enum NodeValue {
    Value(ValueAtHeight),
    Node(NodeId),
}

impl NodeValue {
    pub fn into_current_height(mut self) -> Self {
        match &mut self {
            NodeValue::Value(v) => {
                v.block_height = env::block_height();
            }
            NodeValue::Node(_) => {}
        };
        self
    }
}

#[derive(BorshSerialize, BorshDeserialize)]
pub struct Node {
    #[borsh_skip]
    pub node_id: NodeId,
    pub block_height: BlockHeight,
    pub children: UnorderedMap<String, NodeValue>,
}

#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct PartialNode {
    pub node_id: NodeId,
    pub block_height: BlockHeight,
    pub children: Vec<(String, NodeValue)>,
    pub from_index: u32,
    pub num_children: u32,
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
    pub fn new(node_id: NodeId, value: Option<ValueAtHeight>) -> Self {
        let mut children = UnorderedMap::new(StorageKey::Node { node_id });
        if let Some(value) = value {
            children.insert(&EMPTY_KEY.to_string(), &NodeValue::Value(value));
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
    pub fn get_node_count(&self) -> u32 {
        self.node_count
    }

    pub fn get_nodes(
        &self,
        from_index: Option<u32>,
        limit: Option<u32>,
    ) -> Vec<Option<PartialNode>> {
        let from_index = from_index.unwrap_or(0);
        let limit = limit.unwrap_or(self.node_count);
        (from_index..std::cmp::min(self.node_count, from_index + limit))
            .map(|node_id| self.get_node(node_id, None, None))
            .collect()
    }

    pub fn get_node(
        &self,
        node_id: NodeId,
        from_index: Option<u32>,
        limit: Option<u32>,
    ) -> Option<PartialNode> {
        Some(if node_id == 0 {
            partial_node_view(&self.root_node, from_index, limit)
        } else {
            partial_node_view(&self.internal_get_node(node_id)?, from_index, limit)
        })
    }
}

fn partial_node_view(node: &Node, from_index: Option<u32>, limit: Option<u32>) -> PartialNode {
    let num_children = node.children.len() as _;
    let from_index = from_index.unwrap_or(0);
    let limit = limit.unwrap_or(num_children);
    let keys = node.children.keys_as_vector();
    let values = node.children.values_as_vector();
    let children = (from_index..std::cmp::min(num_children, from_index + limit))
        .map(|index| {
            (
                keys.get(index as _).unwrap(),
                values.get(index as _).unwrap(),
            )
        })
        .collect();
    PartialNode {
        node_id: node.node_id,
        block_height: node.block_height,
        children,
        from_index,
        num_children,
    }
}
