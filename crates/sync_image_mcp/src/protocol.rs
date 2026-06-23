use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::latest::load_latest_image;

#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub image_root: PathBuf,
    pub image_user: String,
}

#[derive(Debug, Deserialize)]
struct JsonRpcRequest {
    id: Option<Value>,
    method: String,
}

pub fn run_stdio<R, W>(config: ServerConfig, reader: R, mut writer: W) -> Result<()>
where
    R: BufRead,
    W: Write,
{
    for line in reader.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }

        let response = handle_line(&config, &line);
        if let Some(response) = response {
            serde_json::to_writer(&mut writer, &response)?;
            writer.write_all(b"\n")?;
            writer.flush()?;
        }
    }

    Ok(())
}

fn handle_line(config: &ServerConfig, line: &str) -> Option<Value> {
    let request = match serde_json::from_str::<JsonRpcRequest>(line) {
        Ok(request) => request,
        Err(err) => {
            return Some(error_response(
                None,
                -32700,
                format!("failed to parse JSON-RPC request: {err}"),
            ));
        }
    };

    let id = request.id.clone();
    let result = match request.method.as_str() {
        "initialize" => initialize_result(),
        "tools/list" => tools_list_result(),
        "tools/call" => tool_call_result(config),
        "notifications/initialized" => return None,
        method => {
            return Some(error_response(
                id,
                -32601,
                format!("unsupported method '{method}'"),
            ));
        }
    };

    id.map(|id| json!({ "jsonrpc": "2.0", "id": id, "result": result }))
}

fn initialize_result() -> Value {
    json!({
        "protocolVersion": "2024-11-05",
        "capabilities": {
            "tools": {}
        },
        "serverInfo": {
            "name": "sync-image-mcp",
            "version": env!("CARGO_PKG_VERSION")
        }
    })
}

fn tools_list_result() -> Value {
    json!({
        "tools": [{
            "name": "get_latest_screenshot",
            "description": "Return the latest uploaded screenshot/image for CLAUDE_IMAGE_USER.",
            "inputSchema": {
                "type": "object",
                "properties": {},
                "additionalProperties": false
            }
        }]
    })
}

fn tool_call_result(config: &ServerConfig) -> Value {
    match latest_content(&config.image_root, &config.image_user) {
        Ok(content) => json!({ "content": content, "isError": false }),
        Err(err) => json!({
            "content": [{
                "type": "text",
                "text": err.to_string()
            }],
            "isError": true
        }),
    }
}

fn latest_content(root: &Path, user_name: &str) -> Result<Vec<Value>> {
    let latest = load_latest_image(root, user_name)?;
    let text = format!(
        "selected_file={}\nupload_time={}",
        latest.selected.file_name,
        latest.upload_time_text()
    );

    Ok(vec![
        json!({ "type": "text", "text": text }),
        json!({
            "type": "image",
            "data": latest.data_base64,
            "mimeType": "image/png"
        }),
    ])
}

fn error_response(id: Option<Value>, code: i64, message: String) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id.unwrap_or(Value::Null),
        "error": {
            "code": code,
            "message": message
        }
    })
}

pub fn config_from_env() -> Result<ServerConfig> {
    let image_root = std::env::var("CLAUDE_IMAGE_ROOT")
        .context("CLAUDE_IMAGE_ROOT is required, for example /data/claude-images")?;
    let image_user = std::env::var("CLAUDE_IMAGE_USER")
        .context("CLAUDE_IMAGE_USER is required, usually set to $USER")?;

    Ok(ServerConfig {
        image_root: PathBuf::from(image_root),
        image_user,
    })
}

#[cfg(test)]
mod tests {
    use std::{
        fs::{self, File},
        io::{BufReader, Cursor, Write},
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };

    use super::*;

    #[test]
    fn tool_call_returns_latest_image_content() {
        let root = temp_root();
        let user_dir = root.join("alice");
        fs::create_dir_all(&user_dir).expect("create dir");
        let mut file = File::create(user_dir.join("20260623_010203_104.png")).expect("image");
        file.write_all(b"png-bytes").expect("write");

        let input = r#"{"jsonrpc":"2.0","id":1,"method":"tools/call"}"#;
        let mut output = Vec::new();
        run_stdio(
            ServerConfig {
                image_root: root.clone(),
                image_user: "alice".to_string(),
            },
            BufReader::new(Cursor::new(format!("{input}\n"))),
            &mut output,
        )
        .expect("server run");

        let response: Value = serde_json::from_slice(&output).expect("json");
        let content = response["result"]["content"]
            .as_array()
            .expect("content array");
        assert_eq!(content[0]["type"], "text");
        assert!(
            content[0]["text"]
                .as_str()
                .unwrap()
                .contains("selected_file")
        );
        assert_eq!(content[1]["type"], "image");
        assert_eq!(content[1]["data"], "cG5nLWJ5dGVz");

        fs::remove_dir_all(root).ok();
    }

    #[test]
    fn tool_call_returns_mcp_error_when_no_image_exists() {
        let root = temp_root();
        fs::create_dir_all(root.join("alice")).expect("create dir");

        let input = r#"{"jsonrpc":"2.0","id":1,"method":"tools/call"}"#;
        let mut output = Vec::new();
        run_stdio(
            ServerConfig {
                image_root: root.clone(),
                image_user: "alice".to_string(),
            },
            BufReader::new(Cursor::new(format!("{input}\n"))),
            &mut output,
        )
        .expect("server run");

        let response: Value = serde_json::from_slice(&output).expect("json");
        assert_eq!(response["result"]["isError"], true);

        fs::remove_dir_all(root).ok();
    }

    fn temp_root() -> PathBuf {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        std::env::temp_dir().join(format!("sync-image-mcp-protocol-test-{suffix}"))
    }
}
