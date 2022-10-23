use crate::*;
use near_contract_standards::storage_management::{
    StorageBalance, StorageBalanceBounds, StorageManagement,
};
use near_sdk::{require, StorageUsage};
use std::convert::TryFrom;

/// 2000 bytes
const MIN_STORAGE_BALANCE: Balance = 2000u128 * env::STORAGE_PRICE_PER_BYTE;

#[derive(BorshSerialize, BorshDeserialize, Serialize)]
#[serde(crate = "near_sdk::serde")]
pub struct Account {
    #[serde(with = "u128_dec_format")]
    pub storage_balance: Balance,
    pub used_bytes: StorageUsage,
    /// Tracks all currently active permissions given by this account.
    #[serde(with = "unordered_map_expensive")]
    pub permissions: UnorderedMap<PermissionKey, Permission>,
    #[borsh_skip]
    pub node_id: NodeId,
    #[serde(skip)]
    #[borsh_skip]
    pub storage_tracker: StorageTracker,
}

#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct PartialAccount {
    #[serde(with = "u128_dec_format")]
    pub storage_balance: Balance,
    pub used_bytes: StorageUsage,
    pub permissions: Vec<(PermissionKey, Permission)>,
    pub node_id: NodeId,
}

#[derive(BorshSerialize, BorshDeserialize)]
pub enum VAccount {
    Current(Account),
}

impl From<VAccount> for Account {
    fn from(v: VAccount) -> Self {
        match v {
            VAccount::Current(c) => c,
        }
    }
}

impl From<Account> for VAccount {
    fn from(c: Account) -> Self {
        VAccount::Current(c)
    }
}

impl Account {
    pub fn new(node_id: NodeId) -> Self {
        Self {
            storage_balance: 0,
            used_bytes: 0,
            permissions: UnorderedMap::new(StorageKey::Permissions { node_id }),
            node_id,
            storage_tracker: Default::default(),
        }
    }

    fn assert_storage_covered(&self) {
        let storage_balance_needed = Balance::from(self.used_bytes) * env::storage_byte_cost();
        assert!(
            storage_balance_needed <= self.storage_balance,
            "Not enough storage balance"
        );
    }
}

impl Contract {
    pub fn internal_get_account(&self, account_id: &str) -> Option<Account> {
        self.root_node
            .children
            .get(&account_id.to_string())
            .map(|v| match v {
                NodeValue::Value(_) => {
                    env::panic_str("Unexpected account key. The value is not a node.")
                }
                NodeValue::Node(node_id) => {
                    let mut account: Account = self.accounts.get(&node_id).unwrap().into();
                    account.node_id = node_id;
                    account
                }
            })
    }

    pub fn internal_unwrap_account(&self, account_id: &str) -> Account {
        self.internal_get_account(account_id)
            .expect("The account doesn't exist")
    }

    pub fn internal_unwrap_account_or_create(
        &mut self,
        account_id: &str,
        storage_deposit: Balance,
    ) -> Account {
        require!(
            env::is_valid_account_id(account_id.as_bytes()),
            "Invalid account id"
        );
        self.internal_get_account(account_id)
            .map(|mut a| {
                a.storage_balance += storage_deposit;
                a
            })
            .unwrap_or_else(|| {
                self.internal_create_account(account_id, storage_deposit, false);
                self.internal_unwrap_account(account_id)
            })
    }

    pub fn internal_create_account(
        &mut self,
        account_id: &str,
        storage_deposit: Balance,
        registration_only: bool,
    ) {
        let min_balance = self.storage_balance_bounds().min.0;
        if storage_deposit < min_balance {
            env::panic_str("The attached deposit is less than the mimimum storage balance");
        }

        let mut account = Account::new(self.create_node_id());
        if registration_only {
            let refund = storage_deposit - min_balance;
            if refund > 0 {
                Promise::new(env::predecessor_account_id()).transfer(refund);
            }
            account.storage_balance = min_balance;
        } else {
            account.storage_balance = storage_deposit;
        }

        account.storage_tracker.start();
        self.internal_set_node(Node::new(account.node_id, None));
        self.root_node.block_height = env::block_height();
        self.root_node
            .children
            .insert(&account_id.to_string(), &NodeValue::Node(account.node_id));
        require!(
            !self.internal_set_account(Account::new(account.node_id)),
            "Internal bug. Account already exists."
        );
        account.storage_tracker.stop();
        self.internal_set_account(account);
    }

