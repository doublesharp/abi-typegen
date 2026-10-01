use super::*;

#[test]
fn fetch_force_rejects_invalid_mutability_before_replacing_an_artifact() {
    let root = tempfile::tempdir().expect("tempdir");
    let input = root.path().join("abi.json");
    let output = root.path().join("out");
    let destination = fetch::artifact_path(&output, "Token");
    std::fs::create_dir_all(destination.parent().expect("parent")).expect("directory");
    std::fs::write(&destination, "old artifact").expect("old artifact");
    for (kind, state) in [
        ("function", "veiw"),
        ("fallback", "view"),
        ("receive", "nonpayable"),
    ] {
        std::fs::write(&input, serde_json::json!([{"type":kind,"name":"read","inputs":[],"outputs":[],"stateMutability":state}]).to_string()).expect("ABI");
        let error = run_fetch(FetchSource::File(&input), "Token", &output, true)
            .expect_err("invalid mutability");
        assert!(format!("{error:#}").contains("mutability"), "{error:#}");
        assert_eq!(
            std::fs::read_to_string(&destination).expect("preserved artifact"),
            "old artifact"
        );
    }
}

#[test]
fn elixir_sdk_macro_names_fail_before_cleaning_but_keep_metadata_mode() {
    for name in ["unquote", "unquoteSplicing", "Unquote"] {
        let root = tempfile::tempdir().expect("tempdir");
        let artifacts = root.path().join("artifacts/Token.sol");
        std::fs::create_dir_all(&artifacts).expect("artifacts");
        let abi = serde_json::json!([{"type":"function","name":name,"inputs":[{"name":"x","type":"bool"}],"outputs":[],"stateMutability":"view"}]);
        std::fs::write(
            artifacts.join("Token.json"),
            serde_json::json!({"abi":abi}).to_string(),
        )
        .expect("ABI");
        let output = root.path().join("output");
        std::fs::create_dir(&output).expect("output");
        std::fs::write(output.join("Old.ex"), "preserve").expect("old binding");
        let mut config = generated_config(
            root.path().join("artifacts"),
            output.clone(),
            Target::Elixir,
        );
        let error = run_generate(&config, true).expect_err("unsupported SDK macro name");
        assert!(error.to_string().contains(name), "{error}");
        assert_eq!(
            std::fs::read_to_string(output.join("Old.ex")).expect("preserved"),
            "preserve"
        );
        assert!(!output.join("token.ex").exists());
        config.wrappers = false;
        run_generate(&config, false).expect("metadata remains supported");
        let metadata = std::fs::read_to_string(output.join("token.ex")).expect("metadata");
        assert!(metadata.contains(name));
        assert!(!metadata.contains("use Ethers.Contract"));
    }
}

#[test]
fn diff_reports_output_directory_errors_even_for_an_empty_selection() {
    let root = tempfile::tempdir().expect("tempdir");
    let artifacts = root.path().join("artifacts");
    std::fs::create_dir(&artifacts).expect("empty artifacts");
    let output = root.path().join("output");
    std::fs::write(&output, "ordinary file").expect("output file");
    let config = generated_config(artifacts, output.clone(), Target::Viem);
    let selected = selected_artifacts(&config).expect("empty selection");
    assert!(selected.is_empty());
    let error = collect_diff_entries(&config, &selected).expect_err("not a directory");
    assert!(error.to_string().contains("cannot read"));
    assert!(run_diff(&config).is_err());
    assert_eq!(
        std::fs::read_to_string(output).expect("preserved"),
        "ordinary file"
    );
}

#[test]
fn json_and_diff_accept_current_output_without_rewriting_it() {
    let root = tempfile::tempdir().expect("tempdir");
    let artifacts = root.path().join("artifacts");
    write_target_matrix_artifact(&artifacts, "Token");
    let output = root.path().join("output");
    let config = generated_config(artifacts, output.clone(), Target::Viem);
    run_generate(&config, false).expect("generate");
    let binding = output.join("Token.abi.ts");
    let original = std::fs::read(&binding).expect("binding");
    let modified = std::fs::metadata(&binding)
        .expect("metadata")
        .modified()
        .expect("mtime");
    std::fs::create_dir(output.join("scratch.abi.ts")).expect("unrelated directory");
    run_diff(&config).expect("current output");
    for pretty in [true, false] {
        run_json(&config, pretty).expect("JSON summary");
    }
    let summaries = collect_json_summaries(&config).expect("summaries");
    assert_eq!(summaries.len(), 1);
    assert_eq!(summaries[0]["name"], "Token");
    assert_eq!(summaries[0]["functions"], 1);
    assert_eq!(std::fs::read(&binding).expect("unchanged"), original);
    assert_eq!(
        std::fs::metadata(&binding)
            .expect("metadata")
            .modified()
            .expect("mtime"),
        modified
    );
    assert!(output.join("scratch.abi.ts").is_dir());
}

