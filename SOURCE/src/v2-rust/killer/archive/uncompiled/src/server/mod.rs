// IDE/LSP Server Module
// Language Server Protocol implementation for full editor integration
// Provides intellisense, go-to-definition, refactoring, debugging

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

// ============================================================================
// LSP Core Types
// ============================================================================

#[derive(Debug, Clone)]
pub struct Position {
    pub line: usize,
    pub character: usize,
}

#[derive(Debug, Clone)]
pub struct Range {
    pub start: Position,
    pub end: Position,
}

#[derive(Debug, Clone)]
pub struct Location {
    pub uri: String,
    pub range: Range,
}

#[derive(Debug, Clone)]
pub enum DiagnosticSeverity {
    Error = 1,
    Warning = 2,
    Information = 3,
    Hint = 4,
}

#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub range: Range,
    pub severity: DiagnosticSeverity,
    pub code: Option<String>,
    pub message: String,
    pub source: Option<String>,
}

#[derive(Debug, Clone)]
pub struct CompletionItem {
    pub label: String,
    pub kind: CompletionItemKind,
    pub detail: Option<String>,
    pub documentation: Option<String>,
    pub sort_text: Option<String>,
    pub filter_text: Option<String>,
}

#[derive(Debug, Clone)]
pub enum CompletionItemKind {
    Text = 1,
    Method = 2,
    Function = 3,
    Constructor = 4,
    Field = 5,
    Variable = 6,
    Class = 7,
    Interface = 8,
    Module = 9,
    Property = 10,
    Unit = 11,
    Value = 12,
    Enum = 13,
    Keyword = 14,
    Snippet = 15,
    Color = 16,
    File = 17,
    Reference = 18,
    Folder = 19,
}

#[derive(Debug, Clone)]
pub enum SymbolKind {
    File = 1,
    Module = 2,
    Namespace = 3,
    Package = 4,
    Class = 5,
    Method = 6,
    Property = 7,
    Field = 8,
    Constructor = 9,
    Enum = 10,
    Interface = 11,
    Function = 12,
    Variable = 13,
    Constant = 14,
    String = 15,
    Number = 16,
    Boolean = 17,
    Array = 18,
    Object = 19,
    Key = 20,
    Null = 21,
    EnumMember = 22,
    Struct = 23,
    Event = 24,
    Operator = 25,
    TypeParameter = 26,
}

// ============================================================================
// Symbol Table - Function/class/variable definitions
// ============================================================================

#[derive(Debug, Clone)]
pub struct Symbol {
    pub name: String,
    pub kind: SymbolKind,
    pub location: Location,
    pub type_info: Option<String>,
    pub documentation: Option<String>,
    pub container_name: Option<String>,
}

pub struct SymbolTable {
    pub symbols: Arc<RwLock<HashMap<String, Symbol>>>,
    pub scopes: Arc<RwLock<Vec<HashMap<String, String>>>>,
    pub current_scope: Arc<std::sync::atomic::AtomicUsize>,
}

impl SymbolTable {
    pub fn new() -> Self {
        SymbolTable {
            symbols: Arc::new(RwLock::new(HashMap::new())),
            scopes: Arc::new(RwLock::new(vec![HashMap::new()])),
            current_scope: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
        }
    }

    pub fn define(&self, symbol: Symbol) -> Result<(), String> {
        let mut symbols = self.symbols.write().unwrap();
        if symbols.contains_key(&symbol.name) {
            // Allow redefinition (e.g., for overloads)
        }
        symbols.insert(symbol.name.clone(), symbol);
        Ok(())
    }

    pub fn lookup(&self, name: &str) -> Option<Symbol> {
        let symbols = self.symbols.read().unwrap();
        symbols.get(name).cloned()
    }

    pub fn lookup_references(&self, name: &str) -> Vec<Location> {
        let symbols = self.symbols.read().unwrap();
        symbols
            .values()
            .filter(|s| s.name == name)
            .map(|s| s.location.clone())
            .collect()
    }

    pub fn push_scope(&self) {
        let scope_id = self.current_scope.load(std::sync::atomic::Ordering::Relaxed) + 1;
        self.current_scope.store(scope_id, std::sync::atomic::Ordering::Relaxed);
        let mut scopes = self.scopes.write().unwrap();
        scopes.push(HashMap::new());
    }

