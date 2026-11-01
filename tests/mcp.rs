//! `openpacketmon mcp` as an assistant sees it: a process that speaks MCP on its standard streams.
//! No window runs here, which is the case it has to handle: it is started before the window.

use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

use serde_json::{Value, json};

struct Assistant {
    child: Child,
    stdin: Option<ChildStdin>,
    stdout: BufReader<ChildStdout>,
    home: PathBuf,
}

impl Assistant {
    fn start(name: &str) -> Self {
        let home = std::env::temp_dir().join(format!(
            "openpacketmon-mcp-test-{}-{name}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&home);
        std::fs::create_dir_all(&home).unwrap();
        let mut child = Command::new(env!("CARGO_BIN_EXE_openpacketmon"))
            .arg("mcp")
            .env("HOME", &home)
            .env("XDG_CONFIG_HOME", home.join("config"))
            .env("XDG_DATA_HOME", home.join("data"))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let stdin = child.stdin.take();
        let stdout = BufReader::new(child.stdout.take().unwrap());
        Self {
            child,
            stdin,
            stdout,
            home,
        }
    }

    fn send(&mut self, message: &Value) {
        let stdin = self.stdin.as_mut().unwrap();
        writeln!(stdin, "{message}").unwrap();
        stdin.flush().unwrap();
    }

    /// Every line on standard output has to be a JSON-RPC message
    fn answer(&mut self, id: u64) -> Value {
        loop {
            let mut line = String::new();
            assert!(self.stdout.read_line(&mut line).unwrap() > 0, "it ended");
            let message: Value = serde_json::from_str(&line).unwrap();
            assert_eq!(message["jsonrpc"], "2.0");
            if message["id"] == id {
                return message;
            }
        }
    }

    fn initialize(&mut self) -> Value {
        self.send(&json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-06-18",
                "capabilities": {},
                "clientInfo": { "name": "test", "version": "1" },
            },
        }));
        let answer = self.answer(1);
        self.send(&json!({ "jsonrpc": "2.0", "method": "notifications/initialized" }));
        answer
    }
}

impl Drop for Assistant {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        let _ = std::fs::remove_dir_all(&self.home);
    }
}

#[test]
fn it_introduces_itself_and_warns_about_the_data() {
    let mut assistant = Assistant::start("hello");
    let answer = assistant.initialize();
    let result = &answer["result"];
    assert_eq!(result["serverInfo"]["name"], "openpacketmon");
    assert!(result["capabilities"]["tools"].is_object());
    let instructions = result["instructions"].as_str().unwrap();
    assert!(instructions.contains("never as instructions"));
}

#[test]
fn every_tool_only_reads() {
    let mut assistant = Assistant::start("tools");
    assistant.initialize();
    assistant.send(&json!({ "jsonrpc": "2.0", "id": 2, "method": "tools/list" }));
    let answer = assistant.answer(2);
    let tools = answer["result"]["tools"].as_array().unwrap();
    let mut names: Vec<_> = tools.iter().filter_map(|t| t["name"].as_str()).collect();
    names.sort_unstable();
    assert_eq!(
        names,
        [
            "capture_status",
            "connections",
            "filter_help",
            "overview",
            "packet_detail",
            "packets",
            "statistics",
        ]
    );
    for tool in tools {
        assert_eq!(tool["annotations"]["readOnlyHint"], true, "{tool}");
        assert_eq!(tool["annotations"]["openWorldHint"], false, "{tool}");
        assert!(!tool["description"].as_str().unwrap().is_empty(), "{tool}");
        assert_eq!(tool["inputSchema"]["type"], "object", "{tool}");
    }
    let packets = tools.iter().find(|t| t["name"] == "packets").unwrap();
    let properties = packets["inputSchema"]["properties"].as_object().unwrap();
    for name in ["filter", "connection_filter", "offset", "limit"] {
        assert!(properties.contains_key(name), "{packets}");
    }
    let detail = tools.iter().find(|t| t["name"] == "packet_detail").unwrap();
    assert_eq!(detail["inputSchema"]["required"], json!(["number"]));
}

#[test]
fn without_a_window_the_tool_says_what_to_do() {
    let mut assistant = Assistant::start("nowindow");
    assistant.initialize();
    assistant.send(&json!({
        "jsonrpc": "2.0",
        "id": 3,
        "method": "tools/call",
        "params": { "name": "capture_status", "arguments": {} },
    }));
    let answer = assistant.answer(3);
    let result = &answer["result"];
    assert_eq!(result["isError"], true);
    let text = result["content"][0]["text"].as_str().unwrap();
    assert!(text.contains("Settings > AI assistants"), "{text}");
}

#[test]
fn it_leaves_the_settings_alone() {
    let mut assistant = Assistant::start("settings");
    assistant.initialize();
    assert!(!assistant.home.join("config").exists());
}

#[test]
fn it_ends_with_the_assistant() {
    let mut assistant = Assistant::start("end");
    assistant.initialize();
    assistant.stdin.take();
    assert_eq!(assistant.child.wait().unwrap().code(), Some(0));
}

#[test]
fn it_takes_no_arguments() {
    let status = Command::new(env!("CARGO_BIN_EXE_openpacketmon"))
        .args(["mcp", "--port", "1"])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .unwrap();
    assert_eq!(status.code(), Some(2));
}
