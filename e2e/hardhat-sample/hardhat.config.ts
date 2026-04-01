import { HardhatUserConfig } from "hardhat/config";
import "@typechain/hardhat";
import "@0xdoublesharp/hardhat-abi-typegen";

const config: HardhatUserConfig = {
  solidity: "0.8.34",
  typechain: {
    outDir: "typechain-types",
    target: "ethers-v6",
  },
  typegen: {
    out: "abi-typegen-out",
    target: "viem",
  },
};

export default config;