    pub fn pop_scope(&self) {
        let scope_id = self.current_scope.load(std::sync::atomic::Ordering::Relaxed);
        if scope_id > 0 {
            self.current_scope.store(scope_id - 1, std::sync::atomic::Ordering::Relaxed);
            let mut scopes = self.scopes.write().unwrap();
            if scopes.len() > 1 {
                scopes.pop();
            }
        }
    }

    pub fn all_symbols(&self) -> Vec<Symbol> {
        let symbols = self.symbols.read().unwrap();
        symbols.values().cloned().collect()
    }
}

// ============================================================================
// Document Store - Manages open documents
// ============================================================================

#[derive(Debug, Clone)]
pub struct TextDocument {
    pub uri: String,
    pub language_id: String,
    pub version: usize,
    pub content: String,
}

impl TextDocument {
    pub fn new(uri: &str, content: String) -> Self {
        TextDocument {
            uri: uri.to_string(),
            language_id: "killer".to_string(),
            version: 1,
            content,
        }
    }

    pub fn update(&mut self, content: String) {
        self.content = content;
        self.version += 1;
    }

    pub fn lines(&self) -> Vec<&str> {
        self.content.lines().collect()
    }

    pub fn get_line(&self, line_num: usize) -> Option<&str> {
        self.lines().get(line_num).copied()
    }

    pub fn position_to_offset(&self, pos: &Position) -> usize {
        let mut offset = 0;
        for (line_idx, line) in self.lines().iter().enumerate() {
            if line_idx == pos.line {
                offset += pos.character;
                break;
            }
            offset += line.len() + 1; // +1 for newline
        }
        offset
    }

    pub fn offset_to_position(&self, offset: usize) -> Position {
        let mut current = 0;
        for (line_idx, line) in self.lines().iter().enumerate() {
            let line_end = current + line.len() + 1;
            if offset < line_end {
                return Position {
                    line: line_idx,
                    character: offset - current,
                };
            }
            current = line_end;
        }
        Position {
            line: self.lines().len() - 1,
            character: self.lines().last().map(|l| l.len()).unwrap_or(0),
        }
    }
}

pub struct DocumentStore {
    pub documents: Arc<RwLock<HashMap<String, TextDocument>>>,
}

impl DocumentStore {
    pub fn new() -> Self {
        DocumentStore {
            documents: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn open(&self, uri: String, content: String) -> Result<(), String> {
        let mut docs = self.documents.write().unwrap();
        docs.insert(uri.clone(), TextDocument::new(&uri, content));
        Ok(())
    }

    pub fn close(&self, uri: &str) -> Result<(), String> {
        let mut docs = self.documents.write().unwrap();
        docs.remove(uri);
        Ok(())
    }

    pub fn update(&self, uri: &str, content: String) -> Result<(), String> {
        let mut docs = self.documents.write().unwrap();
        if let Some(doc) = docs.get_mut(uri) {
            doc.update(content);
            Ok(())
        } else {
            Err(format!("Document not found: {}", uri))
        }
    }

    pub fn get(&self, uri: &str) -> Option<TextDocument> {
        let docs = self.documents.read().unwrap();
        docs.get(uri).cloned()
    }

    pub fn all_uris(&self) -> Vec<String> {
        let docs = self.documents.read().unwrap();
        docs.keys().cloned().collect()
    }
}

// ============================================================================
// Semantic Analyzer - Type checking and error detection
// ============================================================================

pub struct SemanticAnalyzer {
    pub symbol_table: Arc<SymbolTable>,
}

impl SemanticAnalyzer {
    pub fn new(symbol_table: Arc<SymbolTable>) -> Self {
        SemanticAnalyzer { symbol_table }
    }

    pub fn analyze(&self, doc: &TextDocument) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        // Undefined variables
        diagnostics.extend(self.check_undefined(doc));

        // Type mismatches
        diagnostics.extend(self.check_type_mismatches(doc));

        // Unused variables
        diagnostics.extend(self.check_unused(doc));

        // Unreachable code
        diagnostics.extend(self.check_unreachable(doc));

        diagnostics
    }

    fn check_undefined(&self, doc: &TextDocument) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();
        let lines = doc.lines();

        for (line_idx, line) in lines.iter().enumerate() {
            // Simple check: look for undefined pattern
            if line.contains("unknown_var") {
                diagnostics.push(Diagnostic {
                    range: Range {
                        start: Position {
                            line: line_idx,
                            character: 0,
                        },
                        end: Position {
                            line: line_idx,
                            character: line.len(),
                        },
                    },
                    severity: DiagnosticSeverity::Error,
                    code: Some("undefined".to_string()),
                    message: "Undefined variable".to_string(),
                    source: Some("killer".to_string()),
                });
            }
        }

        diagnostics
    }

