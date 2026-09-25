# Makefile — abi-typegen development targets

# Written once by .cargo/setup-scratch.py; absent in ordinary clones and CI.
-include .cargo/scratch.local.mk

.PHONY: build test check fmt lint e2e e2e-native e2e-native-artifacts e2e-go e2e-rust e2e-swift e2e-kotlin e2e-c e2e-csharp e2e-java e2e-dart e2e-python e2e-php e2e-cobol e2e-ruby e2e-shell bench coverage coverage-open coverage-summary \
        fuzz fuzz-parse-artifact fuzz-config-toml fuzz-sol-type fuzz-codegen-full fuzz-barrel \
        fuzz-corpus fuzz-init-corpus scratch-setup scratch-disable test-storage

DART ?= dart

DOUBLCOV ?= npx --yes @0xdoublesharp/doublcov@0

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

e2e: e2e-foundry e2e-hardhat e2e-hardhat3 ## Run all e2e tests

e2e-foundry: build ## E2E: Foundry → abi-typegen → tsc
	cd e2e/foundry-sample && forge build
	cd e2e/foundry-sample && ../../target/debug/abi-typegen generate --artifacts ./out --out ./src/generated --target viem
	cd e2e/foundry-sample && ../../target/debug/abi-typegen generate --artifacts ./out --out ./src/generated --target ethers
	cd e2e/foundry-sample && pnpm exec tsc --noEmit
	cd e2e/foundry-sample && ../../target/debug/abi-typegen generate --artifacts ./out --out ./src/generated-zod --target zod
	cd e2e/foundry-sample && pnpm exec tsc --noEmit -p tsconfig.zod.json
	python3 e2e/native/anvil.py --cwd e2e/foundry-sample node --test --test-force-exit test-anvil.mjs
	cd e2e/foundry-sample && ../../target/debug/abi-typegen generate --artifacts ./out --out ./src/generated-solidity --target solidity
	cd e2e/foundry-sample && forge build --contracts ./solidity-validation --out ./out-solidity-validation
	@echo "e2e-foundry: pass"

e2e-hardhat: build ## E2E: Hardhat → abi-typegen --hardhat → tsc
	cd e2e/hardhat-sample && pnpm exec hardhat compile --quiet
	cd e2e/hardhat-sample && ../../target/debug/abi-typegen generate --hardhat --artifacts ./artifacts/contracts --out ./abi-typegen-out --target viem
	@echo "e2e-hardhat: pass"

e2e-hardhat3: build ## E2E: Hardhat 3 plugin → abi-typegen --hardhat → tsc
	cd e2e/hardhat3-sample && pnpm exec hardhat compile
	cd e2e/hardhat3-sample && test -f abi-typegen-out/Token.viem.ts
	cd e2e/hardhat3-sample && pnpm exec tsc --noEmit
	@echo "e2e-hardhat3: pass"

# ── Native-language e2e ──────────────────────────────────────────────────────
# Generates native bindings from the Foundry sample and tests real consumers.
# Requires Foundry (forge, cast, anvil), Python 3.11+, Cargo, and the selected
# language toolchain. The same targets run in GitHub Actions.

NATIVE_TYPEGEN := ../../../target/debug/abi-typegen generate --artifacts ../../foundry-sample/out

e2e-native: e2e-go e2e-rust e2e-swift e2e-kotlin e2e-c e2e-csharp e2e-java e2e-dart e2e-python e2e-php e2e-cobol e2e-ruby e2e-shell ## E2E: all native-language targets

e2e-native-artifacts: build
	cd e2e/foundry-sample && forge build

e2e-go: e2e-native-artifacts ## E2E: Go bindings → gofmt, go vet, go test
	cd e2e/native/go && rm -rf contracts && $(NATIVE_TYPEGEN) --out ./contracts --target go
	cd e2e/native/go && rm -rf usage/tupleindexed && ../../../target/debug/abi-typegen generate --artifacts ./artifacts --out ./usage/tupleindexed --target go --package tupleindexed
	cd e2e/native/go && unformatted="$$(gofmt -l .)" && test -z "$$unformatted" || { echo "gofmt: $$unformatted"; exit 1; }
	cd e2e/native/go && go vet ./... && go test ./...
	python3 e2e/native/anvil.py --cwd e2e/native/go go test ./usage -run TestGeneratedBindingsAnvil -count=1
	@echo "e2e-go: pass"

