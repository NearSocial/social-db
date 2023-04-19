# `0.10.0`

- Remove `1` yoctoNEAR requirement when writing from the matching predecessor. It allows to write under your account with a simple limited access key without requesting the permission first.
- Introduce Shared Storage Pools. Pools can share available storage bytes to new or existing accounts. The total shared storage may exceed the prepaid storage, but the prepaid storage is still limits the total amount of bytes that can be written by the accounts using this shared pool. It allows optimistically provide storage quotas.
  - Add `pub fn shared_storage_pool_deposit(&mut self, owner_id: Option<AccountId>)` - requires at least 100N in deposit. This creates a shared storage pool for a given owner_id (or predecessor) with the attached deposit. If the pool already exists for this owner, then the deposit is added to the existing pool.
  - Add `pub fn share_storage(&mut self, account_id: AccountId, max_bytes: StorageUsage)` that should be called by the pool owner. It will share the storage from the owner's pool with the given account. If the account already has shared storage, then the `max_bytes` should be higher than the existing `max_bytes`.
  - Add `pub fn get_account_storage(&self, account_id: AccountId) -> Option<StorageView>` that returns the amount of used bytes and the amount of bytes available. It accounts for the shared storage.

# `0.9.0`

- Fix returned values of the trailing `/` for `get` and `keys`.
- New options for `keys` API: `value_only`. If `true`, only matches keys which value is not a node. It's needed to filter out deleted entries. Since a node can't be deleted right now.

# `0.8.0`

- Ability to delete data by passing `null` leaves.
- Support for retrieving deleted data with option `return_deleted: true`.

# `0.7.0`

- Add `options` to `get` and `keys` API calls.

# `0.6.0`

- Adding ability to pause a contract and migrate it to a new genesis.
- Methods for extracting the data based on nodes. Useful for API server implementations.

# `0.5.1`

- Decrease storage requirement to `2000` bytes from `10000` bytes.

# `0.5.0`

- Restrict `**` to be only a suffix.
- Added `keys` to retrieve matched keys including non-leaf nodes.
- Limit key length to `256` characters.

# `0.4.0`

- `README.md` and `CHANGELOG.md` are added
- Added `is_write_permission_granted` to check write permission access.