#[test]
fn fetch_rejects_names_that_cannot_be_safe_contract_paths_before_writing() {
    for name in [
        "",
        "../escape",
        "nested/Token",
        "Token\\Nested",
        "9Token",
        "bad-name",
        " Token",
        "Token.sol",
    ] {
        let root = tempfile::tempdir().expect("tempdir");
        let input = root.path().join("input.json");
        std::fs::write(&input, "[]").expect("ABI");
        let output = root.path().join("out");
        std::fs::create_dir(&output).expect("out");
        let sentinel = output.join("keep.txt");
        std::fs::write(&sentinel, "preserve me").expect("sentinel");
        let result = run_fetch(FetchSource::File(&input), name, &output, true);
        assert!(result.is_err(), "accepted invalid name {name:?}");
        assert!(
            result
                .expect_err("invalid name")
                .to_string()
                .contains("contract name")
        );
        assert_eq!(
            std::fs::read_to_string(&sentinel).expect("preserved"),
            "preserve me"
        );
        assert_eq!(std::fs::read_dir(&output).expect("entries").count(), 1);
    }
}

#[test]
fn fetch_force_replaces_only_the_selected_valid_artifact() {
    for name in ["Token", "_Token2", "$Token"] {
        let root = tempfile::tempdir().expect("tempdir");
        let input = root.path().join("abi.json");
        std::fs::write(&input, "[]").expect("input");
        let output = root.path().join("out");
        let destination = fetch::artifact_path(&output, name);
        std::fs::create_dir_all(destination.parent().expect("parent")).expect("directory");
        std::fs::write(&destination, "old").expect("old artifact");
        let keep = output.join("unrelated.json");
        std::fs::write(&keep, "keep").expect("other artifact");
        run_fetch(FetchSource::File(&input), name, &output, true).expect("forced fetch");
        let value: serde_json::Value =
            serde_json::from_slice(&std::fs::read(destination).expect("artifact")).expect("JSON");
        assert_eq!(value, serde_json::json!({"abi":[]}));
        assert_eq!(
            std::fs::read_to_string(keep).expect("other artifact"),
            "keep"
        );
    }
}

#[test]
fn duplicate_contract_names_fail_before_cleaning_or_writing() {
    let root = tempfile::tempdir().expect("tempdir");
    let artifacts = root.path().join("artifacts");
    for folder in ["a", "b"] {
        write_target_matrix_artifact(&artifacts.join(folder), "Token");
    }
    let output = root.path().join("output");
    std::fs::create_dir(&output).expect("output");
    std::fs::write(output.join("Old.abi.ts"), "keep").expect("old binding");
    let config = generated_config(artifacts, output.clone(), Target::Viem);
    let error = run_generate(&config, true).expect_err("duplicate name");
    assert!(
        error
            .to_string()
            .contains("duplicate contract name 'Token'")
    );
    assert_eq!(
        std::fs::read_to_string(output.join("Old.abi.ts")).expect("preserved"),
        "keep"
    );
    assert_eq!(std::fs::read_dir(output).expect("entries").count(), 1);
}

#[test]
fn check_and_diff_report_unreadable_expected_outputs_as_io_errors() {
    let root = tempfile::tempdir().expect("tempdir");
    let artifacts = root.path().join("artifacts");
    write_target_matrix_artifact(&artifacts, "Token");
    let output = root.path().join("output");
    std::fs::create_dir_all(output.join("Token.abi.ts")).expect("directory at output filename");
    let config = generated_config(artifacts, output.clone(), Target::Viem);
    let selected = selected_artifacts(&config).expect("selection");
    assert!(
        collect_diff_entries(&config, &selected)
            .expect_err("read error")
            .to_string()
            .contains("cannot read")
    );
    assert!(run_check(&config).is_err());
    assert!(
        run_generate(&config, false)
            .expect_err("write error")
            .to_string()
            .contains("cannot read")
    );
    assert!(output.join("Token.abi.ts").is_dir());
}

