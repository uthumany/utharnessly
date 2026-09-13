use std::{
    io::{Read, Write},
    net::TcpListener,
    process::{Command, Stdio},
};
use tempfile::tempdir;

fn run(bin: &str, cwd: &std::path::Path, home: &std::path::Path, args: &[&str]) -> String {
    run_with_env(bin, cwd, home, args, &[])
}

fn run_with_env(
    bin: &str,
    cwd: &std::path::Path,
    home: &std::path::Path,
    args: &[&str],
    extra_env: &[(&str, &str)],
) -> String {
    let mut command = Command::new(bin);
    for key in [
        "UTHARNESS_PROVIDER",
        "UTHARNESS_PROVIDER_URL",
        "UTHARNESS_MODEL",
        "UTHARNESS_API_KEY",
        "OPENROUTER_API_KEY",
        "OPENAI_API_KEY",
        "GROQ_API_KEY",
        "TOGETHER_API_KEY",
        "DEEPSEEK_API_KEY",
        "FIREWORKS_API_KEY",
        "NVIDIA_API_KEY",
    ] {
        command.env_remove(key);
    }
    command
        .current_dir(cwd)
        .env("HOME", home)
        .env("UTHARNESS_HOME", home.join(".utharness"))
        .args(args);
    for (key, value) in extra_env {
        command.env(key, value);
    }
    let output = command.output().expect("run utharness");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).expect("utf8 output")
}

