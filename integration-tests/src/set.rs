mod get_workspace_dir;

use crate::get_workspace_dir::get_workspace_dir;
use anyhow::Result;
use serde_json::json;
use std::collections::HashMap;
use std::fs;
use near_workspaces::network::Sandbox;
use near_workspaces::{Account, Contract, Worker};
use near_workspaces::types::NearToken;

static CONTRACT_WASM_FILEPATH: &str = "res/social_db_local.wasm";

//NEAR_ENABLE_SANDBOX_LOG = 1
/// Tests the `set` method.
#[tokio::main]
async fn main() -> Result<()> {
    test_set_method().await?;
    test_set_method_and_refund().await?;
    test_set_method_and_refund_with_existing_deposit().await?;
    Ok(())
}

/// Sanity check that we can `set` and `get` a value.
async fn test_set_method() -> Result<()> {
    let (_, contract, user) = init_contract_and_user().await?;

    let user_id = user.id().to_string();
    let name = "Alice";
    let args = json!({
        "data": {
            &user_id: {
                "profile": {
                    "name": name,
                },
            }
        }
    });

    user.call(contract.id(), "set")
        .args_json(args)
        .deposit(NearToken::from_yoctonear(100_000_000_000_000_000_000_000u128))
        .transact()
        .await?
        .into_result()?;

    let name_key = format!("{user_id}/profile/name");
    let result = user
        .view(contract.id(), "get")
        .args_json(json!({ "keys": [name_key] }))
        .await?
        .json::<HashMap<String, HashMap<String, HashMap<String, String>>>>()?;
    let result_name = result
        .get(&user_id)
        .unwrap()
        .get("profile")
        .unwrap()
        .get("name")
        .unwrap();

    assert_eq!(name, result_name);

    Ok(())
}

/// Test that if a user requests a refund, they receive it.
async fn test_set_method_and_refund() -> Result<()> {
    let (_, contract, user) = init_contract_and_user().await?;

    let user_id = user.id().to_string();
    let name = "Alice";
    let args = json!({
        "data": {
            &user_id: {
                "profile": {
                    "name": name,
                },
            }
        },
        "options": {
            "refund_unused_deposit": true
        }
    });

    let prev_balance = user.view_account().await?.balance;
    let deposit = NearToken::from_near(1);

    user.call(contract.id(), "set")
        .args_json(args)
        .deposit(deposit)
        .transact()
        .await?
        .into_result()?;

    let post_balance = user.view_account().await?.balance;

    // Check that the refund isn't more than the deposit. Because of gas fees post balance must be
    // less than prev balance.
    assert!(post_balance < prev_balance);
    // Check that the refund is received.
    assert!(post_balance > prev_balance.checked_sub(deposit).unwrap());

    Ok(())
}

/// Tests that when two users fund an account and only the second requests a refund:
/// 1. First user doesn't receive refund.
/// 2. Second user does receive refund.
/// 3. Second user doesn't receive more than what they put in.
async fn test_set_method_and_refund_with_existing_deposit() -> Result<()> {
    let (worker, contract, first_user) = init_contract_and_user().await?;

    let key = first_user.id().to_string();
    let args = json!({
        "data": {
            &key: {
                "profile": {
                    "name": "Alice",
                },
            }
        },
    });

    let deposit = NearToken::from_near(1);

    let first_prev_balance = first_user.view_account().await?.balance;
    first_user
        .call(contract.id(), "set")
        .args_json(args)
        .deposit(deposit)
        .transact()
        .await?
        .into_result()?;

    let second_user = worker.dev_create_account().await?;

    let args = json!({
        "data": {
            &key: {}
        },
        "options": {
            "refund_unused_deposit": true
        }
    });

    let second_prev_balance = second_user.view_account().await?.balance;
    second_user
        .call(contract.id(), "set")
        .args_json(args)
        .deposit(deposit)
        .transact()
        .await?
        .into_result()?;
    let first_post_balance = first_user.view_account().await?.balance;
    let second_post_balance = second_user.view_account().await?.balance;

    // Make sure first user didn't receive refund.
    assert!(first_post_balance < first_prev_balance.checked_sub(deposit).unwrap());
    // Make sure second user did receive refund.
    assert!(second_post_balance > second_prev_balance.checked_sub(deposit).unwrap());
    // Make sure second user didn't receive more than what they put in.
    assert!(second_post_balance < second_prev_balance);

    Ok(())
}

async fn init_contract_and_user() -> Result<(Worker<Sandbox>, Contract, Account)> {
    let workspace_dir = get_workspace_dir();
    let wasm_filepath = workspace_dir.join(CONTRACT_WASM_FILEPATH);

    // Create a sandboxed environment.
    // NOTE: Each call will create a new sandboxed environment
    let worker = near_workspaces::sandbox().await?;
    // or for testnet:
    //let worker = near_workspaces::testnet().await?;
    let wasm = fs::read(wasm_filepath)?;
    // 测试前编译合约
    let wasm = near_workspaces::compile_project(wasm_filepath).await?;

    let contract = worker.dev_deploy(&wasm).await?;
    contract.call("new").transact().await?.into_result()?;
    contract
        .as_account()
        .call(contract.id(), "set_status")
        .args_json(json!({
            "status": "Live"
        }))
        .transact()
        .await?
        .into_result()?;

    let account = worker.dev_create_account().await?;
    let user = account
        .create_subaccount("alice")
        .initial_balance(NearToken::from_near(30))
        .transact()
        .await?
        .into_result()?;
    Ok((worker, contract, user))
}
