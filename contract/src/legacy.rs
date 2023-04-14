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

impl From<AccountV0> for Account {
    fn from(c: AccountV0) -> Self {
        Self {
            storage_balance: c.storage_balance,
            used_bytes: c.used_bytes,
            permissions: c.permissions,
            ..Default::default()
        }
    }
}
