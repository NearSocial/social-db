use crate::*;
use near_sdk::require;
use near_sdk::serde_json::map::Entry;
use near_sdk::serde_json::{Map, Value};
use std::collections::HashSet;

pub const MAX_KEY_LENGTH: usize = 256;
pub const SEPARATOR: char = '/';
pub const STAR: &str = "*";
pub const RECURSIVE_STAR: &str = "**";
pub const KEY_BLOCK_HEIGHT: &str = ":block";
pub const KEY_NODE_ID: &str = ":node";

#[derive(Serialize, Deserialize, Default)]
#[serde(crate = "near_sdk::serde")]
pub struct GetOptions {
    pub with_block_height: Option<bool>,
    pub with_node_id: Option<bool>,
    pub return_deleted: Option<bool>,
}

#[derive(Copy, Clone, Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub enum KeysReturnType {
    True,
    BlockHeight,
    NodeId,
}

#[derive(Serialize, Deserialize, Default)]
#[serde(crate = "near_sdk::serde")]
pub struct KeysOptions {
    /// The type of the returned values. By default returns true.
    pub return_type: Option<KeysReturnType>,
    /// Whether to match keys for deleted values.
    pub return_deleted: Option<bool>,
    /// Whether to match nodes.
    pub values_only: Option<bool>,
}

#[near_bindgen]
impl Contract {
    /// ```js
    ///
    /// get({keys: [
    ///   "alex.near/profile/*",
    ///   "alex.near/profile/**",
    ///   "alex.near/profile/[name,url,image_url]",
    ///   "alex.near/profile/url",
    ///   "alex.near/profile/",
    ///   "bob.near/profile/*",
    ///   "alex.near/graph/follow/*",
    /// ]})
    /// ```
    pub fn get(self, keys: Vec<String>, options: Option<GetOptions>) -> Value {
        let options = options.unwrap_or_default();
        let mut res: Map<String, Value> = Map::new();
        for key in keys {
            let mut path: Vec<&str> = key.split(SEPARATOR).collect();
            if path.last() == Some(&EMPTY_KEY) {
                path.pop();
                if path.last() == Some(&EMPTY_KEY) {
                    continue;
                }
            }
            if path.is_empty() {
                continue;
            }
            self.recursive_get(&mut res, &self.root_node, &path[..], &options)
        }
        json_map_recursive_cleanup(&mut res);
        Value::Object(res)
    }

    /// ```js
    /// Note, recursive match all pattern "**" is not allowed.
    ///
    /// keys({keys: [
    ///   "alex.near/profile/*",
    ///   "alex.near/profile/*",
    ///   "alex.near/profile/[name,url,image_url]",
    ///   "alex.near/profile/url",
    ///   "alex.near/profile/",
    ///   "bob.near/profile/*",
    ///   "alex.near/graph/follow/*",
    /// ]})
    /// ```
    pub fn keys(self, keys: Vec<String>, options: Option<KeysOptions>) -> Value {
        let options = options.unwrap_or_default();
        let mut res: Map<String, Value> = Map::new();
        for key in keys {
            let mut path: Vec<&str> = key.split(SEPARATOR).collect();
            if path.last() == Some(&EMPTY_KEY) {
                path.pop();
                if path.last() == Some(&EMPTY_KEY) {
                    continue;
                }
            }
            if path.is_empty() {
                continue;
            }
            self.recursive_keys(
                &mut res,
                &self.root_node,
                &path[..],
                &options,
            )
        }
        json_map_recursive_cleanup(&mut res);
        Value::Object(res)
    }

    /// ```js
    /// user_set({
    ///   "alex.near": {
    ///     "graph": "yoloyoloyoloyolo:yoloyoloyoloyoloyo:lo",
    ///   }
    /// })
    ///
    /// $account_id/badge/$badge_id/metadata
    /// $account_id/badge/$badge_id/owners/$receiver_id
    ///
    /// user_set({
    ///   "alex.near": {
    ///     "graph": {
    ///       "follow": {
    ///         "root.near": "",
    ///         "bob.near": "",
    ///       }
    ///     }
    ///   }
    /// })
    /// ```
    #[payable]
    pub fn set(&mut self, mut data: Value) {
        self.assert_live();
        let account_id = env::predecessor_account_id();
        let mut attached_balance = env::attached_deposit();
        for (key, value) in data.as_object_mut().expect("Data is not a JSON object") {
            let mut account = self.internal_unwrap_account_or_create(&key, attached_balance);
            attached_balance = 0;
            let write_approved = key == account_id.as_str() && env::attached_deposit() > 0;
            let writable_node_ids = if write_approved {
                HashSet::new()
            } else {
                account.internal_get_writeable_node_ids()
            };
            let node = self.internal_unwrap_node(account.node_id);
            account.storage_tracker.start();
            self.recursive_set(node, value, write_approved, &writable_node_ids);
            account.storage_tracker.stop();
            self.internal_set_account(account);
        }
    }
}