e2e-rust: e2e-native-artifacts ## E2E: Rust bindings → rustfmt, clippy, rustdoc, tests
	cd e2e/native/rust && rm -rf src/contracts && $(NATIVE_TYPEGEN) --out ./src/contracts --target rust
	cd e2e/native/rust && cargo fmt --check
	cd e2e/native/rust && cargo clippy --all-targets -- -D warnings
	cd e2e/native/rust && RUSTDOCFLAGS="-D warnings" cargo doc --no-deps
	cd e2e/native/rust && cargo test
	python3 e2e/native/anvil.py --cwd e2e/native/rust cargo test anvil -- --nocapture
	@echo "e2e-rust: pass"

e2e-swift: e2e-native-artifacts ## E2E: Swift bindings → Swift 6 build across modules, tests
	cd e2e/native/swift && rm -rf Sources/Generated && $(NATIVE_TYPEGEN) --out ./Sources/Generated --target swift
	cd e2e/native/swift && swift build --build-tests
	cd e2e/native/swift && swift test
	python3 e2e/native/anvil.py --cwd e2e/native/swift swift test
	@echo "e2e-swift: pass"

e2e-kotlin: e2e-native-artifacts ## E2E: Kotlin bindings → Gradle build with web3j, Java interop tests
	cd e2e/native/kotlin && rm -rf build/generated-contracts && $(NATIVE_TYPEGEN) --out ./build/generated-contracts --target kotlin --package com.example.contracts
	cd e2e/native/kotlin && gradle test --console=plain
	python3 e2e/native/anvil.py --cwd e2e/native/kotlin gradle test --rerun-tasks --console=plain
	@echo "e2e-kotlin: pass"

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
# Requires cargo-llvm-cov and Node/npm:
#   cargo install cargo-llvm-cov --locked
#
# Run:   make coverage         → Doublcov report in coverage/report/index.html
# Run:   make coverage-open    → generate + open in browser
# Run:   make coverage-summary → print summary to stdout

coverage: ## Doublcov report in coverage/report/index.html
	$(DOUBLCOV) cargo-llvm-cov --mode standalone --no-open -- --workspace

coverage-open: ## Generate HTML coverage report and open in browser
	$(DOUBLCOV) cargo-llvm-cov --mode standalone --open -- --workspace

coverage-summary: ## Print line/branch coverage summary to stdout
	cargo llvm-cov --workspace --summary-only

fuzz-corpus: ## Update seeds from test fixtures
	cp tests/fixtures/erc20.json    fuzz/seeds/fuzz_parse_artifact/erc20.json
	cp tests/fixtures/vault.json    fuzz/seeds/fuzz_parse_artifact/vault.json
	cp tests/fixtures/minimal.json  fuzz/seeds/fuzz_parse_artifact/minimal.json
	cp tests/fixtures/erc20.json    fuzz/seeds/fuzz_codegen_full/erc20.json
	cp tests/fixtures/vault.json    fuzz/seeds/fuzz_codegen_full/vault.json
	cp tests/fixtures/minimal.json  fuzz/seeds/fuzz_codegen_full/minimal.json

scratch-setup: ## Reapply the saved disposable-storage configuration
	python3 .cargo/setup-scratch.py

scratch-disable: ## Return to default paths, preserving external data
	python3 .cargo/setup-scratch.py --disable

test-storage: ## Test optional storage tooling (Python 3.11+)
	PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s tests -p test_scratch.py

# C and C++ share one runtime; both consumer compilers are exercised.
e2e-c: e2e-native-artifacts
	cargo build -p abi-typegen-runtime
	python3 e2e/native/anvil.py sh e2e/native/c/run.sh