#[test]
fn clean_keeps_unrelated_directories_and_only_removes_stale_generated_files() {
    let root = tempfile::tempdir().expect("tempdir");
    let output = root.path();
    std::fs::create_dir(output.join("Old.abi.ts")).expect("directory");
    std::fs::write(output.join("README.md"), "keep").expect("README");
    std::fs::write(output.join("Current.abi.ts"), "current").expect("current");
    std::fs::write(output.join("Stale.abi.ts"), "stale").expect("stale");
    clean_stale_files(output, &HashSet::from(["Current.abi.ts".into()])).expect("clean");
    assert!(output.join("Old.abi.ts").is_dir());
    assert_eq!(
        std::fs::read_to_string(output.join("README.md")).expect("README"),
        "keep"
    );
    assert_eq!(
        std::fs::read_to_string(output.join("Current.abi.ts")).expect("current"),
        "current"
    );
    assert!(!output.join("Stale.abi.ts").exists());
}

#[test]
fn stale_target_selection_keeps_active_and_unknown_directories() {
    let root = tempfile::tempdir().expect("tempdir");
    for name in ["viem", "q", "my-project"] {
        std::fs::create_dir(root.path().join(name)).expect("directory");
    }
    std::fs::write(root.path().join("ocaml"), "ordinary file").expect("file");
    let active = HashSet::from(["viem".into()]);
    assert_eq!(
        stale_target_dirs(root.path(), &active).expect("stale"),
        vec!["q"]
    );
    clean_stale_target_dirs(root.path(), &active).expect("clean");
    assert!(root.path().join("viem").is_dir());
    assert!(root.path().join("my-project").is_dir());
    assert_eq!(
        std::fs::read_to_string(root.path().join("ocaml")).expect("file"),
        "ordinary file"
    );
    assert!(!root.path().join("q").exists());
}

#[test]
fn hardhat_config_uses_its_artifact_default_and_honors_cli_overrides() {
    let root = tempfile::tempdir().expect("tempdir");
    let path = root.path().join("hardhat.abi-typegen.toml");
    assert_eq!(
        resolve_config_path(&None, true),
        PathBuf::from("hardhat.config.ts")
    );
    assert_eq!(resolve_config_path(&Some(path.clone()), true), path);
    std::fs::write(&path, "export default {};").expect("JavaScript config");
    let mut config = load_config(&path, true).expect("default");
    assert_eq!(config.artifacts_dir, PathBuf::from("artifacts/contracts"));
    apply_overrides(
        &mut config,
        Some(PathBuf::from("custom")),
        None,
        None,
        false,
        None,
    )
    .expect("overrides");
    assert_eq!(config.artifacts_dir, PathBuf::from("custom"));
}

#[test]
fn init_appends_once_without_changing_existing_configuration() {
    let root = tempfile::tempdir().expect("tempdir");
    let path = root.path().join("foundry.toml");
    let original = "[profile.default]\nout = 'custom-out'\n";
    std::fs::write(&path, original).expect("config");
    run_init(&path).expect("append");
    let first = std::fs::read_to_string(&path).expect("new config");
    assert!(first.starts_with(original));
    assert_eq!(first.matches("[abi-typegen]").count(), 1);
    run_init(&path).expect("second init");
    assert_eq!(std::fs::read_to_string(&path).expect("unchanged"), first);
}

#[test]
fn duplicate_cli_targets_are_rendered_once_and_empty_excludes_are_ignored() {
    let mut config = Config::from_toml_str("").expect("config");
    config.targets = vec![Target::Viem, Target::Viem, Target::Yaml];
    let outputs = target_configs(&config);
    assert_eq!(outputs.len(), 2);
    assert_eq!(outputs[0].0, "viem/");
    assert_eq!(outputs[1].0, "yaml/");
    apply_exclude(&mut config, &Some(", ,*Mock,, I* ,".into()));
    assert_eq!(config.exclude, ["*Mock", "I*"]);
}

