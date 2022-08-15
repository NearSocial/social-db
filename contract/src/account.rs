use crate::*;
use near_contract_standards::storage_management::{
    StorageBalance, StorageBalanceBounds, StorageManagement,
};
use near_sdk::StorageUsage;
use std::convert::TryFrom;

/// 10000 bytes
const MIN_STORAGE_BALANCE: Balance = 10000u128 * env::STORAGE_PRICE_PER_BYTE;

#[derive(BorshSerialize, BorshDeserialize)]
pub struct Account {
    pub storage_balance: Balance,
    pub used_bytes: StorageUsage,
    /// Tracks all currently active permissions given by this account.
    pub permissions: UnorderedMap<PermissionKey, Permission>,
    #[borsh_skip]
    pub node_id: NodeId,
    #[borsh_skip]
    pub storage_tracker: StorageTracker,
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
            .expect("Storage for account is missing")
    }

    pub fn internal_set_account(&mut self, mut account: Account) {
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
        self.accounts.insert(&node_id, &account.into());
    }

    pub fn internal_storage_balance_of(&self, account_id: &AccountId) -> Option<StorageBalance> {
        self.internal_get_account(account_id.as_str())
            .map(|storage| StorageBalance {
                total: storage.storage_balance.into(),
                available: U128(
                    storage.storage_balance
                        - std::cmp::max(
                            Balance::from(storage.used_bytes) * env::storage_byte_cost(),
                            self.storage_balance_bounds().min.0,
                        ),
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
        let amount: Balance = env::attached_deposit();
        let account_id = account_id
            .map(|a| a.into())
            .unwrap_or_else(|| env::predecessor_account_id());
        let account = self.internal_get_account(account_id.as_str());
        let registration_only = registration_only.unwrap_or(false);
        if let Some(mut account) = account {
            if registration_only && amount > 0 {
                Promise::new(env::predecessor_account_id()).transfer(amount);
            } else {
                account.storage_balance += amount;
                self.internal_set_account(account);
            }
        } else {
            let min_balance = self.storage_balance_bounds().min.0;
            if amount < min_balance {
                env::panic_str("The attached deposit is less than the mimimum storage balance");
            }

            let mut account = Account::new(self.create_node_id());
            if registration_only {
                let refund = amount - min_balance;
                if refund > 0 {
                    Promise::new(env::predecessor_account_id()).transfer(refund);
                }
                account.storage_balance = min_balance;
            } else {
                account.storage_balance = amount;
            }

            account.storage_tracker.start();
            self.internal_set_node(Node::new(account.node_id, None));
            self.root_node.block_height = env::block_height();
            self.root_node
                .children
                .insert(&account_id.to_string(), &NodeValue::Node(account.node_id));
            self.internal_set_account(Account::new(account.node_id));
            account.storage_tracker.stop();
            self.internal_set_account(account);
        }
        self.internal_storage_balance_of(&account_id).unwrap()
    }

    #[payable]
    fn storage_withdraw(&mut self, amount: Option<U128>) -> StorageBalance {
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
    /// Helper method for debugging storage usage that ignores minimum storage limits.
    pub fn debug_storage_balance_of(&self, account_id: AccountId) -> Option<StorageBalance> {
        self.internal_get_account(account_id.as_str())
            .map(|storage| StorageBalance {
                total: storage.storage_balance.into(),
                available: U128(
                    storage.storage_balance
                        - Balance::from(storage.used_bytes) * env::storage_byte_cost(),
                ),
            })
    }

    /// Returns limited account information for accounts from a given index up to a given limit.
    /// The information includes number of shares for collateral and borrowed assets.
    /// This method can be used to iterate on the accounts for liquidation.
    pub fn get_accounts_paged(
        &self,
        from_index: Option<u64>,
        limit: Option<u64>,
    ) -> Vec<AccountId> {
        let keys = self.root_node.children.keys_as_vector();
        let from_index = from_index.unwrap_or(0);
        let limit = limit.unwrap_or(keys.len());
        (from_index..std::cmp::min(keys.len(), from_index + limit))
            .filter_map(|index| AccountId::try_from(keys.get(index).unwrap()).ok())
            .collect()
    }

    /// Returns the number of accounts
    pub fn get_num_accounts(&self) -> u32 {
        self.root_node.children.len() as _
    }
}
