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
