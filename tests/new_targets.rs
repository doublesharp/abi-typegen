use abi_typegen_config::{Config, parse_target};
use std::process::Command;

#[test]
fn new_target_names_and_aliases_resolve() {
    for aliases in [
        ["fsharp", "f#", "fs"],
        ["ocaml", "ml", "ocaml"],
        ["q", "kdb", "kdb+"],
    ] {
        let canonical = parse_target(aliases[0]);
        assert!(canonical.is_some(), "missing target {}", aliases[0]);
        for alias in aliases {
            assert_eq!(parse_target(alias), canonical, "alias {alias}");
            let config =
                Config::from_toml_str(&format!("[abi-typegen]\ntarget = {alias:?}\n")).unwrap();
            assert!(!config.target().emits_barrel());
            assert!(!config.target().emits_typescript_abi());
        }
    }
}

#[test]
fn new_targets_generate_together_and_check_without_writing() {
    let root = tempfile::tempdir().unwrap();
    let artifacts = root.path().join("artifacts");
    let output = root.path().join("generated");
    std::fs::create_dir_all(artifacts.join("Token.sol")).unwrap();
    std::fs::write(
        artifacts.join("Token.sol/Token.json"),
        include_str!("fixtures/erc20.json"),
    )
    .unwrap();
    let invoke = |extra: &[&str]| {
        Command::new(env!("CARGO_BIN_EXE_abi-typegen"))
            .current_dir(root.path())
            .args(["generate", "--artifacts"])
            .arg(&artifacts)
            .arg("--out")
            .arg(&output)
            .args(["--target", "fsharp,ocaml,q"])
            .args(extra)
            .output()
            .unwrap()
    };
    let generated = invoke(&["--no-wrappers"]);
    assert!(
        generated.status.success(),
        "{}",
        String::from_utf8_lossy(&generated.stderr)
    );
    for (target, extension) in [("fsharp", "fs"), ("ocaml", "ml"), ("q", "q")] {
        let paths: Vec<_> = std::fs::read_dir(output.join(target))
            .unwrap()
            .map(|p| p.unwrap().path())
            .collect();
        assert!(
            paths
                .iter()
                .any(|p| p.extension().is_some_and(|e| e == extension)),
            "{target}: {paths:?}"
        );
        assert!(
            !paths
                .iter()
                .any(|p| p.extension().is_some_and(|e| e == "c" || e == "ts"))
        );
    }
    let checked = invoke(&["--no-wrappers", "--check"]);
    assert!(
        checked.status.success(),
        "{}",
        String::from_utf8_lossy(&checked.stderr)
    );
    let generated = invoke(&[]);
    assert!(
        generated.status.success(),
        "{}",
        String::from_utf8_lossy(&generated.stderr)
    );
    for target in ["ocaml", "q"] {
        assert!(output.join(target).join("abi_typegen.h").is_file());
        assert!(
            std::fs::read_dir(output.join(target)).unwrap().any(|p| p
                .unwrap()
                .path()
                .extension()
                .is_some_and(|e| e == "c"))
        );
    }
    for (target, filename) in [
        ("fsharp", "Stale.fs"),
        ("ocaml", "Stale.ml"),
        ("q", "Stale.q"),
    ] {
        std::fs::write(output.join(target).join(filename), "stale").unwrap();
    }
    assert!(
        !invoke(&["--check"]).status.success(),
        "stale target files must fail --check"
    );
    let cleaned = invoke(&["--no-wrappers", "--clean"]);
    assert!(
        cleaned.status.success(),
        "{}",
        String::from_utf8_lossy(&cleaned.stderr)
    );
    for (target, filename) in [
        ("fsharp", "Stale.fs"),
        ("ocaml", "Stale.ml"),
        ("q", "Stale.q"),
    ] {
        assert!(
            !output.join(target).join(filename).exists(),
            "stale {filename} remained"
        );
    }
    for target in ["ocaml", "q"] {
        assert!(!std::fs::read_dir(output.join(target)).unwrap().any(|p| {
            p.unwrap()
                .path()
                .extension()
                .is_some_and(|e| e == "c" || e == "h")
        }));
    }
}

#[test]
fn new_targets_reject_normalized_contract_collisions_before_writing() {
    for target in ["fsharp", "ocaml", "q"] {
        let root = tempfile::tempdir().unwrap();
        let artifacts = root.path().join("out");
        let output = root.path().join("generated");
        std::fs::create_dir(&output).unwrap();
        let preserved = output.join("keep.txt");
        std::fs::write(&preserved, "existing output").unwrap();
        for name in ["Foo_Bar", "FooBar"] {
            let directory = artifacts.join(format!("{name}.sol"));
            std::fs::create_dir_all(&directory).unwrap();
            std::fs::write(directory.join(format!("{name}.json")), r#"{"abi":[]}"#).unwrap();
        }
        let result = Command::new(env!("CARGO_BIN_EXE_abi-typegen"))
            .current_dir(root.path())
            .args(["generate", "--artifacts"])
            .arg(&artifacts)
            .arg("--out")
            .arg(&output)
            .args(["--target", target, "--clean"])
            .output()
            .unwrap();
        assert!(
            !result.status.success(),
            "{target} accepted colliding namespaces"
        );
        assert!(String::from_utf8_lossy(&result.stderr).contains("namespace"));
        assert_eq!(
            std::fs::read_to_string(&preserved).unwrap(),
            "existing output"
        );
        assert_eq!(std::fs::read_dir(output).unwrap().count(), 1);
    }
}