impl Contract {
    pub fn recursive_get(
        &self,
        res: &mut Map<String, Value>,
        node: &Node,
        keys: &[&str],
        options: &GetOptions,
    ) {
        let is_recursive_match_all = keys[0] == RECURSIVE_STAR;
        if is_recursive_match_all {
            require!(keys.len() == 1, "'**' pattern can only be used as a suffix")
        }
        let matched_entries = if keys[0] == STAR || is_recursive_match_all {
            node.children.to_vec()
        } else {
            let key = keys[0].to_string();
            if let Some(value) = node.children.get(&key) {
                vec![(key, value)]
            } else {
                vec![]
            }
        };
        if options.with_block_height == Some(true) {
            res.insert(KEY_BLOCK_HEIGHT.to_string(), node.block_height.into());
        }
        if options.with_node_id == Some(true) {
            res.insert(KEY_NODE_ID.to_string(), node.node_id.into());
        }
        for (key, value) in matched_entries {
            match value {
                NodeValue::Node(node_id) => {
                    let inner_node = self.internal_unwrap_node(node_id);
                    if keys.len() > 1 || is_recursive_match_all {
                        // Going deeper
                        let inner_map = json_map_get_inner_object(res, key);
                        if keys.len() > 1 {
                            self.recursive_get(inner_map, &inner_node, &keys[1..], options);
                        }
                        if is_recursive_match_all {
                            // Non skipping step in.
                            self.recursive_get(inner_map, &inner_node, keys, options);
                        }
                    } else {
                        if let Some(node_value) = inner_node.children.get(&EMPTY_KEY.to_string()) {
                            if options.with_node_id == Some(true) {
                                let inner_map = json_map_get_inner_object(res, key.clone());
                                inner_map
                                    .insert(KEY_NODE_ID.to_string(), inner_node.node_id.into());
                            }
                            json_map_set_key(res, key, node_value, &options);
                        } else {
                            // mismatch skipping
                        }
                    }
                }
                node_value => {
                    if keys.len() == 1 {
                        json_map_set_key(res, key, node_value, &options);
                    }
                }
            }
        }
    }

    pub fn recursive_keys(
        &self,
        res: &mut Map<String, Value>,
        node: &Node,
        keys: &[&str],
        options: &KeysOptions,
    ) {
        let matched_entries = if keys[0] == STAR {
            node.children.to_vec()
        } else {
            let key = keys[0].to_string();
            if let Some(value) = node.children.get(&key) {
                vec![(key, value)]
            } else {
                vec![]
            }
        };
        for (key, value) in matched_entries {
            match value {
                NodeValue::Node(node_id) => {
                    if keys.len() == 1 {
                        let value = if options.values_only.unwrap_or(false) {
                            let inner_node = self.internal_unwrap_node(node_id);
                            if let Some(node_value) = inner_node.children.get(&EMPTY_KEY.to_string()) {
                                if options.return_deleted.unwrap_or(false) || !matches!(node_value, NodeValue::DeletedEntry(_)) {
                                    match options.return_type.unwrap_or(KeysReturnType::True) {
                                        KeysReturnType::True => true.into(),
                                        KeysReturnType::BlockHeight => {
                                            node_value.get_block_height().unwrap().into()
                                        }
                                        KeysReturnType::NodeId => node_id.into(),
                                    }
                                } else {
                                    // deleted entry
                                    continue;
                                }
                            } else {
                                // mismatch skipping
                                continue;
                            }
                        } else {
                            match options.return_type.unwrap_or(KeysReturnType::True) {
                                KeysReturnType::True => true.into(),
                                KeysReturnType::BlockHeight => {
                                    let inner_node = self.internal_unwrap_node(node_id);
                                    inner_node.block_height.into()
                                }
                                KeysReturnType::NodeId => node_id.into(),
                            }
                        };
                        json_map_set_value(res, key, value);
                    } else {
                        let inner_node = self.internal_unwrap_node(node_id);
                        let inner_map = json_map_get_inner_object(res, key);
                        self.recursive_keys(
                            inner_map,
                            &inner_node,
                            &keys[1..],
                            options,
                        );
                    }
                }
                NodeValue::Value(value_at_height) => {
                    if keys.len() == 1 {
                        let value = match options.return_type.unwrap_or(KeysReturnType::True) {
                            KeysReturnType::True => true.into(),
                            KeysReturnType::BlockHeight => value_at_height.block_height.into(),
                            KeysReturnType::NodeId => Value::Null,
                        };
                        json_map_set_value(res, key, value);
                    }
                }
                NodeValue::DeletedEntry(block_height) => {
                    if keys.len() == 1 && options.return_deleted.unwrap_or(false) {
                        let value = match options.return_type.unwrap_or(KeysReturnType::True) {
                            KeysReturnType::True => true.into(),
                            KeysReturnType::BlockHeight => block_height.into(),
                            KeysReturnType::NodeId => Value::Null,
                        };
                        json_map_set_value(res, key, value);
                    }
                }
            }
        }
    }

