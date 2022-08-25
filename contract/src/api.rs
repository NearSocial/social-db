use crate::*;
use near_sdk::serde_json::map::Entry;
use near_sdk::serde_json::{Map, Value};
use near_sdk::{log, require};
use std::collections::HashSet;

pub const SEPARATOR: char = '/';

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
    pub fn get(self, keys: Vec<String>) -> Value {
        let mut res: Map<String, Value> = Map::new();
        for key in keys {
            let path: Vec<&str> = key.split(SEPARATOR).collect();
            if path.is_empty() {
                continue;
            }
            self.recursive_get(&mut res, &self.root_node, &path[..])
        }
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
            log!("account's node_id: {}", account.node_id);
            let node = self.internal_unwrap_node(account.node_id);
            account.storage_tracker.start();
            self.recursive_set(node, value, write_approved, &writable_node_ids);
            account.storage_tracker.stop();
            self.internal_set_account(account);
        }
    }
}

impl Contract {
    pub fn recursive_get(&self, res: &mut Map<String, Value>, node: &Node, keys: &[&str]) {
        let matched_entries = if keys[0] == "*" || keys[0] == "**" {
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
                    let inner_node = self.internal_unwrap_node(node_id);
                    if keys.len() > 1 || keys[0] == "**" {
                        // Going deeper
                        let inner_map = json_map_get_inner_object(res, key);
                        if keys.len() > 1 {
                            self.recursive_get(inner_map, &inner_node, &keys[1..]);
                        }
                        if keys[0] == "**" {
                            // Non skipping step in.
                            self.recursive_get(inner_map, &inner_node, keys);
                        }
                    } else {
                        if let Some(NodeValue::Value(value_at_height)) =
                            inner_node.children.get(&EMPTY_KEY.to_string())
                        {
                            json_map_set_key(res, key, value_at_height.value);
                        } else {
                            // mismatch skipping
                        }
                    }
                }
                NodeValue::Value(value_at_height) => {
                    if keys.len() == 1 {
                        json_map_set_key(res, key, value_at_height.value);
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
        log!(
            "node_id: {}, value: {}",
            node.node_id,
            near_sdk::serde_json::to_string(&value).unwrap()
        );
        let write_approved = write_approved || writable_node_ids.contains(&node.node_id);
        if let Some(s) = value.as_str() {
            require!(write_approved, ERR_PERMISSION_DENIED);
            node.set(&EMPTY_KEY.to_string(), s);
        } else if let Some(obj) = value.as_object_mut() {
            for (key, value) in obj {
                assert_key_valid(key.as_str());
                let node_value = node.children.get(key);
                match node_value {
                    None => {
                        require!(write_approved, ERR_PERMISSION_DENIED);
                        if let Some(s) = value.as_str() {
                            node.set(key, s);
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
                    Some(NodeValue::Value(value_at_height)) => {
                        require!(write_approved, ERR_PERMISSION_DENIED);
                        if let Some(s) = value.as_str() {
                            node.set(key, s);
                        } else {
                            assert_ne!(
                                key.as_str(),
                                EMPTY_KEY,
                                "The empty key's value should be a string"
                            );
                            let node_id = self.create_node_id();
                            node.children.insert(key, &NodeValue::Node(node_id));
                            self.recursive_set(
                                Node::new(node_id, Some(value_at_height)),
                                value,
                                write_approved,
                                writable_node_ids,
                            );
                        }
                    }
                }
            }
        } else {
            env::panic_str("The JSON value is not a string and not an object")
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
                // Assuming the previous value is a string
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

fn json_map_set_key(res: &mut Map<String, Value>, key: String, value: String) {
    match res.entry(key) {
        Entry::Vacant(e) => {
            e.insert(Value::String(value));
        }
        Entry::Occupied(mut e) => {
            match e.get_mut() {
                Value::Object(o) => {
                    o.insert(EMPTY_KEY.to_string(), Value::String(value));
                }
                Value::String(s) => {
                    *s = value;
                }
                _ => unreachable!(),
            };
        }
    };
}

pub(crate) fn is_key_valid(key: &str) -> bool {
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
    assert!(is_key_valid(key), "Key contains invalid character");
}
