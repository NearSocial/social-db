use crate::*;

/// Legacy version of the account, before shared storage pools were introduced.
#[derive(BorshSerialize, BorshDeserialize, Serialize)]
#[serde(crate = "near_sdk::serde")]
pub struct AccountV0 {
    #[serde(with = "u128_dec_format")]
    pub storage_balance: Balance,
    pub used_bytes: StorageUsage,
    /// Tracks all currently active permissions given by this account.
    #[serde(with = "unordered_map_expensive")]
    pub permissions: UnorderedMap<PermissionKey, Permission>,
}

/// During this migration, we introduce a new field `shared_storage` to the account, which requires
/// extra bytes of storage. We also increase the storage balance by the storage price per byte.
impl From<AccountV0> for Account {
    fn from(c: AccountV0) -> Self {
        Self {
            storage_balance: c.storage_balance + env::STORAGE_PRICE_PER_BYTE,
            used_bytes: c.used_bytes + 1,
            permissions: c.permissions,
            node_id: 0,
            storage_tracker: Default::default(),
            shared_storage: None,
        }
    }
}