e2e-java: e2e-native-artifacts
	cd e2e/native/java && rm -rf build/generated-contracts && $(NATIVE_TYPEGEN) --out ./build/generated-contracts --target java --package com.example.contracts
	cd e2e/native/java && gradle test --console=plain
	python3 e2e/native/anvil.py --cwd e2e/native/java gradle test --rerun-tasks --console=plain

e2e-csharp: e2e-native-artifacts
	cd e2e/native/csharp && rm -rf Generated && $(NATIVE_TYPEGEN) --out ./Generated --target csharp
	cd e2e/native/csharp && dotnet run --project Consumer.csproj
	python3 e2e/native/anvil.py --cwd e2e/native/csharp dotnet run --project Consumer.csproj

e2e-dart: e2e-native-artifacts
	cd e2e/native/dart && rm -rf lib/generated && $(NATIVE_TYPEGEN) --out ./lib/generated --target dart
	cd e2e/native/dart && rm -rf lib/regressions && ../../../target/debug/abi-typegen generate --artifacts ./artifacts --out ./lib/regressions --target dart
	cd e2e/native/dart && $(DART) pub get && $(DART) analyze && $(DART) test
	cd e2e/native/dart && $(DART) run bin/validate_generated.dart
	python3 e2e/native/anvil.py --cwd e2e/native/dart $(DART) test

e2e-python: e2e-native-artifacts
	python3 -m venv e2e/native/python/.venv
	e2e/native/python/.venv/bin/python -m pip install -r e2e/native/python/requirements.txt
	cd e2e/native/python && rm -rf Generated && $(NATIVE_TYPEGEN) --out ./Generated --target python
	cd e2e/native/python && .venv/bin/python -m compileall -q Generated && .venv/bin/python test_consumer.py
	python3 e2e/native/anvil.py --cwd e2e/native/python .venv/bin/python test_consumer.py

e2e-php: e2e-native-artifacts
	cd e2e/native/php && composer install --no-interaction --prefer-dist
	cd e2e/native/php && rm -rf Generated && $(NATIVE_TYPEGEN) --out ./Generated --target php --package NativeBindings
	find e2e/native/php/Generated -name '*.php' -print0 | xargs -0 -n1 php -l
	cd e2e/native/php && php test_consumer.php
	cd e2e/native/php && rm -rf Metadata && $(NATIVE_TYPEGEN) --out ./Metadata --target php --package NativeBindings --no-wrappers
	find e2e/native/php/Metadata -name '*.php' -print0 | xargs -0 -n1 php -l
	cd e2e/native/php && php test_metadata.php Metadata
	python3 e2e/native/anvil.py --cwd e2e/native/php php test_consumer.php

e2e-cobol: e2e-native-artifacts
	cargo build -p abi-typegen-runtime
	sh e2e/native/cobol/run.sh
	python3 e2e/native/anvil.py sh e2e/native/cobol/run.sh anvil

e2e-shell: e2e-native-artifacts
	sh e2e/native/shell/run.sh
	python3 e2e/native/anvil.py sh e2e/native/shell/run.sh anvil

e2e-ruby: e2e-native-artifacts
	cd e2e/native/ruby && bundle config set --local path vendor/bundle
	cd e2e/native/ruby && bundle config set --local build.rbsecp256k1 --with-system-library
	cd e2e/native/ruby && bundle install
	cd e2e/native/ruby && rm -rf build/generated-contracts && $(NATIVE_TYPEGEN) --out ./build/generated-contracts --target ruby
	cd e2e/native/ruby && bundle exec ruby verify_generated.rb build/generated-contracts
	cd e2e/native/ruby && rm -rf build/metadata-contracts && $(NATIVE_TYPEGEN) --out ./build/metadata-contracts --target ruby --no-wrappers
	cd e2e/native/ruby && ruby verify_generated.rb build/metadata-contracts --metadata
	cd e2e/native/ruby && bundle exec rspec generated_spec.rb anvil_spec.rb
	python3 e2e/native/anvil.py --cwd e2e/native/ruby bundle exec rspec generated_spec.rb anvil_spec.rb
