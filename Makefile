# Makefile — abi-typegen development targets

.PHONY: build test check fmt lint e2e bench coverage coverage-open coverage-summary \
        fuzz fuzz-parse-artifact fuzz-config-toml fuzz-sol-type fuzz-codegen-full fuzz-barrel \
        fuzz-corpus fuzz-init-corpus

# ── Development ───────────────────────────────────────────────────────────────

build:
	cargo build --all

test:
	cargo test --all

check:
	cargo check --all

fmt:
	cargo fmt --all

lint:
	cargo clippy --all -- -D warnings

# ── End-to-end ───────────────────────────────────────────────────────────────
# Builds Solidity contracts with forge, generates TypeScript with abi-typegen,
# and type-checks the output with tsc.
# Requires: forge, pnpm, cargo

e2e: e2e-foundry e2e-hardhat ## Run all e2e tests

e2e-foundry: build ## E2E: Foundry → abi-typegen → tsc
	cd e2e/foundry-sample && forge build
	cd e2e/foundry-sample && ../../target/debug/abi-typegen generate --artifacts ./out --out ./src/generated --target viem
	cd e2e/foundry-sample && pnpm exec tsc --noEmit
	cd e2e/foundry-sample && ../../target/debug/abi-typegen generate --artifacts ./out --out ./src/generated-zod --target zod
	cd e2e/foundry-sample && pnpm exec tsc --noEmit -p tsconfig.zod.json
	cd e2e/foundry-sample && ../../target/debug/abi-typegen generate --artifacts ./out --out ./src/generated-solidity --target solidity
	cd e2e/foundry-sample && forge build --contracts ./solidity-validation --out ./out-solidity-validation
	@echo "e2e-foundry: pass"

e2e-hardhat: build ## E2E: Hardhat → abi-typegen --hardhat → tsc
	cd e2e/hardhat-sample && pnpm exec hardhat compile --quiet
	cd e2e/hardhat-sample && ../../target/debug/abi-typegen generate --hardhat --artifacts ./artifacts/contracts --out ./abi-typegen-out --target viem
	@echo "e2e-hardhat: pass"

bench: ## Benchmark abi-typegen vs TypeChain (10 runs each)
	./e2e/bench.sh 10

# ── Fuzz testing ──────────────────────────────────────────────────────────────
# Uses cargo-fuzz (libFuzzer, requires nightly).  Each target runs indefinitely;
# stop with Ctrl-C.  Crashes are saved to fuzz/artifacts/<target>/.
#
# Directory layout:
#   fuzz/seeds/<target>/   — hand-curated seeds, committed to git
#   fuzz/corpus/<target>/  — auto-discovered by libFuzzer, gitignored (local only)
#   fuzz/artifacts/<tgt>/  — crash inputs found by the fuzzer, committed to git
#   fuzz/logs/             — run logs, gitignored
#
# Run a single target:   make fuzz-parse-artifact
# Run all in parallel:   make fuzz

# Single-target workers: available CPUs minus one, minimum one.
_NCPUS       := $(shell sysctl -n hw.logicalcpu 2>/dev/null || nproc 2>/dev/null || echo 2)
FUZZ_JOBS    := $(shell j=$$(( $(_NCPUS) - 1 )); [ "$$j" -lt 1 ] && echo 1 || echo $$j)
# Parallel workers: distribute CPUs across all 5 targets evenly, minimum one each.
_FUZZ_NTGTS  := 5
FUZZ_PAR     := $(shell j=$$(( $(_NCPUS) / $(_FUZZ_NTGTS) )); [ "$$j" -lt 1 ] && echo 1 || echo $$j)

fuzz: fuzz-init-corpus ## Run all fuzz targets in parallel, distributing all CPUs (Ctrl-C to stop)
	@mkdir -p fuzz/logs
	@echo "Fuzzing $(_FUZZ_NTGTS) targets in parallel — $(FUZZ_PAR) worker(s) each ($(_NCPUS) total CPUs)"
	@echo "Logs → fuzz/logs/  |  Crashes → fuzz/artifacts/"
	cargo +nightly fuzz run fuzz_parse_artifact fuzz/corpus/fuzz_parse_artifact fuzz/seeds/fuzz_parse_artifact -- -workers=$(FUZZ_PAR) > fuzz/logs/fuzz_parse_artifact.log 2>&1 &
	cargo +nightly fuzz run fuzz_config_toml    fuzz/corpus/fuzz_config_toml    fuzz/seeds/fuzz_config_toml    -- -workers=$(FUZZ_PAR) > fuzz/logs/fuzz_config_toml.log 2>&1 &
	cargo +nightly fuzz run fuzz_sol_type_str   fuzz/corpus/fuzz_sol_type_str   fuzz/seeds/fuzz_sol_type_str   -- -workers=$(FUZZ_PAR) > fuzz/logs/fuzz_sol_type_str.log 2>&1 &
	cargo +nightly fuzz run fuzz_codegen_full   fuzz/corpus/fuzz_codegen_full   fuzz/seeds/fuzz_codegen_full   -- -workers=$(FUZZ_PAR) > fuzz/logs/fuzz_codegen_full.log 2>&1 &
	cargo +nightly fuzz run fuzz_barrel         fuzz/corpus/fuzz_barrel         fuzz/seeds/fuzz_barrel         -- -workers=$(FUZZ_PAR) > fuzz/logs/fuzz_barrel.log 2>&1 &
	wait

