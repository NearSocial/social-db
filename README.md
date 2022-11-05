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

Arguments:
- `data` is an object to store. The leaf values should be strings or null values. String values will be added, while null values will be deleted.

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

Returns the data for a list of given key patterns.
It takes one or more path patterns as arguments, and returns the matching data.
The path pattern is a string that can contain wildcards.
For example:
- `alice.near/profile/**` will match the entire profile data of account `alice.near`.
- `alice.near/profile/*` will match all the fields of the profile, but not the nested objects.
- `alice.near/profile/name` will match only the name field of the profile.
- `*/widget/*` will match all the widgets of all the accounts.

```rust
pub struct GetOptions {
    pub with_block_height: Option<bool>,
    pub with_node_id: Option<bool>,
    pub return_deleted: Option<bool>,
}

pub fn get(self, keys: Vec<String>, options: Option<GetOptions>) -> Value;
```

Arguments:
- `keys` - an array of key patterns to return.
- `options` - optional argument to specify options.

Options:
- `with_block_height` - if true, for every value and a node will add the block height of the data with the key `:block`.
- `with_node_id` - if true, for every node will add the node index with the key `:node`.
- `return_deleted` - if true, will include deleted keys with the value `null`.

Returns the aggregated JSON object.

Examples:

```js
get({keys: ["alex.near/profile/name"]})

get({keys: ["alex.near/profile/name", "root.near/profile/name"]})

get({keys: ["alex.near/profile/name", "alex.near/profile/description"]})

get({keys: ["alex.near/profile/tags/*"]})

get({keys: ["alex.near/profile/**"]})

get({keys: ["*/widget/*"]})

get({keys: ["alex.near/profile/tags/*"], options: {return_deleted: true}})
```

### Reading keys

The `keys` method allows to get the list of keys that match the path pattern.
It's useful for querying the data without reading values.
It also has an additional `options` field that can be used to specify the return type and whether to return deleted keys.
For example:
- `alice.near/profile/*` will return the list of all the fields of the profile, but not the nested objects.
- `*/profile/image/nft` will return the list of all the accounts that have an NFT image in their profile.
- `alice.near/widget/*` with `return_deleted` option will return the list of all the widget names of the account, including the deleted ones.
- `alice.near/widget/*` with `return_type` equal to `BlockHeight` will return the list of all the widget names of the account and the value will be the block height when the widget was last updated.
- Note `**` is not supported by the `keys` method.

```rust
pub enum KeysReturnType {
    True,
    BlockHeight,
    NodeId,
}

pub struct KeysOptions {
    pub return_type: Option<KeysReturnType>,
    pub return_deleted: Option<bool>,
}

pub fn keys(self, keys: Vec<String>, options: Option<KeysOptions>) -> Value;
```

Arguments:
- `keys` - an array of key patterns to return.
- `options` - optional argument to specify options.

Options:
- `return_type` - if `BlockHeight`, will return the block height of the key instead of `true`, if `NodeId`, will return the node index of the key instead of `true`.
- `return_deleted` - if true, will include deleted keys.

Returns the aggregated JSON object.

Examples:

```js
keys({keys: ["alex.near/profile/*"]})

keys({keys: ["*/profile/image/nft"]})

keys({keys: ["alex.near/widget/*"], options: {return_deleted: true}})

keys({keys: ["alex.near/widget/*"], options: {return_type: "BlockHeight"}})
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



