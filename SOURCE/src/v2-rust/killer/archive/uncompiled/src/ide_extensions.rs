// Phase 18: IDE Extensions & LSP - Language Server Protocol, code intelligence
// Features: LSP support, code completion, diagnostics, quick fixes, hover information

use std::collections::HashMap;

/// Language Server Protocol message types
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum LSPMessageType {
    Initialize,
    Initialized,
    Shutdown,
    Exit,
    DidOpen,
    DidChange,
    DidClose,
    DidSave,
    Hover,
    Completion,
    Definition,
    References,
    Rename,
    PublishDiagnostics,
    Custom(String),
}

impl LSPMessageType {
    pub fn as_str(&self) -> &str {
        match self {
            LSPMessageType::Initialize => "initialize",
            LSPMessageType::Initialized => "initialized",
            LSPMessageType::Shutdown => "shutdown",
            LSPMessageType::Exit => "exit",
            LSPMessageType::DidOpen => "textDocument/didOpen",
            LSPMessageType::DidChange => "textDocument/didChange",
            LSPMessageType::DidClose => "textDocument/didClose",
            LSPMessageType::DidSave => "textDocument/didSave",
            LSPMessageType::Hover => "textDocument/hover",
            LSPMessageType::Completion => "textDocument/completion",
            LSPMessageType::Definition => "textDocument/definition",
            LSPMessageType::References => "textDocument/references",
            LSPMessageType::Rename => "textDocument/rename",
            LSPMessageType::PublishDiagnostics => "textDocument/publishDiagnostics",
            LSPMessageType::Custom(name) => name,
        }
    }
}

/// Position in document
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Position {
    pub line: u32,
    pub character: u32,
}

impl Position {
    pub fn new(line: u32, character: u32) -> Self {
        Position { line, character }
    }
}

/// Range in document
#[derive(Clone, Debug)]
pub struct Range {
    pub start: Position,
    pub end: Position,
}

impl Range {
    pub fn new(start: Position, end: Position) -> Self {
        Range { start, end }
    }

    /// Check if position in range
    pub fn contains(&self, pos: &Position) -> bool {
        (pos.line > self.start.line || 
         (pos.line == self.start.line && pos.character >= self.start.character))
        && (pos.line < self.end.line ||
            (pos.line == self.end.line && pos.character <= self.end.character))
    }
}

/// Diagnostic severity
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DiagnosticSeverity {
    Error,
    Warning,
    Information,
    Hint,
}

impl DiagnosticSeverity {
    pub fn as_str(&self) -> &str {
        match self {
            DiagnosticSeverity::Error => "error",
            DiagnosticSeverity::Warning => "warning",
            DiagnosticSeverity::Information => "information",
            DiagnosticSeverity::Hint => "hint",
        }
    }
}

/// Diagnostic
#[derive(Clone, Debug)]
pub struct Diagnostic {
    pub range: Range,
    pub severity: DiagnosticSeverity,
    pub message: String,
    pub code: Option<String>,
    pub source: String,
}

impl Diagnostic {
    pub fn new(range: Range, severity: DiagnosticSeverity, message: String, source: String) -> Self {
        Diagnostic {
            range,
            severity,
            message,
            code: None,
            source,
        }
    }

    /// Set code
    pub fn with_code(mut self, code: String) -> Self {
        self.code = Some(code);
        self
    }
}

/// Completion item kind
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CompletionItemKind {
    Text,
    Method,
    Function,
    Constructor,
    Field,
    Variable,
    Class,
    Interface,
    Module,
    Property,
    Unit,
    Value,
    Enum,
    Keyword,
    Snippet,
    Color,
    File,
    Reference,
    Folder,
}

impl CompletionItemKind {
    pub fn as_str(&self) -> &str {
        match self {
            CompletionItemKind::Text => "text",
            CompletionItemKind::Method => "method",
            CompletionItemKind::Function => "function",
            CompletionItemKind::Constructor => "constructor",
            CompletionItemKind::Field => "field",
            CompletionItemKind::Variable => "variable",
            CompletionItemKind::Class => "class",
            CompletionItemKind::Interface => "interface",
            CompletionItemKind::Module => "module",
            CompletionItemKind::Property => "property",
            CompletionItemKind::Unit => "unit",
            CompletionItemKind::Value => "value",
            CompletionItemKind::Enum => "enum",
            CompletionItemKind::Keyword => "keyword",
            CompletionItemKind::Snippet => "snippet",
            CompletionItemKind::Color => "color",
            CompletionItemKind::File => "file",
            CompletionItemKind::Reference => "reference",
            CompletionItemKind::Folder => "folder",
        }
    }
}