    pub fn internal_set_account(&mut self, mut account: Account) -> bool {
        if account.storage_tracker.bytes_added >= account.storage_tracker.bytes_released {
            let extra_bytes_used =
                account.storage_tracker.bytes_added - account.storage_tracker.bytes_released;
            account.used_bytes += extra_bytes_used;
            account.assert_storage_covered();
        } else {
            let bytes_released =
                account.storage_tracker.bytes_released - account.storage_tracker.bytes_added;
            assert!(
                account.used_bytes >= bytes_released,
                "Internal storage accounting bug"
            );
            account.used_bytes -= bytes_released;
        }
        account.storage_tracker.bytes_released = 0;
        account.storage_tracker.bytes_added = 0;
        let node_id = account.node_id;
        self.accounts.insert(&node_id, &account.into()).is_some()
    }

    pub fn internal_storage_balance_of(&self, account_id: &AccountId) -> Option<StorageBalance> {
        self.internal_get_account(account_id.as_str())
            .map(|storage| StorageBalance {
                total: storage.storage_balance.into(),
                available: U128(
                    storage.storage_balance
                        - Balance::from(storage.used_bytes) * env::storage_byte_cost(),
                ),
            })
    }
}

#[near_bindgen]
impl StorageManagement for Contract {
    #[payable]
    fn storage_deposit(
        &mut self,
        account_id: Option<AccountId>,
        registration_only: Option<bool>,
    ) -> StorageBalance {
        self.assert_live();
        let attached_deposit: Balance = env::attached_deposit();
        let account_id = account_id
            .map(|a| a.into())
            .unwrap_or_else(|| env::predecessor_account_id());
        let account = self.internal_get_account(account_id.as_str());
        let registration_only = registration_only.unwrap_or(false);
        if let Some(mut account) = account {
            if registration_only && attached_deposit > 0 {
                Promise::new(env::predecessor_account_id()).transfer(attached_deposit);
            } else {
                account.storage_balance += attached_deposit;
                self.internal_set_account(account);
            }
        } else {
            self.internal_create_account(account_id.as_str(), attached_deposit, registration_only);
        }
        self.internal_storage_balance_of(&account_id).unwrap()
    }

    #[payable]
    fn storage_withdraw(&mut self, amount: Option<U128>) -> StorageBalance {
        self.assert_live();
        assert_one_yocto();
        let account_id = env::predecessor_account_id();
        if let Some(storage_balance) = self.internal_storage_balance_of(&account_id) {
            let amount = amount.unwrap_or(storage_balance.available).0;
            if amount > storage_balance.available.0 {
                env::panic_str("The amount is greater than the available storage balance");
            }
            if amount > 0 {
                let mut account = self.internal_unwrap_account(account_id.as_str());
                account.storage_balance -= amount;
                self.internal_set_account(account);
                Promise::new(account_id.clone()).transfer(amount);
            }
            self.internal_storage_balance_of(&account_id).unwrap()
        } else {
            env::panic_str(&format!("The account {} is not registered", &account_id));
        }
    }

    #[allow(unused_variables)]
    #[payable]
    fn storage_unregister(&mut self, force: Option<bool>) -> bool {
        self.assert_live();
        env::panic_str("The account can't be unregistered");
    }

    fn storage_balance_bounds(&self) -> StorageBalanceBounds {
        StorageBalanceBounds {
            min: U128(MIN_STORAGE_BALANCE),
            max: None,
        }
    }

    fn storage_balance_of(&self, account_id: AccountId) -> Option<StorageBalance> {
        self.internal_storage_balance_of(&account_id)
    }
}

#[near_bindgen]
impl Contract {
    /// Returns account information for accounts from a given index up to a given limit.
    pub fn get_accounts(
        &self,
        from_index: Option<u64>,
        limit: Option<u64>,
    ) -> Vec<(AccountId, Account)> {
        let keys = self.root_node.children.keys_as_vector();
        let values = self.root_node.children.values_as_vector();
        let from_index = from_index.unwrap_or(0);
        let limit = limit.unwrap_or(keys.len());
        (from_index..std::cmp::min(keys.len(), from_index + limit))
            .map(|index| {
                let node_id = match values.get(index).unwrap() {
                    NodeValue::Value(_) => {
                        unreachable!();
                    }
                    NodeValue::Node(node_id) => node_id,
                };
                let mut account: Account = self.accounts.get(&node_id).unwrap().into();
                account.node_id = node_id;
                (
                    AccountId::try_from(keys.get(index).unwrap()).unwrap(),
                    account,
                )
            })
            .collect()
    }

    /// Returns the number of accounts
    pub fn get_account_count(&self) -> u32 {
        self.root_node.children.len() as _
    }
}