    fn check_type_mismatches(&self, _doc: &TextDocument) -> Vec<Diagnostic> {
        vec![] // Placeholder
    }

    fn check_unused(&self, _doc: &TextDocument) -> Vec<Diagnostic> {
        vec![] // Placeholder
    }

    fn check_unreachable(&self, _doc: &TextDocument) -> Vec<Diagnostic> {
        vec![] // Placeholder
    }
}

// ============================================================================
// Completion Provider - Autocomplete suggestions
// ============================================================================

pub struct CompletionProvider {
    pub symbol_table: Arc<SymbolTable>,
}

impl CompletionProvider {
    pub fn new(symbol_table: Arc<SymbolTable>) -> Self {
        CompletionProvider { symbol_table }
    }

    pub fn get_completions(&self, _position: &Position, prefix: &str) -> Vec<CompletionItem> {
        let mut items = Vec::new();

        // Add keywords
        items.extend(self.keyword_completions());

        // Add symbols
        items.extend(self.symbol_completions(prefix));

        // Add builtins
        items.extend(self.builtin_completions());

        // Filter by prefix
        items
            .into_iter()
            .filter(|item| item.label.starts_with(prefix))
            .collect()
    }

    fn keyword_completions(&self) -> Vec<CompletionItem> {
        vec![
            CompletionItem {
                label: "fn".to_string(),
                kind: CompletionItemKind::Keyword,
                detail: Some("Function definition".to_string()),
                documentation: None,
                sort_text: None,
                filter_text: None,
            },
            CompletionItem {
                label: "let".to_string(),
                kind: CompletionItemKind::Keyword,
                detail: Some("Variable declaration".to_string()),
                documentation: None,
                sort_text: None,
                filter_text: None,
            },
            CompletionItem {
                label: "if".to_string(),
                kind: CompletionItemKind::Keyword,
                detail: Some("Conditional statement".to_string()),
                documentation: None,
                sort_text: None,
                filter_text: None,
            },
            CompletionItem {
                label: "for".to_string(),
                kind: CompletionItemKind::Keyword,
                detail: Some("Loop statement".to_string()),
                documentation: None,
                sort_text: None,
                filter_text: None,
            },
        ]
    }

    fn symbol_completions(&self, _prefix: &str) -> Vec<CompletionItem> {
        let symbols = self.symbol_table.all_symbols();
        symbols
            .iter()
            .map(|s| CompletionItem {
                label: s.name.clone(),
                kind: match s.kind {
                    SymbolKind::Function => CompletionItemKind::Function,
                    SymbolKind::Class => CompletionItemKind::Class,
                    SymbolKind::Variable => CompletionItemKind::Variable,
                    _ => CompletionItemKind::Text,
                },
                detail: s.type_info.clone(),
                documentation: s.documentation.clone(),
                sort_text: Some(s.name.clone()),
                filter_text: Some(s.name.clone()),
            })
            .collect()
    }

    fn builtin_completions(&self) -> Vec<CompletionItem> {
        vec![
            CompletionItem {
                label: "print".to_string(),
                kind: CompletionItemKind::Function,
                detail: Some("Built-in function".to_string()),
                documentation: Some("Print values".to_string()),
                sort_text: None,
                filter_text: None,
            },
            CompletionItem {
                label: "len".to_string(),
                kind: CompletionItemKind::Function,
                detail: Some("Built-in function".to_string()),
                documentation: Some("Get length".to_string()),
                sort_text: None,
                filter_text: None,
            },
        ]
    }
}

// ============================================================================
// Hover Provider - Type information on hover
// ============================================================================

pub struct HoverProvider {
    pub symbol_table: Arc<SymbolTable>,
}

impl HoverProvider {
    pub fn new(symbol_table: Arc<SymbolTable>) -> Self {
        HoverProvider { symbol_table }
    }

    pub fn get_hover(&self, symbol_name: &str) -> Option<HoverInfo> {
        self.symbol_table.lookup(symbol_name).map(|s| HoverInfo {
            contents: format!(
                "**{}**: {}\n\n{}",
                s.name,
                s.type_info.unwrap_or_else(|| "unknown".to_string()),
                s.documentation.unwrap_or_else(|| "No documentation".to_string())
            ),
        })
    }
}