/// Completion item
#[derive(Clone, Debug)]
pub struct CompletionItem {
    pub label: String,
    pub kind: CompletionItemKind,
    pub detail: Option<String>,
    pub documentation: Option<String>,
    pub insert_text: String,
    pub sort_text: Option<String>,
}

impl CompletionItem {
    pub fn new(label: String, kind: CompletionItemKind) -> Self {
        let insert_text = label.clone();
        CompletionItem {
            label,
            kind,
            detail: None,
            documentation: None,
            insert_text,
            sort_text: None,
        }
    }

    /// Set detail
    pub fn with_detail(mut self, detail: String) -> Self {
        self.detail = Some(detail);
        self
    }

    /// Set documentation
    pub fn with_documentation(mut self, doc: String) -> Self {
        self.documentation = Some(doc);
        self
    }

    /// Set insert text
    pub fn with_insert_text(mut self, text: String) -> Self {
        self.insert_text = text;
        self
    }

    /// Set sort text
    pub fn with_sort_text(mut self, text: String) -> Self {
        self.sort_text = Some(text);
        self
    }
}

/// Text document
#[derive(Clone, Debug)]
pub struct TextDocument {
    pub uri: String,
    pub language_id: String,
    pub version: u32,
    pub text: String,
    pub dirty: bool,
}

impl TextDocument {
    pub fn new(uri: String, language_id: String) -> Self {
        TextDocument {
            uri,
            language_id,
            version: 1,
            text: String::new(),
            dirty: false,
        }
    }

    /// Update content
    pub fn update(mut self, text: String) -> Self {
        self.text = text;
        self.version += 1;
        self.dirty = true;
        self
    }

    /// Get line
    pub fn get_line(&self, line: u32) -> Option<String> {
        self.text.lines().nth(line as usize).map(|l| l.to_string())
    }

    /// Get line count
    pub fn line_count(&self) -> u32 {
        self.text.lines().count() as u32
    }

    /// Get character at position
    pub fn get_char_at(&self, pos: &Position) -> Option<char> {
        self.get_line(pos.line)
            .and_then(|line| line.chars().nth(pos.character as usize))
    }

    /// Mark saved
    pub fn mark_saved(mut self) -> Self {
        self.dirty = false;
        self
    }
}

/// Hover information
#[derive(Clone, Debug)]
pub struct Hover {
    pub contents: String,
    pub range: Option<Range>,
}

impl Hover {
    pub fn new(contents: String) -> Self {
        Hover {
            contents,
            range: None,
        }
    }

    /// Set range
    pub fn with_range(mut self, range: Range) -> Self {
        self.range = Some(range);
        self
    }
}

/// Location
#[derive(Clone, Debug)]
pub struct Location {
    pub uri: String,
    pub range: Range,
}

impl Location {
    pub fn new(uri: String, range: Range) -> Self {
        Location { uri, range }
    }
}

/// Symbol kind
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SymbolKind {
    File,
    Module,
    Namespace,
    Package,
    Class,
    Method,
    Property,
    Field,
    Constructor,
    Enum,
    Interface,
    Function,
    Variable,
    Constant,
    String,
    Number,
    Boolean,
    Array,
    Object,
    Key,
    Null,
    EnumMember,
    Struct,
    Event,
    Operator,
    TypeParameter,
}

impl SymbolKind {
    pub fn as_str(&self) -> &str {
        match self {
            SymbolKind::File => "File",
            SymbolKind::Module => "Module",
            SymbolKind::Namespace => "Namespace",
            SymbolKind::Package => "Package",
            SymbolKind::Class => "Class",
            SymbolKind::Method => "Method",
            SymbolKind::Property => "Property",
            SymbolKind::Field => "Field",
            SymbolKind::Constructor => "Constructor",
            SymbolKind::Enum => "Enum",
            SymbolKind::Interface => "Interface",
            SymbolKind::Function => "Function",
            SymbolKind::Variable => "Variable",
            SymbolKind::Constant => "Constant",
            SymbolKind::String => "String",
            SymbolKind::Number => "Number",
            SymbolKind::Boolean => "Boolean",
            SymbolKind::Array => "Array",
            SymbolKind::Object => "Object",
            SymbolKind::Key => "Key",
            SymbolKind::Null => "Null",
            SymbolKind::EnumMember => "EnumMember",
            SymbolKind::Struct => "Struct",
            SymbolKind::Event => "Event",
            SymbolKind::Operator => "Operator",
            SymbolKind::TypeParameter => "TypeParameter",
        }
    }
}

