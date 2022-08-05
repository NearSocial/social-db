use crate::*;
use near_sdk::BlockHeight;

pub const EMPTY_KEY: &str = "";

#[derive(BorshSerialize, BorshDeserialize)]
pub struct ValueAtHeight {
    pub value: String,
    pub block_height: BlockHeight,
}

#[derive(BorshSerialize, BorshDeserialize)]
pub enum NodeValue {
    Value(ValueAtHeight),
    Node(NodeId),
}

#[derive(BorshSerialize, BorshDeserialize)]
pub struct Node {
    #[borsh_skip]
    pub node_id: NodeId,
    pub block_height: BlockHeight,
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
        assert!(
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