fuzz-parse-artifact: fuzz-init-corpus ## Fuzz parse_artifact (full JSON pipeline)
	@echo "Fuzzing fuzz_parse_artifact with $(FUZZ_JOBS) worker(s)"
	cargo +nightly fuzz run fuzz_parse_artifact fuzz/corpus/fuzz_parse_artifact fuzz/seeds/fuzz_parse_artifact -- -workers=$(FUZZ_JOBS)

fuzz-config-toml: fuzz-init-corpus ## Fuzz Config::from_toml_str
	@echo "Fuzzing fuzz_config_toml with $(FUZZ_JOBS) worker(s)"
	cargo +nightly fuzz run fuzz_config_toml fuzz/corpus/fuzz_config_toml fuzz/seeds/fuzz_config_toml -- -workers=$(FUZZ_JOBS)

fuzz-sol-type: fuzz-init-corpus ## Fuzz parse_type_string (Solidity type parser)
	@echo "Fuzzing fuzz_sol_type_str with $(FUZZ_JOBS) worker(s)"
	cargo +nightly fuzz run fuzz_sol_type_str fuzz/corpus/fuzz_sol_type_str fuzz/seeds/fuzz_sol_type_str -- -workers=$(FUZZ_JOBS)

fuzz-codegen-full: fuzz-init-corpus ## Fuzz full codegen pipeline (parse → viem + ethers + barrel)
	@echo "Fuzzing fuzz_codegen_full with $(FUZZ_JOBS) worker(s)"
	cargo +nightly fuzz run fuzz_codegen_full fuzz/corpus/fuzz_codegen_full fuzz/seeds/fuzz_codegen_full -- -workers=$(FUZZ_JOBS)

fuzz-barrel: fuzz-init-corpus ## Fuzz barrel/index generator with arbitrary contract name lists
	@echo "Fuzzing fuzz_barrel with $(FUZZ_JOBS) worker(s)"
	cargo +nightly fuzz run fuzz_barrel fuzz/corpus/fuzz_barrel fuzz/seeds/fuzz_barrel -- -workers=$(FUZZ_JOBS)

fuzz-init-corpus: ## Create local corpus dirs (gitignored); must run before fuzzing
	mkdir -p fuzz/corpus/fuzz_parse_artifact fuzz/corpus/fuzz_config_toml fuzz/corpus/fuzz_sol_type_str \
	         fuzz/corpus/fuzz_codegen_full fuzz/corpus/fuzz_barrel

# ── Coverage ──────────────────────────────────────────────────────────────────
# Measures test-suite coverage of the production codebase.
# Requires cargo-llvm-cov:  cargo install cargo-llvm-cov --locked
#
# Run:   make coverage         → HTML report in coverage/html/index.html
# Run:   make coverage-open    → generate + open in browser
# Run:   make coverage-summary → print summary to stdout

coverage: ## HTML report in coverage/html/index.html
	cargo llvm-cov --workspace --html --output-dir coverage

coverage-open: ## Generate HTML coverage report and open in browser
	cargo llvm-cov --workspace --html --output-dir coverage --open

coverage-summary: ## Print line/branch coverage summary to stdout
	cargo llvm-cov --workspace --summary-only

fuzz-corpus: ## Update seeds from test fixtures
	cp tests/fixtures/erc20.json    fuzz/seeds/fuzz_parse_artifact/erc20.json
	cp tests/fixtures/vault.json    fuzz/seeds/fuzz_parse_artifact/vault.json
	cp tests/fixtures/minimal.json  fuzz/seeds/fuzz_parse_artifact/minimal.json
	cp tests/fixtures/erc20.json    fuzz/seeds/fuzz_codegen_full/erc20.json
	cp tests/fixtures/vault.json    fuzz/seeds/fuzz_codegen_full/vault.json
	cp tests/fixtures/minimal.json  fuzz/seeds/fuzz_codegen_full/minimal.json
