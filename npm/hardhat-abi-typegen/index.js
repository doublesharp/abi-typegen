const plugin = {
  id: "@0xdoublesharp/hardhat-abi-typegen",
  npmPackage: "@0xdoublesharp/hardhat-abi-typegen",
  hookHandlers: {
    config: () => import("./config-hooks.js"),
    solidity: () => import("./solidity-hooks.js"),
  },
};

export default plugin;
