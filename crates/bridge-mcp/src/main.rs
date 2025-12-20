//! MCP Server for Coachly Code Bridge
//!
//! Provides Model Context Protocol server for Claude Code integration.
//! Exposes file sharing, sync, and screenshot tools.

use anyhow::Result;
use axum::{
    extract::State,
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use bridge_core::{Bridge, Config};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_http::cors::CorsLayer;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

mod tools;

/// MCP Server State
pub struct McpState {
    bridge: RwLock<Bridge>,
    config: Config,
}

/// MCP Protocol Messages

#[derive(Debug, Serialize, Deserialize)]
struct McpRequest {
    jsonrpc: String,
    id: Option<serde_json::Value>,
    method: String,
    params: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
struct McpResponse {
    jsonrpc: String,
    id: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<McpError>,
}

#[derive(Debug, Serialize, Deserialize)]
struct McpError {
    code: i32,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ServerInfo {
    name: String,
    version: String,
    protocol_version: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct Tool {
    name: String,
    description: String,
    input_schema: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize)]
struct ToolResult {
    content: Vec<ToolContent>,
    is_error: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ToolContent {
    #[serde(rename = "type")]
    content_type: String,
    text: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    info!("Starting Code Bridge MCP Server...");

    // Check for stdio vs HTTP mode
    let args: Vec<String> = std::env::args().collect();
    let use_stdio = args.get(1).map(|s| s == "--stdio").unwrap_or(false);

    if use_stdio {
        // Run in stdio mode for Claude Code
        run_stdio_server().await
    } else {
        // Run in HTTP mode for debugging/testing
        let port: u16 = args
            .get(1)
            .and_then(|s| s.parse().ok())
            .unwrap_or(3100);
        run_http_server(port).await
    }
}

async fn run_stdio_server() -> Result<()> {
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

    info!("Running in stdio mode");

    let config = Config::load_or_create()?;
    let bridge = Bridge::new().await?;

    let state = Arc::new(McpState {
        bridge: RwLock::new(bridge),
        config,
    });

    let stdin = tokio::io::stdin();
    let mut stdout = tokio::io::stdout();
    let mut reader = BufReader::new(stdin);

    loop {
        let mut line = String::new();
        match reader.read_line(&mut line).await {
            Ok(0) => break, // EOF
            Ok(_) => {
                if line.trim().is_empty() {
                    continue;
                }

                match serde_json::from_str::<McpRequest>(&line) {
                    Ok(request) => {
                        let response = handle_mcp_request(&state, request).await;
                        let response_json = serde_json::to_string(&response)?;
                        stdout.write_all(response_json.as_bytes()).await?;
                        stdout.write_all(b"\n").await?;
                        stdout.flush().await?;
                    }
                    Err(e) => {
                        let error_response = McpResponse {
                            jsonrpc: "2.0".to_string(),
                            id: None,
                            result: None,
                            error: Some(McpError {
                                code: -32700,
                                message: format!("Parse error: {}", e),
                                data: None,
                            }),
                        };
                        let response_json = serde_json::to_string(&error_response)?;
                        stdout.write_all(response_json.as_bytes()).await?;
                        stdout.write_all(b"\n").await?;
                        stdout.flush().await?;
                    }
                }
            }
            Err(e) => {
                eprintln!("Error reading stdin: {}", e);
                break;
            }
        }
    }

    Ok(())
}

async fn run_http_server(port: u16) -> Result<()> {
    info!("Running in HTTP mode on port {}", port);

    let config = Config::load_or_create()?;
    let bridge = Bridge::new().await?;

    let state = Arc::new(McpState {
        bridge: RwLock::new(bridge),
        config,
    });

    let app = Router::new()
        .route("/", get(health_check))
        .route("/mcp", post(handle_mcp_http))
        .layer(CorsLayer::permissive())
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port)).await?;
    info!("MCP Server listening on http://0.0.0.0:{}", port);

    axum::serve(listener, app).await?;

    Ok(())
}

async fn health_check() -> &'static str {
    "Code Bridge MCP Server"
}

async fn handle_mcp_http(
    State(state): State<Arc<McpState>>,
    Json(request): Json<McpRequest>,
) -> (StatusCode, Json<McpResponse>) {
    let response = handle_mcp_request(&state, request).await;
    (StatusCode::OK, Json(response))
}

async fn handle_mcp_request(state: &Arc<McpState>, request: McpRequest) -> McpResponse {
    let result = match request.method.as_str() {
        "initialize" => handle_initialize(&request).await,
        "tools/list" => handle_tools_list().await,
        "tools/call" => handle_tools_call(state, &request).await,
        "resources/list" => handle_resources_list().await,
        "resources/read" => handle_resources_read(state, &request).await,
        _ => Err(McpError {
            code: -32601,
            message: format!("Method not found: {}", request.method),
            data: None,
        }),
    };

    match result {
        Ok(result) => McpResponse {
            jsonrpc: "2.0".to_string(),
            id: request.id,
            result: Some(result),
            error: None,
        },
        Err(error) => McpResponse {
            jsonrpc: "2.0".to_string(),
            id: request.id,
            result: None,
            error: Some(error),
        },
    }
}

async fn handle_initialize(_request: &McpRequest) -> Result<serde_json::Value, McpError> {
    Ok(serde_json::json!({
        "protocolVersion": "2024-11-05",
        "capabilities": {
            "tools": {},
            "resources": {}
        },
        "serverInfo": {
            "name": "code-bridge",
            "version": env!("CARGO_PKG_VERSION")
        }
    }))
}

async fn handle_tools_list() -> Result<serde_json::Value, McpError> {
    let tools = vec![
        Tool {
            name: "bridge_status".to_string(),
            description: "Get the current status of Code Bridge including connected peers and sync state".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
        },
        Tool {
            name: "bridge_sync".to_string(),
            description: "Trigger a sync with all connected peers".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "project": {
                        "type": "string",
                        "description": "Project name to sync (optional, syncs all if not specified)"
                    }
                },
                "required": []
            }),
        },
        Tool {
            name: "bridge_share".to_string(),
            description: "Share a file or directory with connected peers".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Path to the file or directory to share"
                    },
                    "peer": {
                        "type": "string",
                        "description": "Specific peer to share with (optional, shares with all if not specified)"
                    }
                },
                "required": ["path"]
            }),
        },
        Tool {
            name: "bridge_peers".to_string(),
            description: "List all discovered and connected peers on the network".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
        },
        Tool {
            name: "bridge_screenshots".to_string(),
            description: "List recent screenshots or search by OCR text".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "search": {
                        "type": "string",
                        "description": "Search query for OCR text in screenshots"
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Maximum number of screenshots to return",
                        "default": 10
                    }
                },
                "required": []
            }),
        },
        Tool {
            name: "bridge_files".to_string(),
            description: "List all tracked files in the content store".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Filter by path prefix"
                    }
                },
                "required": []
            }),
        },
    ];

    Ok(serde_json::json!({ "tools": tools }))
}

