use crate::*;

const MINIMUM_SHARED_STORAGE_BALANCE: Balance = 100 * 10u128.pow(24);

#[derive(BorshSerialize, BorshDeserialize)]
pub enum VSharedStoragePool {
    Current(SharedStoragePool),
}

impl From<VSharedStoragePool> for SharedStoragePool {
    fn from(v: VSharedStoragePool) -> Self {
        match v {
            VSharedStoragePool::Current(c) => c,
        }
    }
}

impl From<SharedStoragePool> for VSharedStoragePool {
    fn from(c: SharedStoragePool) -> Self {
        VSharedStoragePool::Current(c)
    }
}

#[derive(BorshSerialize, BorshDeserialize, Serialize)]
#[serde(crate = "near_sdk::serde")]
pub struct SharedStoragePool {
    #[serde(with = "u128_dec_format")]
    pub storage_balance: Balance,
    pub used_bytes: StorageUsage,
    /// The sum of the maximum number of bytes of storage that are shared between all accounts.
    /// This number might be larger than the total number of bytes of storage that are available.
    pub shared_bytes: StorageUsage,
}

impl SharedStoragePool {
    pub fn new() -> Self {
        Self {
            storage_balance: 0,
            used_bytes: 0,
            shared_bytes: 0,
        }
    }

    pub fn available_bytes(&self) -> StorageUsage {
        let max_bytes = (self.storage_balance / env::storage_byte_cost()) as StorageUsage;
        max_bytes - self.used_bytes
    }
}

#[derive(BorshSerialize, BorshDeserialize, Serialize, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct AccountSharedStorage {
    /// The maximum number of bytes of storage from the shared storage pool.
    pub max_bytes: StorageUsage,
    /// The amount of storage used by the account from the shared storage pool.
    pub used_bytes: StorageUsage,

    /// The account ID of the storage pool that donated storage to the account.
    pub pool_id: AccountId,
}

impl AccountSharedStorage {
    pub fn available_bytes(&self, shared_storage_pool: &SharedStoragePool) -> StorageUsage {
        std::cmp::min(
            self.max_bytes - self.used_bytes,
            shared_storage_pool.available_bytes(),
        )
    }
}

#[derive(Serialize)]
#[serde(crate = "near_sdk::serde")]
pub struct StorageView {
    pub used_bytes: StorageUsage,
    pub available_bytes: StorageUsage,
}

impl Contract {
    pub fn internal_get_shared_storage_pool(
        &self,
        owner_id: &AccountId,
    ) -> Option<SharedStoragePool> {
        self.shared_storage_pools.get(&owner_id).map(|p| p.into())
    }

    pub fn internal_unwrap_shared_storage_pool(&self, owner_id: &AccountId) -> SharedStoragePool {
        self.internal_get_shared_storage_pool(owner_id)
            .expect("Shared storage pool not found")
    }

    pub fn internal_set_shared_storage_pool(
        &mut self,
        owner_id: &AccountId,
        shared_storage_pool: SharedStoragePool,
    ) {
        self.shared_storage_pools
            .insert(&owner_id, &shared_storage_pool.into());
    }
}

#[near_bindgen]
impl Contract {
    pub fn get_shared_storage_pool(&self, owner_id: AccountId) -> Option<SharedStoragePool> {
        self.internal_get_shared_storage_pool(&owner_id)
    }

    #[payable]
    pub fn shared_storage_pool_deposit(&mut self, owner_id: Option<AccountId>) {
        let attached_deposit = env::attached_deposit();
        let owner_id = owner_id.unwrap_or_else(env::predecessor_account_id);
        if attached_deposit < MINIMUM_SHARED_STORAGE_BALANCE {
            env::panic_str(
                format!(
                    "Attached deposit is less than the minimum amount {}",
                    MINIMUM_SHARED_STORAGE_BALANCE
                )
                .as_str(),
            );
        }
        let mut storage_tracker = StorageTracker::default();
        let mut shared_storage_pool = self
            .internal_get_shared_storage_pool(&owner_id)
            .unwrap_or_else(|| SharedStoragePool::new());
        shared_storage_pool.storage_balance += attached_deposit;
        storage_tracker.start();
        self.internal_set_shared_storage_pool(&owner_id, shared_storage_pool);
        storage_tracker.stop();
        if storage_tracker.bytes_added > storage_tracker.bytes_released {
            let mut shared_storage_pool = self.internal_unwrap_shared_storage_pool(&owner_id);
            shared_storage_pool.used_bytes +=
                storage_tracker.bytes_added - storage_tracker.bytes_released;
            self.internal_set_shared_storage_pool(&owner_id, shared_storage_pool);
        }
        storage_tracker.clear();
    }