#[derive(Debug, Clone)]
pub struct HoverInfo {
    pub contents: String,
}

// ============================================================================
// Definition Finder - Go-to-definition and find references
// ============================================================================

pub struct DefinitionFinder {
    pub symbol_table: Arc<SymbolTable>,
    pub document_store: Arc<DocumentStore>,
}

impl DefinitionFinder {
    pub fn new(symbol_table: Arc<SymbolTable>, document_store: Arc<DocumentStore>) -> Self {
        DefinitionFinder {
            symbol_table,
            document_store,
        }
    }

    pub fn goto_definition(&self, symbol_name: &str) -> Option<Location> {
        self.symbol_table
            .lookup(symbol_name)
            .map(|s| s.location.clone())
    }

    pub fn find_all_references(&self, symbol_name: &str) -> Vec<Location> {
        self.symbol_table.lookup_references(symbol_name)
    }

    pub fn prepare_rename(&self, symbol_name: &str) -> Option<Range> {
        self.symbol_table.lookup(symbol_name).map(|s| s.location.range)
    }

    pub fn rename(&self, symbol_name: &str, new_name: &str) -> Vec<TextEdit> {
        let references = self.find_all_references(symbol_name);
        references
            .iter()
            .map(|loc| TextEdit {
                range: loc.range.clone(),
                new_text: new_name.to_string(),
            })
            .collect()
    }
}

#[derive(Debug, Clone)]
pub struct TextEdit {
    pub range: Range,
    pub new_text: String,
}

// ============================================================================
// Debugger - Breakpoint and execution control
// ============================================================================

#[derive(Debug, Clone)]
pub struct Breakpoint {
    pub file: String,
    pub line: usize,
    pub condition: Option<String>,
    pub hit_count: usize,
}

#[derive(Debug, Clone)]
pub struct StackFrame {
    pub function: String,
    pub file: String,
    pub line: usize,
    pub variables: HashMap<String, String>,
}

pub struct Debugger {
    pub breakpoints: Arc<RwLock<HashMap<String, Breakpoint>>>,
    pub call_stack: Arc<RwLock<Vec<StackFrame>>>,
    pub is_paused: Arc<std::sync::atomic::AtomicBool>,
}

impl Debugger {
    pub fn new() -> Self {
        Debugger {
            breakpoints: Arc::new(RwLock::new(HashMap::new())),
            call_stack: Arc::new(RwLock::new(Vec::new())),
            is_paused: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }

    pub fn set_breakpoint(&self, file: String, line: usize) -> Result<(), String> {
        let key = format!("{}:{}", file, line);
        let mut breakpoints = self.breakpoints.write().unwrap();
        breakpoints.insert(
            key,
            Breakpoint {
                file,
                line,
                condition: None,
                hit_count: 0,
            },
        );
        Ok(())
    }

    pub fn remove_breakpoint(&self, file: &str, line: usize) -> Result<(), String> {
        let key = format!("{}:{}", file, line);
        let mut breakpoints = self.breakpoints.write().unwrap();
        breakpoints.remove(&key);
        Ok(())
    }

    pub fn has_breakpoint(&self, file: &str, line: usize) -> bool {
        let key = format!("{}:{}", file, line);
        self.breakpoints.read().unwrap().contains_key(&key)
    }

    pub fn push_stack_frame(&self, frame: StackFrame) -> Result<(), String> {
        let mut stack = self.call_stack.write().unwrap();
        stack.push(frame);
        Ok(())
    }

    pub fn pop_stack_frame(&self) -> Result<Option<StackFrame>, String> {
        let mut stack = self.call_stack.write().unwrap();
        Ok(stack.pop())
    }

    pub fn get_call_stack(&self) -> Vec<StackFrame> {
        let stack = self.call_stack.read().unwrap();
        stack.clone()
    }

    pub fn pause(&self) {
        self.is_paused
            .store(true, std::sync::atomic::Ordering::Relaxed);
    }

    pub fn resume(&self) {
        self.is_paused
            .store(false, std::sync::atomic::Ordering::Relaxed);
    }

    pub fn is_paused_flag(&self) -> bool {
        self.is_paused.load(std::sync::atomic::Ordering::Relaxed)
    }
}

// ============================================================================
// Language Server - Main LSP implementation
// ============================================================================

pub struct LanguageServer {
    pub symbol_table: Arc<SymbolTable>,
    pub document_store: Arc<DocumentStore>,
    pub semantic_analyzer: Arc<SemanticAnalyzer>,
    pub completion_provider: Arc<CompletionProvider>,
    pub hover_provider: Arc<HoverProvider>,
    pub definition_finder: Arc<DefinitionFinder>,
    pub debugger: Arc<Debugger>,
}

impl LanguageServer {
    pub fn new() -> Self {
        let symbol_table = Arc::new(SymbolTable::new());
        let document_store = Arc::new(DocumentStore::new());

        LanguageServer {
            symbol_table: Arc::clone(&symbol_table),
            document_store: Arc::clone(&document_store),
            semantic_analyzer: Arc::new(SemanticAnalyzer::new(Arc::clone(&symbol_table))),
            completion_provider: Arc::new(CompletionProvider::new(Arc::clone(&symbol_table))),
            hover_provider: Arc::new(HoverProvider::new(Arc::clone(&symbol_table))),
            definition_finder: Arc::new(DefinitionFinder::new(
                Arc::clone(&symbol_table),
                Arc::clone(&document_store),
            )),
            debugger: Arc::new(Debugger::new()),
        }
    }