    pub fn recursive_set(
        &mut self,
        mut node: Node,
        value: &mut Value,
        write_approved: bool,
        writable_node_ids: &HashSet<NodeId>,
    ) {
        let write_approved = write_approved || writable_node_ids.contains(&node.node_id);
        if value.is_string() || value.is_null() {
            require!(write_approved, ERR_PERMISSION_DENIED);
            node.set(&EMPTY_KEY.to_string(), value);
        } else if let Some(obj) = value.as_object_mut() {
            for (key, value) in obj {
                assert_key_valid(key.as_str());
                let node_value = node.children.get(key);
                match node_value {
                    None => {
                        require!(write_approved, ERR_PERMISSION_DENIED);
                        if value.is_string() || value.is_null() {
                            node.set(key, value);
                        } else {
                            let node_id = self.create_node_id();
                            node.children.insert(key, &NodeValue::Node(node_id));
                            self.recursive_set(
                                Node::new(node_id, None),
                                value,
                                write_approved,
                                writable_node_ids,
                            );
                        }
                    }
                    Some(NodeValue::Node(node_id)) => {
                        self.recursive_set(
                            self.internal_unwrap_node(node_id),
                            value,
                            write_approved,
                            writable_node_ids,
                        );
                    }
                    Some(old_node_value) => {
                        require!(write_approved, ERR_PERMISSION_DENIED);
                        if value.is_string() || value.is_null() {
                            node.set(key, value);
                        } else {
                            assert_ne!(
                                key.as_str(),
                                EMPTY_KEY,
                                "The empty key's value should be a string or null"
                            );
                            let node_id = self.create_node_id();
                            node.children.insert(key, &NodeValue::Node(node_id));
                            self.recursive_set(
                                Node::new(node_id, Some(old_node_value)),
                                value,
                                write_approved,
                                writable_node_ids,
                            );
                        }
                    }
                }
            }
        } else {
            env::panic_str("The JSON value is not a string, a null or an object")
        }
        self.internal_set_node(node);
    }
}

fn json_map_get_inner_object(res: &mut Map<String, Value>, key: String) -> &mut Map<String, Value> {
    match res.entry(key.clone()) {
        Entry::Vacant(e) => {
            e.insert(Value::Object(Map::new()));
        }
        Entry::Occupied(mut e) => {
            if !e.get().is_object() {
                // Assuming the previous value is a string or null
                let prev_value = e.insert(Value::Object(Map::new()));
                e.get_mut()
                    .as_object_mut()
                    .unwrap()
                    .insert(EMPTY_KEY.to_string(), prev_value);
            }
        }
    };
    res.get_mut(&key).unwrap().as_object_mut().unwrap()
}

fn json_map_set_value(res: &mut Map<String, Value>, key: String, value: Value) {
    match res.entry(key) {
        Entry::Vacant(e) => {
            e.insert(value);
        }
        Entry::Occupied(mut e) => {
            match e.get_mut() {
                Value::Object(o) => {
                    o.insert(EMPTY_KEY.to_string(), value);
                }
                _ => {}
            };
        }
    };
}

fn json_map_set_key(
    res: &mut Map<String, Value>,
    key: String,
    node_value: NodeValue,
    options: &GetOptions,
) {
    match res.entry(key) {
        Entry::Vacant(e) => {
            let block_height = node_value.get_block_height();
            let new_value = if let NodeValue::Value(value_at_height) = node_value {
                Value::String(value_at_height.value)
            } else if options.return_deleted == Some(true)
                && matches!(node_value, NodeValue::DeletedEntry(_))
            {
                Value::Null
            } else {
                return;
            };
            if options.with_block_height == Some(true) {
                let mut m = Map::new();
                m.insert(KEY_BLOCK_HEIGHT.to_string(), block_height.unwrap().into());
                m.insert(EMPTY_KEY.to_string(), new_value);

                e.insert(Value::Object(m));
            } else {
                e.insert(new_value);
            }
        }
        Entry::Occupied(mut e) => {
            match e.get_mut() {
                Value::Object(o) => {
                    json_map_set_key(o, EMPTY_KEY.to_string(), node_value, options);
                }
                _ => {
                    // Shouldn't be any changes as the values should match.
                }
            };
        }
    };
}

// Returns true if the given map is not empty.
fn json_map_recursive_cleanup(res: &mut Map<String, Value>) -> bool {
    let mut num_special_keys = 0;
    res.retain(|k, v| {
        if k.starts_with(":") {
            num_special_keys += 1;
            return true;
        }
        match v {
            Value::Object(o) => json_map_recursive_cleanup(o),
            _ => true,
        }
    });
    res.len() > num_special_keys
}

pub(crate) fn is_key_valid(key: &str) -> bool {
    if key.len() > MAX_KEY_LENGTH {
        return false;
    }
    for &c in key.as_bytes() {
        match c {
            b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' => {}
            b'_' | b'.' | b'-' => {}
            _ => return false,
        }
    }
    true
}

pub(crate) fn assert_key_valid(key: &str) {
    assert!(
        is_key_valid(key),
        "Key contains invalid character or longer than {}",
        MAX_KEY_LENGTH
    );
}
