const nearAPI = require("near-api-js");
const path = require("path");
const homedir = require("os").homedir();
const Big = require("big.js");

const credentialsPath = path.join(homedir, ".near-credentials");
const keyStore = new nearAPI.keyStores.UnencryptedFileSystemKeyStore(
  credentialsPath
);

const GasBoat = Big(10).pow(12).mul(300).toFixed(0);

// const config = {
//   keyStore,
//   networkId: "testnet",
//   nodeUrl: "https://rpc.testnet.near.org",
//   inputAccountId: "v0.social08.testnet",
//   outputAccountId: "v1.social08.testnet",
// };
const config = {
  keyStore,
  networkId: "mainnet",
  nodeUrl: "https://rpc.mainnet.near.org",
  inputAccountId: "db.social08.near",
  outputAccountId: "social.near",
};

(async () => {
  const near = await nearAPI.connect(config);
  const account = await near.account(config.outputAccountId);
  if (
    (await account.viewFunction(config.inputAccountId, "get_status")) !==
    "ReadOnly"
  ) {
    throw new Error("The input account is not at read-only state");
  }

  if (
    (await account.viewFunction(config.outputAccountId, "get_status")) !==
    "Genesis"
  ) {
    throw new Error("The output account is not at genesis state");
  }

  const numNodes = await account.viewFunction(
    config.inputAccountId,
    "get_node_count"
  );

  const limit = 50;
  const nodesPromises = [];
  for (let i = 0; i < numNodes; i += limit) {
    nodesPromises.push(
      account.viewFunction(config.inputAccountId, "get_nodes", {
        from_index: i,
        limit,
      })
    );
  }
  const nodes = (await Promise.all(nodesPromises)).flat();

  console.log("Num nodes: " + nodes.length);

  const numAccounts = await account.viewFunction(
    config.inputAccountId,
    "get_account_count"
  );

  const accountsPromises = [];
  for (let i = 0; i < numAccounts; i += limit) {
    accountsPromises.push(
      account.viewFunction(config.inputAccountId, "get_accounts", {
        from_index: i,
        limit,
      })
    );
  }
  const accounts = (await Promise.all(accountsPromises)).flat();

  console.log("Num accounts: " + accounts.length);

  console.log(
    "Total balance: " +
      accounts
        .reduce((s, a) => s.add(Big(a[1].storage_balance)), Big(0))
        .div(Big(10).pow(24))
        .toFixed(3) +
      " NEAR"
  );

  console.log("Initializing node count");
  await account.functionCall({
    contractId: config.outputAccountId,
    methodName: "genesis_init_node_count",
    args: { node_count: numNodes },
    gas: GasBoat,
  });

  const initLimit = 20;
  for (let i = 0; i < numNodes; i += initLimit) {
    const partialNodes = nodes.slice(i, i + initLimit);
    console.log(
      `Initializing nodes from ${i} to ${
        i + partialNodes.length
      } out of ${numNodes}`
    );
    await account.functionCall({
      contractId: config.outputAccountId,
      methodName: "genesis_init_nodes",
      args: { nodes: partialNodes },
      gas: GasBoat,
    });
  }

  for (let i = 0; i < numAccounts; i += initLimit) {
    const partialAccounts = accounts.slice(i, i + initLimit);
    console.log(
      `Initializing accounts from ${i} to ${
        i + partialAccounts.length
      } out of ${numAccounts}`
    );
    await account.functionCall({
      contractId: config.outputAccountId,
      methodName: "genesis_init_accounts",
      args: { accounts: partialAccounts },
      gas: GasBoat,
    });
  }
})().catch((e) => {
  console.error(e);
  process.exit(1);
});
