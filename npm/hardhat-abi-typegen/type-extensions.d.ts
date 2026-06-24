import "hardhat/types/config";
import "hardhat/config";

interface AbiTypegenUserConfig {
  /** Output directory for generated TypeScript files. Default: "src/generated" */
  out?: string;
  /** Generation target. Default: "viem". Accepts a single target, comma-separated targets, or an array of targets. */
  target?: string | string[];
  /** Emit typed wrapper files when supported. Default: true */
  wrappers?: boolean;
  /** Limit generation to named contracts. Default: [] (all) */
  contracts?: string[];
  /** Exclude contracts matching glob patterns. Default: [] */
  exclude?: string[];
}

interface AbiTypegenConfig {
  out: string;
  target: string | string[];
  wrappers: boolean;
  contracts: string[];
  exclude: string[];
}

declare module "hardhat/types/config" {
  export interface HardhatUserConfig {
    typegen?: AbiTypegenUserConfig;
  }

  export interface HardhatConfig {
    typegen: AbiTypegenConfig;
  }
}

declare module "hardhat/config" {
  export interface HardhatUserConfig {
    typegen?: AbiTypegenUserConfig;
  }

  export interface HardhatConfig {
    typegen: AbiTypegenConfig;
  }
}

declare module "hardhat/dist/src/types/config.js" {
  export interface HardhatUserConfig {
    typegen?: AbiTypegenUserConfig;
  }

  export interface HardhatConfig {
    typegen: AbiTypegenConfig;
  }
}