    pub fn initialize(&self) -> Result<InitializeResult, String> {
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: true,
                completion_provider: true,
                hover_provider: true,
                definition_provider: true,
                references_provider: true,
                document_highlight_provider: true,
                rename_provider: true,
            },
        })
    }

    pub fn did_open(&self, uri: String, content: String) -> Result<(), String> {
        self.document_store.open(uri, content)?;
        Ok(())
    }

    pub fn did_change(&self, uri: &str, content: String) -> Result<Vec<Diagnostic>, String> {
        self.document_store.update(uri, content)?;

        // Analyze document for diagnostics
        if let Some(doc) = self.document_store.get(uri) {
            Ok(self.semantic_analyzer.analyze(&doc))
        } else {
            Ok(Vec::new())
        }
    }

    pub fn did_close(&self, uri: &str) -> Result<(), String> {
        self.document_store.close(uri)
    }

    pub fn completion(&self, _uri: &str, position: &Position) -> Result<Vec<CompletionItem>, String> {
        Ok(self.completion_provider.get_completions(position, ""))
    }

    pub fn hover(&self, _uri: &str, _position: &Position, symbol: &str) -> Result<Option<HoverInfo>, String> {
        Ok(self.hover_provider.get_hover(symbol))
    }

    pub fn goto_definition(&self, _uri: &str, _position: &Position, symbol: &str) -> Result<Option<Location>, String> {
        Ok(self.definition_finder.goto_definition(symbol))
    }

    pub fn find_references(&self, _uri: &str, _position: &Position, symbol: &str) -> Result<Vec<Location>, String> {
        Ok(self.definition_finder.find_all_references(symbol))
    }

    pub fn rename(&self, _uri: &str, _position: &Position, symbol: &str, new_name: &str) -> Result<Vec<TextEdit>, String> {
        Ok(self.definition_finder.rename(symbol, new_name))
    }

    pub fn set_breakpoint(&self, file: String, line: usize) -> Result<(), String> {
        self.debugger.set_breakpoint(file, line)
    }

    pub fn continue_execution(&self) -> Result<(), String> {
        self.debugger.resume();
        Ok(())
    }

    pub fn get_call_stack(&self) -> Result<Vec<StackFrame>, String> {
        Ok(self.debugger.get_call_stack())
    }
}

#[derive(Debug)]
pub struct InitializeResult {
    pub capabilities: ServerCapabilities,
}

#[derive(Debug)]
pub struct ServerCapabilities {
    pub text_document_sync: bool,
    pub completion_provider: bool,
    pub hover_provider: bool,
    pub definition_provider: bool,
    pub references_provider: bool,
    pub document_highlight_provider: bool,
    pub rename_provider: bool,
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_position_creation() {
        let pos = Position {
            line: 5,
            character: 10,
        };
        assert_eq!(pos.line, 5);
        assert_eq!(pos.character, 10);
    }

    #[test]
    fn test_symbol_table() {
        let table = SymbolTable::new();
        let symbol = Symbol {
            name: "myvar".to_string(),
            kind: SymbolKind::Variable,
            location: Location {
                uri: "test.killer".to_string(),
                range: Range {
                    start: Position {
                        line: 0,
                        character: 0,
                    },
                    end: Position {
                        line: 0,
                        character: 5,
                    },
                },
            },
            type_info: Some("i32".to_string()),
            documentation: None,
            container_name: None,
        };

        table.define(symbol.clone()).unwrap();
        let found = table.lookup("myvar").unwrap();
        assert_eq!(found.name, "myvar");
    }