/// Document symbol
#[derive(Clone, Debug)]
pub struct DocumentSymbol {
    pub name: String,
    pub kind: SymbolKind,
    pub range: Range,
    pub selection_range: Range,
    pub children: Vec<DocumentSymbol>,
}

impl DocumentSymbol {
    pub fn new(name: String, kind: SymbolKind, range: Range, selection_range: Range) -> Self {
        DocumentSymbol {
            name,
            kind,
            range,
            selection_range,
            children: Vec::new(),
        }
    }

    /// Add child
    pub fn add_child(mut self, child: DocumentSymbol) -> Self {
        self.children.push(child);
        self
    }
}

/// Language server
#[derive(Clone, Debug)]
pub struct LanguageServer {
    pub documents: HashMap<String, TextDocument>,
    pub diagnostics: HashMap<String, Vec<Diagnostic>>,
    pub initialized: bool,
}

impl LanguageServer {
    pub fn new() -> Self {
        LanguageServer {
            documents: HashMap::new(),
            diagnostics: HashMap::new(),
            initialized: false,
        }
    }

    /// Initialize
    pub fn initialize(mut self) -> Self {
        self.initialized = true;
        self
    }

    /// Open document
    pub fn open_document(&mut self, uri: String, language_id: String) {
        let doc = TextDocument::new(uri.clone(), language_id);
        self.documents.insert(uri, doc);
    }

    /// Close document
    pub fn close_document(&mut self, uri: &str) -> Option<TextDocument> {
        self.documents.remove(uri)
    }

    /// Get document
    pub fn get_document(&self, uri: &str) -> Option<TextDocument> {
        self.documents.get(uri).cloned()
    }

    /// Update document
    pub fn update_document(&mut self, uri: &str, text: String) -> Result<(), String> {
        let doc = self.documents.get_mut(uri)
            .ok_or_else(|| format!("Document {} not found", uri))?;
        *doc = doc.clone().update(text);
        Ok(())
    }

    /// Publish diagnostics
    pub fn publish_diagnostics(&mut self, uri: String, diagnostics: Vec<Diagnostic>) {
        self.diagnostics.insert(uri, diagnostics);
    }

    /// Get diagnostics
    pub fn get_diagnostics(&self, uri: &str) -> Vec<Diagnostic> {
        self.diagnostics.get(uri).cloned().unwrap_or_default()
    }

    /// Clear diagnostics
    pub fn clear_diagnostics(&mut self, uri: &str) {
        self.diagnostics.remove(uri);
    }

    /// Document count
    pub fn document_count(&self) -> usize {
        self.documents.len()
    }

    /// Get diagnostic count
    pub fn diagnostic_count(&self, uri: &str) -> usize {
        self.get_diagnostics(uri).len()
    }
}

impl Default for LanguageServer {
    fn default() -> Self {
        Self::new()
    }
}

/// Quick fix
#[derive(Clone, Debug)]
pub struct QuickFix {
    pub title: String,
    pub description: String,
    pub range: Range,
    pub new_text: String,
}

impl QuickFix {
    pub fn new(title: String, range: Range, new_text: String) -> Self {
        QuickFix {
            title,
            description: String::new(),
            range,
            new_text,
        }
    }

    /// Set description
    pub fn with_description(mut self, desc: String) -> Self {
        self.description = desc;
        self
    }
}

/// Code intelligence provider
pub struct CodeIntelligenceProvider {
    pub language_server: LanguageServer,
    pub quick_fixes: HashMap<String, Vec<QuickFix>>,
}

impl CodeIntelligenceProvider {
    pub fn new() -> Self {
        CodeIntelligenceProvider {
            language_server: LanguageServer::new(),
            quick_fixes: HashMap::new(),
        }
    }

    /// Register quick fix
    pub fn register_quick_fix(&mut self, uri: String, fix: QuickFix) {
        self.quick_fixes.entry(uri)
            .or_insert_with(Vec::new)
            .push(fix);
    }

    /// Get quick fixes
    pub fn get_quick_fixes(&self, uri: &str) -> Vec<QuickFix> {
        self.quick_fixes.get(uri).cloned().unwrap_or_default()
    }

