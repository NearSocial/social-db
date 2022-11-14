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
