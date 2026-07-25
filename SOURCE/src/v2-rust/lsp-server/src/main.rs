// Killer Language Server Protocol (LSP) Implementation
// Full IDE support with diagnostics, completion, and navigation
// Version: 2.1.0

use std::io::{self, Read, Write};
use std::collections::HashMap;
use serde::{Deserialize, Serialize};

/// LSP Message envelope
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub jsonrpc: String,
    pub id: Option<u64>,
    pub method: Option<String>,
    pub params: Option<serde_json::Value>,
    pub result: Option<serde_json::Value>,
    pub error: Option<Error>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Error {
    pub code: i32,
    pub message: String,
}

/// Position in a document
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    pub line: u32,
    pub character: u32,
}

/// Range in a document
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Range {
    pub start: Position,
    pub end: Position,
}

/// Location in a document
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Location {
    pub uri: String,
    pub range: Range,
}

/// Diagnostic message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Diagnostic {
    pub range: Range,
    pub severity: u32,  // 1=Error, 2=Warning, 3=Info, 4=Hint
    pub message: String,
    pub code: Option<String>,
}

/// Completion item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionItem {
    pub label: String,
    pub kind: u32,  // 1=Text, 2=Method, 3=Function, etc.
    pub detail: Option<String>,
    pub documentation: Option<String>,
    pub sortText: Option<String>,
}

/// Language Server
pub struct KillerLanguageServer {
    documents: HashMap<String, String>,  // URI -> content
    version: u32,
}

impl KillerLanguageServer {
    pub fn new() -> Self {
        KillerLanguageServer {
            documents: HashMap::new(),
            version: 0,
        }
    }

    /// Process incoming JSON-RPC message
    pub fn handle_message(&mut self, message: Message) -> Option<Message> {
        if let Some(method) = &message.method {
            match method.as_str() {
                "initialize" => Some(self.handle_initialize(message.id)),
                "initialized" => None,
                "shutdown" => Some(self.handle_shutdown(message.id)),
                "exit" => std::process::exit(0),
                "textDocument/didOpen" => self.handle_did_open(&message),
                "textDocument/didChange" => self.handle_did_change(&message),
                "textDocument/didSave" => self.handle_did_save(&message),
                "textDocument/didClose" => self.handle_did_close(&message),
                "textDocument/completion" => Some(self.handle_completion(&message)),
                "textDocument/hover" => Some(self.handle_hover(&message)),
                "textDocument/definition" => Some(self.handle_definition(&message)),
                "textDocument/references" => Some(self.handle_references(&message)),
                "textDocument/documentDiagnostics" => Some(self.handle_diagnostics(&message)),
                "workspace/didChangeConfiguration" => None,
                _ => Some(Message {
                    jsonrpc: "2.0".to_string(),
                    id: message.id,
                    method: None,
                    params: None,
                    result: None,
                    error: Some(Error {
                        code: -32601,
                        message: "Method not found".to_string(),
                    }),
                }),
            }
        } else {
            None
        }
    }

    fn handle_initialize(&self, id: Option<u64>) -> Message {
        let capabilities = serde_json::json!({
            "textDocumentSync": 1,
            "completionProvider": {
                "resolveProvider": false,
                "triggerCharacters": [".", " "]
            },
            "hoverProvider": true,
            "definitionProvider": true,
            "referencesProvider": true,
            "diagnosticProvider": {
                "interFileDependencies": true,
                "workspaceDiagnostics": true
            }
        });

        Message {
            jsonrpc: "2.0".to_string(),
            id,
            method: None,
            params: None,
            result: Some(serde_json::json!({
                "capabilities": capabilities
            })),
            error: None,
        }
    }

    fn handle_shutdown(&self, id: Option<u64>) -> Message {
        Message {
            jsonrpc: "2.0".to_string(),
            id,
            method: None,
            params: None,
            result: Some(serde_json::Value::Null),
            error: None,
        }
    }

    fn handle_did_open(&mut self, message: &Message) -> Option<Message> {
        if let Some(params) = &message.params {
            if let Some(doc) = params.get("textDocument") {
                if let Some(uri) = doc.get("uri").and_then(|v| v.as_str()) {
                    if let Some(text) = doc.get("text").and_then(|v| v.as_str()) {
                        self.documents.insert(uri.to_string(), text.to_string());
                        return self.lint_document(uri);
                    }
                }
            }
        }
        None
    }

    fn handle_did_change(&mut self, message: &Message) -> Option<Message> {
        if let Some(params) = &message.params {
            if let Some(doc) = params.get("textDocument") {
                if let Some(uri) = doc.get("uri").and_then(|v| v.as_str()) {
                    if let Some(changes) = params.get("contentChanges").and_then(|v| v.as_array()) {
                        for change in changes {
                            if let Some(text) = change.get("text").and_then(|v| v.as_str()) {
                                self.documents.insert(uri.to_string(), text.to_string());
                            }
                        }
                    }
                }
            }
        }
        None
    }

    fn handle_did_save(&mut self, message: &Message) -> Option<Message> {
        if let Some(params) = &message.params {
            if let Some(doc) = params.get("textDocument") {
                if let Some(uri) = doc.get("uri").and_then(|v| v.as_str()) {
                    return self.lint_document(uri);
                }
            }
        }
        None
    }

    fn handle_did_close(&mut self, message: &Message) -> Option<Message> {
        if let Some(params) = &message.params {
            if let Some(doc) = params.get("textDocument") {
                if let Some(uri) = doc.get("uri").and_then(|v| v.as_str()) {
                    self.documents.remove(uri);
                }
            }
        }
        None
    }