    pub fn share_storage(&mut self, account_id: AccountId, max_bytes: StorageUsage) {
        if max_bytes < MIN_STORAGE_BYTES {
            env::panic_str(format!("Max bytes must be at least {}", MIN_STORAGE_BYTES).as_str());
        }
        let pool_id = env::predecessor_account_id();
        let account = self.internal_get_account(account_id.as_str());
        let mut shared_storage_pool = self.internal_unwrap_shared_storage_pool(&pool_id);
        let available_bytes = shared_storage_pool.available_bytes();
        if available_bytes < max_bytes {
            env::panic_str("Not enough storage available in the shared storage pool");
        }
        if let Some(mut account) = account {
            // The account already exists.

            if let Some(current_shared_storage) = account.shared_storage.take() {
                // The account is already using shared storage.

                if max_bytes < current_shared_storage.used_bytes {
                    env::panic_str("Max bytes must be greater than or equal to used bytes");
                }
                if current_shared_storage.pool_id == pool_id {
                    // The account is already using shared storage from the same pool.

                    if max_bytes <= current_shared_storage.max_bytes {
                        env::panic_str("Max bytes must be greater than the current max bytes");
                    }

                    let new_shared_storage = AccountSharedStorage {
                        max_bytes,
                        used_bytes: current_shared_storage.used_bytes,
                        pool_id,
                    };

                    shared_storage_pool.shared_bytes +=
                        max_bytes - current_shared_storage.max_bytes;
                    self.internal_set_shared_storage_pool(
                        &new_shared_storage.pool_id,
                        shared_storage_pool,
                    );

                    account.shared_storage = Some(new_shared_storage);
                } else {
                    // The account is already using shared storage from a different pool.

                    let mut current_shared_storage_pool =
                        self.internal_unwrap_shared_storage_pool(&current_shared_storage.pool_id);
                    let current_available_bytes =
                        current_shared_storage.available_bytes(&current_shared_storage_pool);
                    let mut new_shared_storage = AccountSharedStorage {
                        max_bytes,
                        used_bytes: 0,
                        pool_id,
                    };

                    let new_available_bytes =
                        new_shared_storage.available_bytes(&shared_storage_pool);
                    if new_available_bytes
                        < current_shared_storage.used_bytes
                            + current_available_bytes
                            + MIN_STORAGE_BYTES
                    {
                        env::panic_str(format!(
                            "The difference between the new available bytes and the current available bytes must be at least {}", MIN_STORAGE_BYTES).as_str()
                        );
                    }

                    current_shared_storage_pool.used_bytes -= current_shared_storage.used_bytes;
                    current_shared_storage_pool.shared_bytes -= current_shared_storage.max_bytes;
                    self.internal_set_shared_storage_pool(
                        &current_shared_storage.pool_id,
                        current_shared_storage_pool,
                    );

                    new_shared_storage.used_bytes = current_shared_storage.used_bytes;
                    shared_storage_pool.used_bytes += new_shared_storage.used_bytes;
                    shared_storage_pool.shared_bytes += new_shared_storage.max_bytes;
                    self.internal_set_shared_storage_pool(
                        &new_shared_storage.pool_id,
                        shared_storage_pool,
                    );

                    account.shared_storage = Some(new_shared_storage)
                }
            } else {
                // The account is not using shared storage.

                shared_storage_pool.shared_bytes += max_bytes;
                self.internal_set_shared_storage_pool(&pool_id, shared_storage_pool);

                account.shared_storage = Some(AccountSharedStorage {
                    max_bytes,
                    used_bytes: 0,
                    pool_id,
                });
            }
            // Custom account saving logic to measure the change in the shared storage.
            let mut storage_tracker = StorageTracker::default();
            storage_tracker.start();
            self.internal_set_account(account);
            storage_tracker.stop();
            let mut account = self.internal_unwrap_account(account_id.as_str());
            account.storage_tracker.consume(&mut storage_tracker);
            self.internal_set_account(account);
        } else {
            // The account does not exist.

            shared_storage_pool.shared_bytes += max_bytes;
            self.internal_set_shared_storage_pool(&pool_id, shared_storage_pool);

            self.internal_create_account_from_shared_storage(
                account_id.as_str(),
                max_bytes,
                pool_id.clone(),
            );
        }
    }

    /// Returns the storage usage of the given account in bytes and accounts for the shared storage.
    pub fn get_account_storage(&self, account_id: AccountId) -> Option<StorageView> {
        self.internal_get_account(account_id.as_str())
            .map(|account| {
                let available_shared_bytes = account
                    .shared_storage
                    .as_ref()
                    .map(|s| {
                        let pool = self.internal_unwrap_shared_storage_pool(&s.pool_id);
                        s.available_bytes(&pool)
                    })
                    .unwrap_or(0);
                let used_shared_bytes = account
                    .shared_storage
                    .as_ref()
                    .map(|s| s.used_bytes)
                    .unwrap_or(0);
                let available_bytes = (account.storage_balance / env::STORAGE_PRICE_PER_BYTE)
                    as u64
                    - (account.used_bytes - used_shared_bytes);

                StorageView {
                    used_bytes: account.used_bytes,
                    available_bytes: available_bytes + available_shared_bytes,
                }
            })
    }
}