#[test]
fn cli_version_matches_cargo_package_version() {
    let output = Command::new(env!("CARGO_BIN_EXE_utharness"))
        .arg("--version")
        .output()
        .expect("run utharness --version");
    assert!(output.status.success());
    let reported = String::from_utf8(output.stdout).expect("utf8 version output");
    assert_eq!(
        reported.trim(),
        format!("utharness {}", env!("CARGO_PKG_VERSION"))
    );
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
fn termux_commands_create_no_root_paths_and_report_optional_features() {
    let workspace = tempdir().unwrap();
    let home = tempdir().unwrap();
    let prefix = tempdir().unwrap();
    std::fs::create_dir_all(prefix.path().join("bin")).unwrap();
    let bin = env!("CARGO_BIN_EXE_utharness");
    let prefix_text = prefix.path().to_str().unwrap();
    let env = [("TERMUX_VERSION", "0.118.0"), ("PREFIX", prefix_text)];

    let setup = run_with_env(
        bin,
        workspace.path(),
        home.path(),
        &["termux", "setup"],
        &env,
    );
    assert!(setup.contains("Termux directories initialized"));
    assert!(home.path().join(".config/utharness").is_dir());
    assert!(home.path().join(".local/share/utharness/skills").is_dir());
    assert!(home.path().join(".cache/utharness").is_dir());

    let info = run_with_env(
        bin,
        workspace.path(),
        home.path(),
        &["termux", "info"],
        &env,
    );
    assert!(info.contains("\"platform\": \"termux\""));
    assert!(info.contains("\"prefix\""));
    let api = run_with_env(bin, workspace.path(), home.path(), &["termux", "api"], &env);
    assert!(api.contains("optional"));
    let permissions = run_with_env(
        bin,
        workspace.path(),
        home.path(),
        &["termux", "permissions"],
        &env,
    );
    assert!(permissions.contains("Storage sandbox"));
    let keys = run_with_env(
        bin,
        workspace.path(),
        home.path(),
        &["termux", "keys", "install"],
        &env,
    );
    assert!(keys.contains("extra keys installed"));
    assert!(home.path().join(".termux/termux.properties").is_file());
    let setup_command = run_with_env(bin, workspace.path(), home.path(), &["setup"], &env);
    assert!(setup_command.contains("Android / Termux"));
    let doctor_command = run_with_env(bin, workspace.path(), home.path(), &["doctor"], &env);
    assert!(doctor_command.contains("UTHARNESS TERMUX DOCTOR"));
    let config = run_with_env(bin, workspace.path(), home.path(), &["config"], &env);
    assert!(config.contains("permission_mode"));
    let sessions = run_with_env(bin, workspace.path(), home.path(), &["sessions"], &env);
    assert!(sessions.contains("No sessions") || sessions.contains("Terminal session"));
    let update = run_with_env(bin, workspace.path(), home.path(), &["update"], &env);
    assert!(update.contains("pkg update"));
    let doctor = run_with_env(
        bin,
        workspace.path(),
        home.path(),
        &["termux", "doctor"],
        &env,
    );
    assert!(doctor.contains("UTHARNESS TERMUX DOCTOR"));
    let models = run_with_env(bin, workspace.path(), home.path(), &["models"], &env);
    assert!(models.contains("MODELS"));
    let mcp = run_with_env(bin, workspace.path(), home.path(), &["mcp"], &env);
    assert!(mcp.contains("MCP"));
    let memory = run_with_env(bin, workspace.path(), home.path(), &["memory"], &env);
    assert!(memory.contains("MEMORY"));
}

#[test]
fn memory_supports_kinds_expiry_and_prune() {
    let workspace = tempdir().unwrap();
    let home = tempdir().unwrap();
    let bin = env!("CARGO_BIN_EXE_utharness");

    let added = run(
        bin,
        workspace.path(),
        home.path(),
        &[
            "memory",
            "add",
            "Prune test note",
            "--kind",
            "fact",
            "--expires",
            "7d",
        ],
    );
    assert!(added.contains("stored memory"));
    let added_again = run(
        bin,
        workspace.path(),
        home.path(),
        &["memory", "add", "Prune test note", "--kind", "fact"],
    );
    assert!(added_again.contains("stored memory"));
    let pruned = run(bin, workspace.path(), home.path(), &["memory", "prune"]);
    assert!(pruned.contains("pruned 0 expired and 1 duplicate"));
    let search = run(
        bin,
        workspace.path(),
        home.path(),
        &["memory", "search", "prune"],
    );
    assert!(search.contains("Prune test note"));
}

#[test]
fn feature_icons_render_glyphs_and_ascii_fallbacks() {
    let workspace = tempdir().unwrap();
    let home = tempdir().unwrap();
    let bin = env!("CARGO_BIN_EXE_utharness");

    let unicode = run_with_env(bin, workspace.path(), home.path(), &["tools"], &[]);
    for glyph in ["\u{13080}", "\u{132F9}", "\u{133DC}"] {
        assert!(unicode.contains(glyph), "missing {glyph} in: {unicode}");
    }
    let ascii = run_with_env(
        bin,
        workspace.path(),
        home.path(),
        &["tools"],
        &[("UTHARNESS_ASCII", "1")],
    );
    for tag in ["[eye]", "[ankh]", "[scroll]"] {
        assert!(ascii.contains(tag), "missing {tag} in: {ascii}");
    }

    let doctor = run_with_env(
        bin,
        workspace.path(),
        home.path(),
        &["desktop", "doctor"],
        &[("UTHARNESS_TOOLS", "desktop")],
    );
    assert!(
        doctor.contains("\u{13080}"),
        "doctor header lost the eye: {doctor}"
    );
}

#[test]
fn desktop_commands_require_approval_and_refuse_danger() {
    let workspace = tempdir().unwrap();
    let home = tempdir().unwrap();
    let bin = env!("CARGO_BIN_EXE_utharness");

    // No --allow: hard refusal before any backend runs.
    let denied = Command::new(bin)
        .current_dir(workspace.path())
        .env("HOME", home.path())
        .env("UTHARNESS_HOME", home.path().join(".utharness"))
        .env("UTHARNESS_TOOLS", "desktop")
        .args(["desktop", "click", "10", "20"])
        .output()
        .expect("run utharness");
    assert!(!denied.status.success());
    assert!(
        String::from_utf8_lossy(&denied.stderr).contains("--allow"),
        "stderr was: {:?}",
        String::from_utf8_lossy(&denied.stderr)
    );

    // Capability missing: refusal names the capability.
    let capped = Command::new(bin)
        .current_dir(workspace.path())
        .env("HOME", home.path())
        .env("UTHARNESS_HOME", home.path().join(".utharness"))
        .env("UTHARNESS_TOOLS", "terminal")
        .args(["desktop", "click", "10", "20", "--allow"])
        .output()
        .expect("run utharness");
    assert!(!capped.status.success());
    assert!(
        String::from_utf8_lossy(&capped.stderr).contains("desktop capability is disabled"),
        "stderr was: {:?}",
        String::from_utf8_lossy(&capped.stderr)
    );

    // Destructive combo refused without touching a backend.
    let logout = Command::new(bin)
        .current_dir(workspace.path())
        .env("HOME", home.path())
        .env("UTHARNESS_HOME", home.path().join(".utharness"))
        .env("UTHARNESS_TOOLS", "desktop")
        .args(["desktop", "key", "ctrl+alt+Delete", "--allow"])
        .output()
        .expect("run utharness");
    assert!(!logout.status.success());
    assert!(
        String::from_utf8_lossy(&logout.stderr).contains("refused"),
        "stderr was: {:?}",
        String::from_utf8_lossy(&logout.stderr)
    );

    let doctor = run_with_env(
        bin,
        workspace.path(),
        home.path(),
        &["desktop", "doctor"],
        &[("UTHARNESS_TOOLS", "desktop")],
    );
    assert!(doctor.contains("DESKTOP"));
    assert!(doctor.contains("policy:"));
}

#[test]
fn desktop_tools_drive_stub_backends_end_to_end() {
    let workspace = tempdir().unwrap();
    let home = tempdir().unwrap();
    let stubs = tempdir().unwrap();
    // Stub `import` (screenshot): writes canned bytes to its last argument.
    std::fs::write(
        stubs.path().join("import"),
        "#!/bin/sh\nprintf 'PNGSTUB' > \"$3\"\n",
    )
    .unwrap();
    // Stub `xdotool` (input): appends its argv to a log file.
    std::fs::write(
        stubs.path().join("xdotool"),
        "#!/bin/sh\necho \"$@\" >> \"$XDOTOOL_LOG\"\n",
    )
    .unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        for tool in ["import", "xdotool"] {
            let path = stubs.path().join(tool);
            let mut perms = std::fs::metadata(&path).unwrap().permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(&path, perms).unwrap();
        }
    }
    let bin = env!("CARGO_BIN_EXE_utharness");
    let stub_path = format!(
        "{}:{}",
        stubs.path().to_str().unwrap(),
        std::env::var("PATH").unwrap()
    );
    let log = home.path().join("xdotool.log");
    let output = Command::new(bin)
        .current_dir(workspace.path())
        .env("HOME", home.path())
        .env("UTHARNESS_HOME", home.path().join(".utharness"))
        .env("PATH", &stub_path)
        .env("UTHARNESS_TOOLS", "desktop")
        .env("DISPLAY", ":9")
        .env_remove("WAYLAND_DISPLAY")
        .env("XDOTOOL_LOG", &log)
        .args(["desktop", "screenshot", "--output", "shot.png", "--allow"])
        .output()
        .expect("run utharness");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        std::fs::read(workspace.path().join("shot.png")).unwrap(),
        b"PNGSTUB"
    );

    for args in [
        vec!["desktop", "click", "10", "20", "--allow"],
        vec!["desktop", "move", "30", "40", "--allow"],
        vec!["desktop", "key", "ctrl+s", "--allow"],
        vec![
            "desktop", "scroll", "down", "--amount", "2", "--x", "5", "--y", "6", "--allow",
        ],
    ] {
        let output = Command::new(bin)
            .current_dir(workspace.path())
            .env("HOME", home.path())
            .env("UTHARNESS_HOME", home.path().join(".utharness"))
            .env("PATH", &stub_path)
            .env("UTHARNESS_TOOLS", "desktop")
            .env("DISPLAY", ":9")
            .env_remove("WAYLAND_DISPLAY")
            .env("XDOTOOL_LOG", &log)
            .args(&args)
            .output()
            .expect("run utharness");
        assert!(
            output.status.success(),
            "{args:?} stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    // Piped text reaches the backend (forbidden patterns still refused).
    let mut child = Command::new(bin)
        .current_dir(workspace.path())
        .env("HOME", home.path())
        .env("UTHARNESS_HOME", home.path().join(".utharness"))
        .env("PATH", &stub_path)
        .env("UTHARNESS_TOOLS", "desktop")
        .env("DISPLAY", ":9")
        .env_remove("WAYLAND_DISPLAY")
        .env("XDOTOOL_LOG", &log)
        .args(["desktop", "type", "--allow"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(b"hello desktop")
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let log_text = std::fs::read_to_string(&log).unwrap();
    assert!(log_text.contains("mousemove 10 20 click 1"), "{log_text}");
    assert!(log_text.contains("mousemove 30 40"), "{log_text}");
    assert!(
        log_text.contains("key --clearmodifiers ctrl+s"),
        "{log_text}"
    );
    assert!(log_text.contains("click 5"), "{log_text}");
    assert!(
        log_text.contains("type --clearmodifiers -- hello desktop"),
        "{log_text}"
    );
}

#[test]
fn cli_persists_workspace_session_memory_and_doctor() {
    let workspace = tempdir().unwrap();
    let home = tempdir().unwrap();
    let bin = env!("CARGO_BIN_EXE_utharness");

    let startup = run(bin, workspace.path(), home.path(), &[]);
    assert!(
        startup.contains("AGENT TERMINAL"),
        "startup stdout was: {startup:?}"
    );
    assert!(!startup.contains("AUTONOMOUS AI AGENT TERMINAL HARNESS"));

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

#[test]
fn provider_and_agent_commands_report_real_runtime_state_without_secrets() {
    let workspace = tempdir().unwrap();
    let home = tempdir().unwrap();
    let bin = env!("CARGO_BIN_EXE_utharness");

    let providers = run(bin, workspace.path(), home.path(), &["providers", "list"]);
    assert!(providers.contains("openrouter"));
    assert!(providers.contains("ollama"));
    assert!(providers.contains("nvidia"));
    assert!(!providers.contains("test-secret"));

    let provider_env = run(bin, workspace.path(), home.path(), &["providers", "env"]);
    assert!(provider_env.contains("GROQ_API_KEY"));
    assert!(provider_env.contains("NVIDIA_API_KEY"));
    assert!(provider_env.contains("never persisted"));

    let agents = run(bin, workspace.path(), home.path(), &["agents", "list"]);
    assert!(agents.contains("Uthy"));
    assert!(agents.contains("SAFE read-only"));
}

#[test]
fn setup_writes_valid_runtime_configuration_without_secrets() {
    let workspace = tempdir().unwrap();
    let home = tempdir().unwrap();
    let bin = env!("CARGO_BIN_EXE_utharness");
    let output = run(
        bin,
        workspace.path(),
        home.path(),
        &[
            "setup",
            "--non-interactive",
            "--mode",
            "full",
            "--provider",
            "ollama",
            "--model",
            "qwen2.5-coder:7b",
            "--skip-validation",
            "--tools",
            "workspace_read,git_inspection,terminal",
        ],
    );
    assert!(output.contains("provider:  ollama"));
    let raw = std::fs::read_to_string(workspace.path().join("utharness.json")).unwrap();
    let config: serde_json::Value = serde_json::from_str(&raw).unwrap();
    assert_eq!(config["schemaVersion"], 1);
    assert_eq!(config["permissionMode"], "ask");
    assert_eq!(config["provider"], "ollama");
    assert_eq!(config["model"], "qwen2.5-coder:7b");
    assert!(raw.contains("workspace_read"));
    assert!(!raw.to_ascii_lowercase().contains("api_key"));
    let global = std::fs::read_to_string(home.path().join(".utharness/config.yaml")).unwrap();
    assert!(global.contains("provider: \"ollama\""));
    assert!(global.contains("secrets_file:"));

    let shown = run(bin, workspace.path(), home.path(), &["config"]);
    assert!(shown.contains("provider = \"ollama\""));
    assert!(shown.contains("setup_mode = \"full\""));
    assert!(shown.contains("permission_mode = \"ask\""));

    let icons = run(
        bin,
        workspace.path(),
        home.path(),
        &["config", "set", "ui.icons", "ascii"],
    );
    assert!(icons.contains("set ui.icons = ascii"));
    let banner = run(
        bin,
        workspace.path(),
        home.path(),
        &["config", "set", "ui.banner", "false"],
    );
    assert!(banner.contains("set ui.banner = false"));
    let updated = std::fs::read_to_string(workspace.path().join("utharness.json")).unwrap();
    assert!(updated.contains("\"icons\": \"ascii\""));
    assert!(updated.contains("\"banner\": false"));
}

#[test]
fn setup_scans_and_persists_validated_secrets_outside_configuration() {
    let workspace = tempdir().unwrap();
    let home = tempdir().unwrap();
    let bin = env!("CARGO_BIN_EXE_utharness");
    let scan = run(bin, workspace.path(), home.path(), &["setup", "--scan"]);
    let report: serde_json::Value = serde_json::from_str(&scan).unwrap();
    assert_eq!(report["os"], std::env::consts::OS);
    assert!(report["components"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item["id"] == "git"));

    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let server = std::thread::spawn(move || {
        let (mut socket, _) = listener.accept().unwrap();
        let mut request = [0_u8; 4096];
        let read = socket.read(&mut request).unwrap();
        let request = String::from_utf8_lossy(&request[..read]);
        assert!(request.contains("GET /v1/models"));
        assert!(request
            .to_ascii_lowercase()
            .contains("authorization: bearer setup-secret"));
        let body = r#"{"data":[{"id":"verified-model"}]}"#;
        write!(socket, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", body.len(), body).unwrap();
    });
    let mut child = Command::new(bin)
        .current_dir(workspace.path())
        .env("HOME", home.path())
        .env("UTHARNESS_HOME", home.path().join(".utharness"))
        .env("UTHARNESS_PROVIDER_URL", format!("http://{address}/v1"))
        .args([
            "setup",
            "--non-interactive",
            "--provider",
            "custom",
            "--model",
            "verified-model",
            "--api-key-stdin",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(b"setup-secret")
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    server.join().unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(!stdout.contains("setup-secret"));
    let config = std::fs::read_to_string(workspace.path().join("utharness.json")).unwrap();
    assert!(!config.contains("setup-secret"));
    let secrets = home.path().join(".utharness/secrets.env");
    assert!(std::fs::read_to_string(&secrets)
        .unwrap()
        .contains("setup-secret"));
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            std::fs::metadata(secrets).unwrap().permissions().mode() & 0o777,
            0o600
        );
    }
}

#[test]
fn chat_streams_from_an_openai_compatible_gateway_and_persists_the_result() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let server = std::thread::spawn(move || {
        let (mut socket, _) = listener.accept().unwrap();
        let mut request = [0_u8; 8192];
        let read = socket.read(&mut request).unwrap();
        let request = String::from_utf8_lossy(&request[..read]);
        assert!(request.contains("POST /v1/chat/completions"));
        assert!(request
            .to_ascii_lowercase()
            .contains("authorization: bearer test-secret"));
        let body = "data: {\"choices\":[{\"delta\":{\"content\":\"live \"}}]}\n\ndata: {\"choices\":[{\"delta\":{\"content\":\"response\"}}]}\n\ndata: [DONE]\n\n";
        write!(
            socket,
            "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        )
        .unwrap();
    });

    let workspace = tempdir().unwrap();
    let home = tempdir().unwrap();
    let endpoint = format!("http://{address}/v1");
    let output = run_with_env(
        env!("CARGO_BIN_EXE_utharness"),
        workspace.path(),
        home.path(),
        &["chat", "hello gateway"],
        &[
            ("UTHARNESS_PROVIDER", "custom"),
            ("UTHARNESS_PROVIDER_URL", &endpoint),
            ("UTHARNESS_MODEL", "fixture-model"),
            ("UTHARNESS_API_KEY", "test-secret"),
        ],
    );
    server.join().unwrap();
    assert!(output.contains("Uthy · custom/fixture-model"));
    assert!(output.contains("live response"));
    assert!(!output.contains("test-secret"));

    let sessions = run(
        env!("CARGO_BIN_EXE_utharness"),
        workspace.path(),
        home.path(),
        &["sessions", "list"],
    );
    assert!(sessions.contains("Terminal session"));
}