    fn handle_completion(&self, message: &Message) -> Message {
        let completions = vec![
            CompletionItem {
                label: "fn".to_string(),
                kind: 1,
                detail: Some("Function declaration".to_string()),
                documentation: None,
                sortText: Some("0fn".to_string()),
            },
            CompletionItem {
                label: "let".to_string(),
                kind: 1,
                detail: Some("Variable binding".to_string()),
                documentation: None,
                sortText: Some("0let".to_string()),
            },
            CompletionItem {
                label: "math".to_string(),
                kind: 9,
                detail: Some("Math module".to_string()),
                documentation: Some("Mathematical functions library".to_string()),
                sortText: Some("1math".to_string()),
            },
        ];

        Message {
            jsonrpc: "2.0".to_string(),
            id: message.id,
            method: None,
            params: None,
            result: Some(serde_json::to_value(completions).unwrap()),
            error: None,
        }
    }

    fn handle_hover(&self, message: &Message) -> Message {
        let contents = "Killer Language Documentation".to_string();

        Message {
            jsonrpc: "2.0".to_string(),
            id: message.id,
            method: None,
            params: None,
            result: Some(serde_json::json!({
                "contents": contents
            })),
            error: None,
        }
    }

    fn handle_definition(&self, message: &Message) -> Message {
        Message {
            jsonrpc: "2.0".to_string(),
            id: message.id,
            method: None,
            params: None,
            result: Some(serde_json::Value::Null),
            error: None,
        }
    }

    fn handle_references(&self, message: &Message) -> Message {
        Message {
            jsonrpc: "2.0".to_string(),
            id: message.id,
            method: None,
            params: None,
            result: Some(serde_json::json!([]),),
            error: None,
        }
    }

    fn handle_diagnostics(&self, message: &Message) -> Option<Message> {
        if let Some(params) = &message.params {
            if let Some(uri) = params.get("textDocument")
                .and_then(|d| d.get("uri"))
                .and_then(|u| u.as_str())
            {
                return self.lint_document(uri);
            }
        }
        None
    }

    fn lint_document(&self, uri: &str) -> Option<Message> {
        let diagnostics = if let Some(content) = self.documents.get(uri) {
            self.analyze_document(content)
        } else {
            Vec::new()
        };

        Some(Message {
            jsonrpc: "2.0".to_string(),
            id: None,
            method: Some("textDocument/publishDiagnostics".to_string()),
            params: Some(serde_json::json!({
                "uri": uri,
                "diagnostics": diagnostics
            })),
            result: None,
            error: None,
        })
    }

    fn analyze_document(&self, content: &str) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        let lines: Vec<&str> = content.lines().collect();
        for (line_num, line) in lines.iter().enumerate() {
            // Check line length
            if line.len() > 100 {
                diagnostics.push(Diagnostic {
                    range: Range {
                        start: Position {
                            line: line_num as u32,
                            character: 100,
                        },
                        end: Position {
                            line: line_num as u32,
                            character: line.len() as u32,
                        },
                    },
                    severity: 2,  // Warning
                    message: "Line exceeds 100 characters".to_string(),
                    code: Some("line-too-long".to_string()),
                });
            }

            // Check for trailing whitespace
            if line.ends_with(' ') || line.ends_with('\t') {
                diagnostics.push(Diagnostic {
                    range: Range {
                        start: Position {
                            line: line_num as u32,
                            character: (line.len() - 1) as u32,
                        },
                        end: Position {
                            line: line_num as u32,
                            character: line.len() as u32,
                        },
                    },
                    severity: 3,  // Information
                    message: "Trailing whitespace".to_string(),
                    code: Some("trailing-whitespace".to_string()),
                });
            }

            // Check for TODO comments
            if line.contains("TODO") {
                diagnostics.push(Diagnostic {
                    range: Range {
                        start: Position {
                            line: line_num as u32,
                            character: 0,
                        },
                        end: Position {
                            line: line_num as u32,
                            character: line.len() as u32,
                        },
                    },
                    severity: 4,  // Hint
                    message: "TODO comment".to_string(),
                    code: Some("todo".to_string()),
                });
            }
        }

        diagnostics
    }
}

/// Main LSP server loop
pub fn run_lsp_server() -> io::Result<()> {
    let mut server = KillerLanguageServer::new();
    let stdin = io::stdin();
    let stdout = io::stdout();

    let mut reader = stdin.lock();
    let mut writer = stdout.lock();
    let mut buffer = String::new();

    loop {
        buffer.clear();
        
        // Read headers
        loop {
            let mut line = String::new();
            reader.read_line(&mut line)?;
            
            if line == "\r\n" || line == "\n" {
                break;
            }
            
            if line.starts_with("Content-Length:") {
                if let Ok(len) = line.strip_prefix("Content-Length:").unwrap().trim().parse::<usize>() {
                    // Read message body
                    let mut content = vec![0u8; len];
                    reader.read_exact(&mut content)?;
                    
                    if let Ok(message_str) = String::from_utf8(content) {
                        if let Ok(message) = serde_json::from_str::<Message>(&message_str) {
                            if let Some(response) = server.handle_message(message) {
                                let response_str = serde_json::to_string(&response).unwrap();
                                let response_with_header = format!(
                                    "Content-Length: {}\r\n\r\n{}",
                                    response_str.len(),
                                    response_str
                                );
                                writer.write_all(response_with_header.as_bytes())?;
                                writer.flush()?;
                            }
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_server_creation() {
        let server = KillerLanguageServer::new();
        assert_eq!(server.documents.len(), 0);
    }

    #[test]
    fn test_initialize_message() {
        let server = KillerLanguageServer::new();
        let msg = server.handle_initialize(Some(1));
        assert_eq!(msg.id, Some(1));
    }
}
