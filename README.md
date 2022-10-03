# Social DB

## Notes

### Testnet account ID

Deployed at `v1.social08.testnet`

https://explorer.testnet.near.org/accounts/v1.social08.testnet

### Mainnet account ID

Deployed at `social.near`

https://explorer.near.org/accounts/social.near

### About empty keys

If a leaf value was originally string, e.g. `name`

```json
{
  "alex.near": {
    "profile": {
      "name": "Alex"
    }
  }
}
```

And later some value was added under `name`, the name will be transformed to a node, and the value
will be moved to an empty key under this node. E.g. `name/foo = "bar"` is added.

```json
{
  "alex.near": {
    "profile": {
      "name": {
        "": "Alex",
        "foo": "bar"
      }
    }
  }
}
```

## Changelog

See [CHANGELOG.md](CHANGELOG.md)

## API

### Storing data

The top level keys of the objects should be account IDs under which the data is stored. Those accounts are covering storage for the underlying data.

The predecessor_id or the signer public key should have permission to write under those keys.
If the predecessor_id matches the top level key, then it can write any data under that key, as long as it has a permission or at least 1 yoctoNEAR is attached.

The attached deposit will be transferred to the first key. If the account doesn't exist, it will be created (the predecessor_id should match).

```rust
#[payable]
pub fn set(&mut self, data: Value);
```

- `data` is an object to store.

Examples:

```js
set({
  data: {
    "alex.near": {
      "profile": {
        "name": "Alex",
        "image": {
          "url": "https://gkfjklgdfjkldfg"
        }
      },
    }
  }
})

set({
  data: {
    "alex.near": {
      "graph": {
        "follow": {
          "root.near": "",
          "bob.near": "",
        }
      }
    }
  }
})
```

### Reading data

```rust
pub fn get(self, keys: Vec<String>) -> Value;
```

- `keys` - an array of key patterns to return.

Returns the aggregated JSON object.

Examples:

```js
// TBD "alex.near/profile/[name,url,image_url]",
get({keys: [
  "alex.near/profile/*",
  "alex.near/profile/**",
  "alex.near/profile/url",
  "alex.near/profile",
  "bob.near/profile/*",
  "alex.near/graph/follow/*",
]})
```

### Permissions

See https://explorer.testnet.near.org/transactions/3c7h9da1z5Px4JumNDsRaJtCDQaZHG46dsc2SnAj5LHx\

```rust
#[payable]
pub fn grant_write_permission(
    &mut self,
    predecessor_id: Option<AccountId>,
    public_key: Option<PublicKey>,
    keys: Vec<String>,
);
```

```rust
/// Returns true if the permission is granted for a given account ID or a given public_key to
/// any prefix of the key.
pub fn is_write_permission_granted(
    &self,
    predecessor_id: Option<AccountId>,
    public_key: Option<PublicKey>,
    key: String,
) -> bool;
```

### Debugging

```bash
export CONTRACT_ID=v1.social08.testnet
export ACCOUNT_ID=eugenethedream
# Full contract data
near view $CONTRACT_ID get '{"keys":["**"]}'
# Full account's data
near view $CONTRACT_ID get '{"keys":["'$ACCOUNT_ID'/**"]}'
```



