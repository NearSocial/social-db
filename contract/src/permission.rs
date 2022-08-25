use crate::*;
use near_sdk::{require, PublicKey};
use std::collections::HashSet;

#[derive(BorshSerialize, BorshDeserialize, Serialize)]
#[serde(crate = "near_sdk::serde")]
pub enum PermissionKey {
    AccountId(AccountId),
    SignerPublicKey(PublicKey),
}

#[derive(BorshSerialize, BorshDeserialize, Serialize)]
#[serde(crate = "near_sdk::serde")]
pub enum Permission {
    Granted(HashSet<NodeId>),
}

impl Permission {
    pub fn is_empty(&self) -> bool {
        match self {
            Permission::Granted(node_ids) => node_ids.is_empty(),
        }
    }
}

#[near_bindgen]
impl Contract {
    #[payable]
    pub fn grant_write_permission(
        &mut self,
        predecessor_id: Option<AccountId>,
        public_key: Option<PublicKey>,
        keys: Vec<String>,
    ) {
        let attached_deposit = env::attached_deposit();
        require!(attached_deposit > 0, "Requires at least 1 yocto");
        let permission_key = predecessor_id
            .map(|a| {
                require!(
                    public_key.is_none(),
                    "Can't supply both account_id and a public_key"
                );
                PermissionKey::AccountId(a)
            })
            .or_else(|| public_key.map(|pk| PermissionKey::SignerPublicKey(pk)))
            .expect("Neither account_id or public_key is provided");
        let account_id = env::predecessor_account_id();
        let mut account =
            self.internal_unwrap_account_or_create(account_id.as_str(), attached_deposit);
        let mut permission = account
            .permissions
            .get(&permission_key)
            .unwrap_or_else(|| Permission::Granted(HashSet::new()));
        match &mut permission {
            Permission::Granted(node_ids) => {
                node_ids.extend(keys.into_iter().map(|key| {
                    account.storage_tracker.start();
                    let path: Vec<&str> = key.split(SEPARATOR).collect();
                    require!(!path.is_empty(), "The key is empty");
                    assert_eq!(
                        path[0],
                        account_id.as_str(),
                        "The path should start with the expected account_id"
                    );
                    let mut node = Some(self.internal_unwrap_node(account.node_id));
                    for &key in &path[1..] {
                        assert_key_valid(key);
                        let node_value = node.as_ref().unwrap().children.get(&key.to_string());
                        match node_value {
                            None => {
                                let node_id = self.create_node_id();
                                node.as_mut()
                                    .unwrap()
                                    .children
                                    .insert(&key.to_string(), &NodeValue::Node(node_id));
                                self.internal_set_node(
                                    node.replace(Node::new(node_id, None)).unwrap(),
                                );
                            }
                            Some(NodeValue::Node(node_id)) => {
                                self.internal_set_node(
                                    node.replace(self.internal_unwrap_node(node_id)).unwrap(),
                                );
                            }
                            Some(NodeValue::Value(value_at_height)) => {
                                assert_ne!(
                                    key, EMPTY_KEY,
                                    "The empty key's value should be a string"
                                );
                                let node_id = self.create_node_id();
                                node.as_mut()
                                    .unwrap()
                                    .children
                                    .insert(&key.to_string(), &NodeValue::Node(node_id));
                                self.internal_set_node(
                                    node.replace(Node::new(node_id, Some(value_at_height)))
                                        .unwrap(),
                                );
                            }
                        };
                    }
                    let node_id = node.as_ref().unwrap().node_id;
                    self.internal_set_node(node.unwrap());
                    account.storage_tracker.stop();
                    node_id
                }));
            }
        };
        account.internal_set_permission(&permission_key, permission);
        self.internal_set_account(account);
    }

    pub fn debug_get_permissions(&self, account_id: AccountId) -> Vec<(PermissionKey, Permission)> {
        let account = self.internal_unwrap_account(account_id.as_str());
        account.permissions.to_vec()
    }

    /// Returns true if the permission is granted for a given account ID or a given public_key to
    /// any prefix of the key.
    pub fn is_write_permission_granted(
        &self,
        predecessor_id: Option<AccountId>,
        public_key: Option<PublicKey>,
        key: String,
    ) -> bool {
        let permission_key = predecessor_id
            .map(|a| {
                require!(
                    public_key.is_none(),
                    "Can't supply both account_id and a public_key"
                );
                PermissionKey::AccountId(a)
            })
            .or_else(|| public_key.map(|pk| PermissionKey::SignerPublicKey(pk)))
            .expect("Neither account_id or public_key is provided");

        let path: Vec<&str> = key.split(SEPARATOR).collect();
        require!(!path.is_empty(), "The key is empty");
        let account = if let Some(account) = self.internal_get_account(path[0]) {
            account
        } else {
            return false;
        };
        let permission = if let Some(permission) = account.permissions.get(&permission_key) {
            permission
        } else {
            return false;
        };

        match permission {
            Permission::Granted(node_ids) => {
                if node_ids.contains(&account.node_id) {
                    return true;
                }

                let mut node = self.internal_unwrap_node(account.node_id);

                for &key in &path[1..] {
                    let node_value = node.children.get(&key.to_string());
                    if let Some(NodeValue::Node(node_id)) = node_value {
                        if node_ids.contains(&node_id) {
                            return true;
                        }
                        node = self.internal_unwrap_node(node_id);
                    } else {
                        return false;
                    }
                }
            }
        }
        return false;
    }
}

impl Account {
    pub fn internal_get_writeable_node_ids(&self) -> HashSet<NodeId> {
        let mut res = HashSet::new();
        if let Some(Permission::Granted(node_ids)) = self
            .permissions
            .get(&PermissionKey::AccountId(env::predecessor_account_id()))
        {
            res.extend(node_ids)
        };
        if let Some(Permission::Granted(node_ids)) = self
            .permissions
            .get(&PermissionKey::SignerPublicKey(env::signer_account_pk()))
        {
            res.extend(node_ids)
        };
        res
    }

    pub fn internal_set_permission(
        &mut self,
        permission_key: &PermissionKey,
        permission: Permission,
    ) {
        self.storage_tracker.start();
        if permission.is_empty() {
            self.permissions.remove(permission_key);
        } else {
            self.permissions.insert(permission_key, &permission);
        }
        self.storage_tracker.stop()
    }
}
