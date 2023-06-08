use crate::*;

#[near_bindgen]
impl Contract {
    // A method to migrate a state during the contract upgrade.
    #[private]
    #[init(ignore_state)]
    pub fn migrate_state() -> Self {
        let old_contract: Self = env::state_read().expect("Old state doesn't exist");
        old_contract
    }

    #[private]
    pub fn genesis_init_node_count(&mut self, node_count: u32) {
        self.assert_genesis();
        self.node_count = node_count;
    }

    #[private]
    pub fn genesis_init_nodes(&mut self, nodes: Vec<PartialNode>) {
        self.assert_genesis();
        for node in nodes {
            if node.node_id == 0 {
                populate_node(&mut self.root_node, node);
            } else {
                let mut current_node = self
                    .internal_get_node(node.node_id)
                    .unwrap_or_else(|| Node::new(node.node_id, None));
                populate_node(&mut current_node, node);
                self.internal_set_node(current_node);
            }
        }
    }

    #[private]
    pub fn genesis_init_accounts(&mut self, accounts: Vec<(AccountId, PartialAccount)>) {
        self.assert_genesis();
        for (account_id, account) in accounts {
            let mut current_account = match self
                .root_node
                .children
                .get(&account_id.to_string())
                .expect("Missing account node, make sure initializing nodes first")
            {
                NodeValue::Node(node_id) => {
                    let mut account: Account = self
                        .accounts
                        .get(&node_id)
                        .map(|va| va.into())
                        .unwrap_or_else(|| Account::new(node_id));
                    account.node_id = node_id;
                    account
                }
                _ => env::panic_str("Unexpected account key. The value is not a node."),
            };
            assert_eq!(
                current_account.node_id, account.node_id,
                "Account Node ID mismatch"
            );
            current_account.storage_balance = account.storage_balance;
            current_account.used_bytes = account.used_bytes;
            current_account.permissions.extend(account.permissions);
            self.accounts
                .insert(&account.node_id, &current_account.into());
        }
    }
}

fn populate_node(node: &mut Node, partial_node: PartialNode) {
    for (key, value) in partial_node.children {
        node.children.insert(&key, &value.into_current_height());
    }
}

impl Contract {
    pub fn assert_genesis(&self) {
        require!(
            matches!(self.status, ContractStatus::Genesis),
            "The status should be set to Genesis"
        );
    }
}