    /// Find symbol at position
    pub fn find_symbol_at(&self, uri: &str, pos: &Position) -> Option<String> {
        let doc = self.language_server.get_document(uri)?;
        let line = doc.get_line(pos.line)?;
        
        // Simple word extraction
        let mut word = String::new();
        let chars: Vec<char> = line.chars().collect();
        let mut idx = pos.character as usize;
        
        // Find start of word
        while idx > 0 && chars[idx - 1].is_alphanumeric() {
            idx -= 1;
        }
        
        // Extract word
        while idx < chars.len() && (chars[idx].is_alphanumeric() || chars[idx] == '_') {
            word.push(chars[idx]);
            idx += 1;
        }
        
        if word.is_empty() { None } else { Some(word) }
    }
}

impl Default for CodeIntelligenceProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lsp_message_type_as_str() {
        assert_eq!(LSPMessageType::Initialize.as_str(), "initialize");
        assert_eq!(LSPMessageType::Hover.as_str(), "textDocument/hover");
    }

    #[test]
    fn test_position_creation() {
        let pos = Position::new(5, 10);
        assert_eq!(pos.line, 5);
        assert_eq!(pos.character, 10);
    }

    #[test]
    fn test_range_creation() {
        let start = Position::new(0, 0);
        let end = Position::new(1, 10);
        let range = Range::new(start, end);
        assert_eq!(range.start.line, 0);
    }

    #[test]
    fn test_range_contains() {
        let start = Position::new(0, 0);
        let end = Position::new(2, 10);
        let range = Range::new(start, end);
        
        let pos = Position::new(1, 5);
        assert!(range.contains(&pos));
        
        let outside = Position::new(3, 0);
        assert!(!range.contains(&outside));
    }

    #[test]
    fn test_diagnostic_severity_as_str() {
        assert_eq!(DiagnosticSeverity::Error.as_str(), "error");
        assert_eq!(DiagnosticSeverity::Warning.as_str(), "warning");
    }

    #[test]
    fn test_diagnostic_creation() {
        let range = Range::new(Position::new(0, 0), Position::new(0, 5));
        let diag = Diagnostic::new(
            range,
            DiagnosticSeverity::Error,
            "Syntax error".to_string(),
            "parser".to_string(),
        );
        assert_eq!(diag.message, "Syntax error");
    }

    #[test]
    fn test_diagnostic_with_code() {
        let range = Range::new(Position::new(0, 0), Position::new(0, 5));
        let diag = Diagnostic::new(
            range,
            DiagnosticSeverity::Error,
            "Error".to_string(),
            "parser".to_string(),
        ).with_code("E0001".to_string());
        assert_eq!(diag.code, Some("E0001".to_string()));
    }

    #[test]
    fn test_completion_item_kind_as_str() {
        assert_eq!(CompletionItemKind::Function.as_str(), "function");
        assert_eq!(CompletionItemKind::Class.as_str(), "class");
    }

    #[test]
    fn test_completion_item_creation() {
        let item = CompletionItem::new("println".to_string(), CompletionItemKind::Function);
        assert_eq!(item.label, "println");
    }

    #[test]
    fn test_completion_item_with_detail() {
        let item = CompletionItem::new("println".to_string(), CompletionItemKind::Function)
            .with_detail("fn(args: ...)".to_string());
        assert_eq!(item.detail, Some("fn(args: ...)".to_string()));
    }

    #[test]
    fn test_text_document_creation() {
        let doc = TextDocument::new("file:///main.rs".to_string(), "rust".to_string());
        assert_eq!(doc.uri, "file:///main.rs");
        assert_eq!(doc.version, 1);
    }

    #[test]
    fn test_text_document_update() {
        let doc = TextDocument::new("file:///main.rs".to_string(), "rust".to_string());
        let updated = doc.update("fn main() {}".to_string());
        assert_eq!(updated.version, 2);
        assert!(updated.dirty);
    }

    #[test]
    fn test_text_document_line_count() {
        let doc = TextDocument::new("file:///main.rs".to_string(), "rust".to_string());
        let updated = doc.update("line1\nline2\nline3".to_string());
        assert_eq!(updated.line_count(), 3);
    }

    #[test]
    fn test_text_document_get_line() {
        let doc = TextDocument::new("file:///main.rs".to_string(), "rust".to_string());
        let updated = doc.update("line1\nline2\nline3".to_string());
        assert_eq!(updated.get_line(1), Some("line2".to_string()));
    }

    #[test]
    fn test_hover_creation() {
        let hover = Hover::new("Documentation".to_string());
        assert_eq!(hover.contents, "Documentation");
    }

    #[test]
    fn test_hover_with_range() {
        let range = Range::new(Position::new(0, 0), Position::new(0, 5));
        let hover = Hover::new("Doc".to_string()).with_range(range.clone());
        assert_eq!(hover.range.unwrap().start.line, 0);
    }

    #[test]
    fn test_location_creation() {
        let range = Range::new(Position::new(5, 10), Position::new(5, 15));
        let loc = Location::new("file:///main.rs".to_string(), range);
        assert_eq!(loc.uri, "file:///main.rs");
    }

    #[test]
    fn test_symbol_kind_as_str() {
        assert_eq!(SymbolKind::Function.as_str(), "Function");
        assert_eq!(SymbolKind::Class.as_str(), "Class");
    }

    #[test]
    fn test_document_symbol_creation() {
        let range = Range::new(Position::new(0, 0), Position::new(5, 0));
        let symbol = DocumentSymbol::new(
            "MyClass".to_string(),
            SymbolKind::Class,
            range.clone(),
            range,
        );
        assert_eq!(symbol.name, "MyClass");
    }

    #[test]
    fn test_document_symbol_add_child() {
        let range = Range::new(Position::new(0, 0), Position::new(5, 0));
        let mut symbol = DocumentSymbol::new(
            "MyClass".to_string(),
            SymbolKind::Class,
            range.clone(),
            range.clone(),
        );
        let method = DocumentSymbol::new(
            "method".to_string(),
            SymbolKind::Method,
            range.clone(),
            range,
        );
        symbol = symbol.add_child(method);
        assert_eq!(symbol.children.len(), 1);
    }

    #[test]
    fn test_language_server_creation() {
        let server = LanguageServer::new();
        assert!(!server.initialized);
    }

    #[test]
    fn test_language_server_initialize() {
        let server = LanguageServer::new();
        let initialized = server.initialize();
        assert!(initialized.initialized);
    }

    #[test]
    fn test_language_server_open_document() {
        let mut server = LanguageServer::new();
        server.open_document("file:///main.rs".to_string(), "rust".to_string());
        assert_eq!(server.document_count(), 1);
    }

    #[test]
    fn test_language_server_close_document() {
        let mut server = LanguageServer::new();
        server.open_document("file:///main.rs".to_string(), "rust".to_string());
        assert!(server.close_document("file:///main.rs").is_some());
        assert_eq!(server.document_count(), 0);
    }

    #[test]
    fn test_language_server_update_document() {
        let mut server = LanguageServer::new();
        server.open_document("file:///main.rs".to_string(), "rust".to_string());
        assert!(server.update_document("file:///main.rs", "new content".to_string()).is_ok());
    }

    #[test]
    fn test_language_server_publish_diagnostics() {
        let mut server = LanguageServer::new();
        server.open_document("file:///main.rs".to_string(), "rust".to_string());
        
        let range = Range::new(Position::new(0, 0), Position::new(0, 5));
        let diag = Diagnostic::new(
            range,
            DiagnosticSeverity::Error,
            "Error".to_string(),
            "parser".to_string(),
        );
        
        server.publish_diagnostics("file:///main.rs".to_string(), vec![diag]);
        assert_eq!(server.diagnostic_count("file:///main.rs"), 1);
    }

    #[test]
    fn test_quick_fix_creation() {
        let range = Range::new(Position::new(0, 0), Position::new(0, 5));
        let fix = QuickFix::new("Fix".to_string(), range, "fixed".to_string());
        assert_eq!(fix.title, "Fix");
    }

    #[test]
    fn test_code_intelligence_provider_creation() {
        let provider = CodeIntelligenceProvider::new();
        assert!(!provider.language_server.initialized);
    }

    #[test]
    fn test_code_intelligence_provider_register_quick_fix() {
        let mut provider = CodeIntelligenceProvider::new();
        let range = Range::new(Position::new(0, 0), Position::new(0, 5));
        let fix = QuickFix::new("Fix".to_string(), range, "fixed".to_string());
        provider.register_quick_fix("file:///main.rs".to_string(), fix);
        assert_eq!(provider.get_quick_fixes("file:///main.rs").len(), 1);
    }
}
