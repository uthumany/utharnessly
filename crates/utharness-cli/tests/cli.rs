use std::process::Command;
use tempfile::tempdir;

fn run(bin: &str, cwd: &std::path::Path, home: &std::path::Path, args: &[&str]) -> String {
    let output = Command::new(bin)
        .current_dir(cwd)
        .env("HOME", home)
        .env("UTHARNESS_HOME", home.join(".utharness"))
        .args(args)
        .output()
        .expect("run utharness");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).expect("utf8 output")
}

#[test]
fn skill_commands_cover_registry_lifecycle_and_local_import() {
    let workspace = tempdir().unwrap();
    let home = tempdir().unwrap();
    let bin = env!("CARGO_BIN_EXE_utharness");
    let git_init = Command::new("git")
        .args(["init", "-q"])
        .current_dir(workspace.path())
        .status()
        .unwrap();
    assert!(git_init.success());

    let list = run(bin, workspace.path(), home.path(), &["skills"]);
    assert!(list.contains("UTHARNESS SKILL REGISTRY"));
    assert!(list.contains("builtin.git-status"));
    let search = run(
        bin,
        workspace.path(),
        home.path(),
        &["skills", "search", "git"],
    );
    assert!(search.contains("builtin.git-status"));
    let categories = run(
        bin,
        workspace.path(),
        home.path(),
        &["skills", "categories"],
    );
    assert!(categories.lines().any(|line| line == "coding"));
    let install = run(
        bin,
        workspace.path(),
        home.path(),
        &["skills", "install", "builtin.git-status"],
    );
    assert!(install.contains("installed builtin.git-status"));
    let tested = run(
        bin,
        workspace.path(),
        home.path(),
        &["skills", "test", "builtin.git-status"],
    );
    assert!(tested.contains("health=healthy"));
    let result = run(
        bin,
        workspace.path(),
        home.path(),
        &["skills", "run", "builtin.git-status"],
    );
    assert!(result.contains("SKILL RESULT"));
    let removed = run(
        bin,
        workspace.path(),
        home.path(),
        &["skills", "remove", "builtin.git-status"],
    );
    assert!(removed.contains("removed builtin.git-status"));
    let doctor = run(bin, workspace.path(), home.path(), &["skills", "doctor"]);
    assert!(doctor.contains("registry healthy"));

    let manifest = workspace.path().join("utharness.skill.json");
    std::fs::write(
        &manifest,
        r#"{"schemaVersion":1,"id":"local.example","name":"Local Example","description":"A local test skill","category":"utilities","source":{"provider":"local","url":"file:///tmp/local","repository":null,"commit":null},"version":"1.0.0","runtime":[],"entrypoint":null,"commands":[],"dependencies":[],"tools":[],"permissions":["context.read"],"environment":[],"inputs":{},"outputs":{},"tags":["test"],"install":{},"compatibility":{},"license":"MIT","homepage":null,"documentation":null,"checksum":null,"updateSource":null}"#,
    )
    .unwrap();
    let imported = run(
        bin,
        workspace.path(),
        home.path(),
        &["skills", "import", manifest.to_str().unwrap()],
    );
    assert!(imported.contains("imported local.example v1.0.0"));
}

#[test]
fn cli_persists_workspace_session_memory_and_doctor() {
    let workspace = tempdir().unwrap();
    let home = tempdir().unwrap();
    let bin = env!("CARGO_BIN_EXE_utharness");

    let startup = run(bin, workspace.path(), home.path(), &[]);
    assert!(
        startup.contains("U T H Y"),
        "startup stdout was: {startup:?}"
    );
    assert!(
        startup.contains("AGENT TERMINAL"),
        "startup stdout was: {startup:?}"
    );

    let init = run(bin, workspace.path(), home.path(), &["init"]);
    assert!(init.contains("UTHARNESS initialized"));

    let created = run(
        bin,
        workspace.path(),
        home.path(),
        &["sessions", "new", "integration"],
    );
    assert!(created.contains("created session"));

    let memory = run(
        bin,
        workspace.path(),
        home.path(),
        &["memory", "add", "SQLite persistence is enabled"],
    );
    assert!(memory.contains("stored memory"));

    let search = run(
        bin,
        workspace.path(),
        home.path(),
        &["memory", "search", "persistence"],
    );
    assert!(search.contains("SQLite persistence is enabled"));

    let doctor = run(bin, workspace.path(), home.path(), &["doctor"]);
    assert!(doctor.contains("✓ diagnostics   clean"));
}