async fn handle_tools_call(
    state: &Arc<McpState>,
    request: &McpRequest,
) -> Result<serde_json::Value, McpError> {
    let params = request.params.as_ref().ok_or(McpError {
        code: -32602,
        message: "Missing params".to_string(),
        data: None,
    })?;

    let tool_name = params
        .get("name")
        .and_then(|v| v.as_str())
        .ok_or(McpError {
            code: -32602,
            message: "Missing tool name".to_string(),
            data: None,
        })?;

    let arguments = params.get("arguments").cloned().unwrap_or(serde_json::json!({}));

    let result = match tool_name {
        "bridge_status" => tools::status(state).await,
        "bridge_sync" => tools::sync(state, &arguments).await,
        "bridge_share" => tools::share(state, &arguments).await,
        "bridge_peers" => tools::peers(state).await,
        "bridge_screenshots" => tools::screenshots(state, &arguments).await,
        "bridge_files" => tools::files(state, &arguments).await,
        _ => Err(format!("Unknown tool: {}", tool_name)),
    };

    match result {
        Ok(text) => Ok(serde_json::json!({
            "content": [{
                "type": "text",
                "text": text
            }]
        })),
        Err(e) => Ok(serde_json::json!({
            "content": [{
                "type": "text",
                "text": format!("Error: {}", e)
            }],
            "isError": true
        })),
    }
}

async fn handle_resources_list() -> Result<serde_json::Value, McpError> {
    Ok(serde_json::json!({
        "resources": [
            {
                "uri": "bridge://status",
                "name": "Bridge Status",
                "description": "Current Code Bridge status and configuration",
                "mimeType": "application/json"
            },
            {
                "uri": "bridge://peers",
                "name": "Connected Peers",
                "description": "List of discovered and connected peers",
                "mimeType": "application/json"
            },
            {
                "uri": "bridge://files",
                "name": "Tracked Files",
                "description": "All files in the content store",
                "mimeType": "application/json"
            }
        ]
    }))
}

async fn handle_resources_read(
    state: &Arc<McpState>,
    request: &McpRequest,
) -> Result<serde_json::Value, McpError> {
    let params = request.params.as_ref().ok_or(McpError {
        code: -32602,
        message: "Missing params".to_string(),
        data: None,
    })?;

    let uri = params
        .get("uri")
        .and_then(|v| v.as_str())
        .ok_or(McpError {
            code: -32602,
            message: "Missing uri".to_string(),
            data: None,
        })?;

    let content = match uri {
        "bridge://status" => {
            let status = tools::status(state).await.unwrap_or_default();
            serde_json::json!({
                "uri": uri,
                "mimeType": "text/plain",
                "text": status
            })
        }
        "bridge://peers" => {
            let peers = tools::peers(state).await.unwrap_or_default();
            serde_json::json!({
                "uri": uri,
                "mimeType": "text/plain",
                "text": peers
            })
        }
        "bridge://files" => {
            let files = tools::files(state, &serde_json::json!({})).await.unwrap_or_default();
            serde_json::json!({
                "uri": uri,
                "mimeType": "text/plain",
                "text": files
            })
        }
        _ => {
            return Err(McpError {
                code: -32602,
                message: format!("Unknown resource: {}", uri),
                data: None,
            });
        }
    };

    Ok(serde_json::json!({
        "contents": [content]
    }))
}
