const DEFAULT_TYPEGEN_CONFIG = {
  out: "src/generated",
  target: "viem",
  wrappers: true,
  contracts: [],
  exclude: [],
};

function resolveTypegenConfig(typegen = {}) {
  return {
    out: typegen.out || DEFAULT_TYPEGEN_CONFIG.out,
    target: typegen.target || DEFAULT_TYPEGEN_CONFIG.target,
    wrappers: typegen.wrappers !== false,
    contracts: typegen.contracts || DEFAULT_TYPEGEN_CONFIG.contracts,
    exclude: typegen.exclude || DEFAULT_TYPEGEN_CONFIG.exclude,
  };
}

function validateString(value, path, errors) {
  if (value !== undefined && typeof value !== "string") {
    errors.push({ path, message: "Expected a string" });
  }
}

function validateStringArray(value, path, errors) {
  if (
    value !== undefined &&
    (!Array.isArray(value) || value.some((item) => typeof item !== "string"))
  ) {
    errors.push({ path, message: "Expected an array of strings" });
  }
}

function validateTarget(value, errors) {
  if (
    value !== undefined &&
    typeof value !== "string" &&
    (!Array.isArray(value) || value.some((item) => typeof item !== "string"))
  ) {
    errors.push({
      path: ["typegen", "target"],
      message: "Expected a string or an array of strings",
    });
  }
}

export default async () => ({
  validateUserConfig: async (userConfig) => {
    const errors = [];
    const typegen = userConfig.typegen;

    if (typegen === undefined) {
      return errors;
    }

    if (typeof typegen !== "object" || typegen === null || Array.isArray(typegen)) {
      errors.push({ path: ["typegen"], message: "Expected an object" });
      return errors;
    }

    validateString(typegen.out, ["typegen", "out"], errors);
    validateTarget(typegen.target, errors);

    if (typegen.wrappers !== undefined && typeof typegen.wrappers !== "boolean") {
      errors.push({ path: ["typegen", "wrappers"], message: "Expected a boolean" });
    }

    validateStringArray(typegen.contracts, ["typegen", "contracts"], errors);
    validateStringArray(typegen.exclude, ["typegen", "exclude"], errors);

    return errors;
  },
  resolveUserConfig: async (userConfig, resolveConfigurationVariable, next) => {
    const resolvedConfig = await next(userConfig, resolveConfigurationVariable);
    return {
      ...resolvedConfig,
      typegen: resolveTypegenConfig(userConfig.typegen),
    };
  },
});
