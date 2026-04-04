import "hardhat/types/config";

declare module "hardhat/types/config" {
  interface HardhatUserConfig {
    typegen?: {
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
    };
  }

  interface HardhatConfig {
    typegen: {
      out: string;
      target: string | string[];
      wrappers: boolean;
      contracts: string[];
      exclude: string[];
    };
  }
}