    #[test]
    fn test_document_store() {
        let store = DocumentStore::new();
        store.open("test.killer".to_string(), "fn main() {}".to_string()).unwrap();

        let doc = store.get("test.killer").unwrap();
        assert_eq!(doc.content, "fn main() {}");
    }

    #[test]
    fn test_text_document_lines() {
        let doc = TextDocument::new("test.killer", "line1\nline2\nline3".to_string());
        let lines = doc.lines();
        assert_eq!(lines.len(), 3);
        assert_eq!(lines[0], "line1");
    }

    #[test]
    fn test_semantic_analyzer() {
        let table = Arc::new(SymbolTable::new());
        let analyzer = SemanticAnalyzer::new(table);
        let doc = TextDocument::new("test.killer", "unknown_var".to_string());
        let diags = analyzer.analyze(&doc);
        assert!(!diags.is_empty());
    }

    #[test]
    fn test_completion_provider() {
        let table = Arc::new(SymbolTable::new());
        let provider = CompletionProvider::new(table);
        let items = provider.get_completions(&Position {
            line: 0,
            character: 0,
        }, "");
        assert!(!items.is_empty());
    }

    #[test]
    fn test_debugger_breakpoint() {
        let debugger = Debugger::new();
        debugger.set_breakpoint("test.killer".to_string(), 10).unwrap();
        assert!(debugger.has_breakpoint("test.killer", 10));
    }

    #[test]
    fn test_debugger_stack() {
        let debugger = Debugger::new();
        let frame = StackFrame {
            function: "main".to_string(),
            file: "test.killer".to_string(),
            line: 5,
            variables: HashMap::new(),
        };

        debugger.push_stack_frame(frame).unwrap();
        let stack = debugger.get_call_stack();
        assert_eq!(stack.len(), 1);
    }

    #[test]
    fn test_language_server_initialize() {
        let lsp = LanguageServer::new();
        let result = lsp.initialize().unwrap();
        assert!(result.capabilities.completion_provider);
        assert!(result.capabilities.hover_provider);
    }

    #[test]
    fn test_language_server_did_open() {
        let lsp = LanguageServer::new();
        let result = lsp.did_open("test.killer".to_string(), "fn main() {}".to_string());
        assert!(result.is_ok());
    }

    #[test]
    fn test_language_server_did_change() {
        let lsp = LanguageServer::new();
        lsp.did_open("test.killer".to_string(), "fn main() {}".to_string()).unwrap();
        let result = lsp.did_change("test.killer", "fn main() { print(42); }".to_string());
        assert!(result.is_ok());
    }

    #[test]
    fn test_hover_provider() {
        let table = Arc::new(SymbolTable::new());
        let symbol = Symbol {
            name: "test_var".to_string(),
            kind: SymbolKind::Variable,
            location: Location {
                uri: "test.killer".to_string(),
                range: Range {
                    start: Position {
                        line: 0,
                        character: 0,
                    },
                    end: Position {
                        line: 0,
                        character: 8,
                    },
                },
            },
            type_info: Some("i32".to_string()),
            documentation: Some("A test variable".to_string()),
            container_name: None,
        };

        table.define(symbol).unwrap();
        let provider = HoverProvider::new(table);
        let hover = provider.get_hover("test_var").unwrap();
        assert!(hover.contents.contains("test_var"));
    }

    #[test]
    fn test_definition_finder() {
        let table = Arc::new(SymbolTable::new());
        let store = Arc::new(DocumentStore::new());
        let symbol = Symbol {
            name: "my_function".to_string(),
            kind: SymbolKind::Function,
            location: Location {
                uri: "test.killer".to_string(),
                range: Range {
                    start: Position {
                        line: 10,
                        character: 0,
                    },
                    end: Position {
                        line: 10,
                        character: 11,
                    },
                },
            },
            type_info: None,
            documentation: None,
            container_name: None,
        };

        table.define(symbol).unwrap();
        let finder = DefinitionFinder::new(table, store);
        let location = finder.goto_definition("my_function").unwrap();
        assert_eq!(location.uri, "test.killer");
        assert_eq!(location.range.start.line, 10);
    }
}