#[test]
fn contract_names_similar_to_runtime_headers_keep_both_headers_distinct() {
    let root = tempfile::tempdir().expect("tempdir");
    let artifacts = root.path().join("artifacts");
    write_target_matrix_artifact(&artifacts, "abi_typegen");
    let output = root.path().join("output");
    std::fs::create_dir(&output).expect("output");
    std::fs::write(output.join("README.txt"), "preserve").expect("old header");
    let config = generated_config(artifacts, output.clone(), Target::C);
    run_generate(&config, true).expect("distinct contract and runtime headers");
    assert!(output.join("atg_abi_typegen.h").is_file());
    assert!(output.join("abi_typegen.h").is_file());
    assert_eq!(
        std::fs::read_to_string(output.join("README.txt")).expect("old header"),
        "preserve"
    );
}

#[test]
fn diff_and_json_reject_missing_artifacts_before_creating_output() {
    let root = tempfile::tempdir().expect("tempdir");
    let output = root.path().join("output");
    let config = generated_config(root.path().join("missing"), output.clone(), Target::Viem);
    assert!(
        run_diff(&config)
            .expect_err("missing artifacts")
            .to_string()
            .contains("does not exist")
    );
    assert!(
        run_json(&config, false)
            .expect_err("missing artifacts")
            .to_string()
            .contains("does not exist")
    );
    assert!(!output.exists());
}

#[test]
fn watch_loop_drains_a_burst_and_preserves_unrelated_files() {
    let root = tempfile::tempdir().expect("tempdir");
    let artifacts = root.path().join("artifacts");
    write_target_matrix_artifact(&artifacts, "Token");
    let output = root.path().join("output");
    std::fs::create_dir(&output).expect("output");
    std::fs::write(output.join("README.txt"), "preserve").expect("README");
    let config = generated_config(artifacts, output.clone(), Target::Viem);
    let (tx, rx) = std::sync::mpsc::channel();
    for _ in 0..4 {
        tx.send(Ok(notify::Event::new(notify::EventKind::Any)))
            .expect("event");
    }
    drop(tx);
    watch_loop(&config, &rx);
    run_check(&config).expect("generated output");
    assert_eq!(
        std::fs::read_to_string(output.join("README.txt")).expect("README"),
        "preserve"
    );
}

#[test]
fn discovery_ignores_directories_debug_files_and_empty_contract_names() {
    let root = tempfile::tempdir().expect("tempdir");
    write_target_matrix_artifact(root.path(), "Token");
    let folder = root.path().join("Token.sol");
    for name in ["Token.dbg.json", ".json", "notes.txt"] {
        std::fs::write(folder.join(name), "invalid ABI").expect("ignored file");
    }
    std::fs::create_dir(folder.join("directory.json")).expect("ignored directory");
    let discovered = discover_artifacts(root.path(), &[]).expect("discovery");
    assert_eq!(discovered.len(), 1);
    assert_eq!(discovered[0].0, "Token");
}

#[cfg(all(unix, not(target_os = "macos")))]
#[test]
fn non_utf8_paths_are_ignored_and_preserved_during_discovery_clean_and_diff() {
    use std::os::unix::ffi::OsStringExt;
    let root = tempfile::tempdir().expect("tempdir");
    let artifacts = root.path().join("artifacts");
    write_target_matrix_artifact(&artifacts, "Token");
    let invalid = std::ffi::OsString::from_vec(vec![0xff]);
    let invalid_sol = std::ffi::OsString::from_vec(vec![0xff, b'.', b's', b'o', b'l']);
    std::fs::create_dir(artifacts.join(invalid_sol)).expect("invalid directory");
    std::fs::write(artifacts.join("Token.sol").join(&invalid), "preserve")
        .expect("invalid filename");
    let output = root.path().join("output");
    let config = generated_config(artifacts, output.clone(), Target::Viem);
    run_generate(&config, false).expect("generate");
    std::fs::write(output.join(&invalid), "preserve").expect("non UTF8 output");
    std::fs::create_dir(output.join("notes")).expect("ordinary directory");
    let invalid_target = output.join(std::ffi::OsString::from_vec(vec![0xfe]));
    std::fs::create_dir(&invalid_target).expect("non UTF8 target");
    assert!(
        stale_target_dirs(&output, &HashSet::new())
            .expect("target dirs")
            .is_empty()
    );
    let artifacts = selected_artifacts(&config).expect("selection");
    assert!(
        collect_diff_entries(&config, &artifacts)
            .expect("diff")
            .is_empty()
    );
    run_generate(&config, true).expect("clean generate");
    run_check(&config).expect("check");
    assert_eq!(
        std::fs::read_to_string(output.join(invalid)).expect("preserved"),
        "preserve"
    );
    assert!(invalid_target.is_dir());
}
