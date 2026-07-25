// Phase 17: Documentation Generator - auto-doc generation, API docs, examples
// Features: Doc parsing, API documentation, example extraction, doc search

use std::collections::HashMap;

/// Documentation types
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum DocType {
    Module,
    Function,
    Struct,
    Enum,
    Trait,
    Type,
    Constant,
    Custom(String),
}

impl DocType {
    pub fn as_str(&self) -> &str {
        match self {
            DocType::Module => "module",
            DocType::Function => "function",
            DocType::Struct => "struct",
            DocType::Enum => "enum",
            DocType::Trait => "trait",
            DocType::Type => "type",
            DocType::Constant => "constant",
            DocType::Custom(name) => name,
        }
    }
}

/// Documentation block
#[derive(Clone, Debug)]
pub struct DocBlock {
    pub name: String,
    pub doc_type: DocType,
    pub description: String,
    pub summary: String,
    pub parameters: Vec<(String, String)>, // name, type/description
    pub return_type: Option<String>,
    pub examples: Vec<String>,
    pub tags: HashMap<String, String>,
}

impl DocBlock {
    pub fn new(name: String, doc_type: DocType) -> Self {
        DocBlock {
            name,
            doc_type,
            description: String::new(),
            summary: String::new(),
            parameters: Vec::new(),
            return_type: None,
            examples: Vec::new(),
            tags: HashMap::new(),
        }
    }

    /// Set description
    pub fn with_description(mut self, desc: String) -> Self {
        self.description = desc;
        self
    }

    /// Set summary
    pub fn with_summary(mut self, summary: String) -> Self {
        self.summary = summary;
        self
    }

    /// Add parameter
    pub fn add_parameter(mut self, name: String, type_desc: String) -> Self {
        self.parameters.push((name, type_desc));
        self
    }

    /// Set return type
    pub fn with_return_type(mut self, ret_type: String) -> Self {
        self.return_type = Some(ret_type);
        self
    }

    /// Add example
    pub fn add_example(mut self, example: String) -> Self {
        self.examples.push(example);
        self
    }

    /// Add tag
    pub fn add_tag(mut self, key: String, value: String) -> Self {
        self.tags.insert(key, value);
        self
    }

    /// Get parameter count
    pub fn param_count(&self) -> usize {
        self.parameters.len()
    }

    /// Get example count
    pub fn example_count(&self) -> usize {
        self.examples.len()
    }

    /// Check if documented
    pub fn is_documented(&self) -> bool {
        !self.description.is_empty() && !self.summary.is_empty()
    }
}

/// Module documentation
#[derive(Clone, Debug)]
pub struct ModuleDoc {
    pub name: String,
    pub path: String,
    pub description: String,
    pub items: Vec<DocBlock>,
    pub submodules: Vec<String>,
}

impl ModuleDoc {
    pub fn new(name: String, path: String) -> Self {
        ModuleDoc {
            name,
            path,
            description: String::new(),
            items: Vec::new(),
            submodules: Vec::new(),
        }
    }

    /// Set description
    pub fn with_description(mut self, desc: String) -> Self {
        self.description = desc;
        self
    }

    /// Add item
    pub fn add_item(&mut self, item: DocBlock) {
        self.items.push(item);
    }

    /// Add submodule
    pub fn add_submodule(&mut self, name: String) {
        self.submodules.push(name);
    }

    /// Get functions
    pub fn get_functions(&self) -> Vec<DocBlock> {
        self.items.iter()
            .filter(|i| i.doc_type == DocType::Function)
            .cloned()
            .collect()
    }

    /// Get structs
    pub fn get_structs(&self) -> Vec<DocBlock> {
        self.items.iter()
            .filter(|i| i.doc_type == DocType::Struct)
            .cloned()
            .collect()
    }

    /// Get traits
    pub fn get_traits(&self) -> Vec<DocBlock> {
        self.items.iter()
            .filter(|i| i.doc_type == DocType::Trait)
            .cloned()
            .collect()
    }

    /// Item count
    pub fn item_count(&self) -> usize {
        self.items.len()
    }

    /// Get documentation coverage
    pub fn coverage(&self) -> f32 {
        if self.items.is_empty() {
            0.0
        } else {
            let documented = self.items.iter().filter(|i| i.is_documented()).count();
            (documented as f32 / self.items.len() as f32) * 100.0
        }
    }
}

/// API documentation
#[derive(Clone, Debug)]
pub struct ApiDoc {
    pub name: String,
    pub version: String,
    pub modules: HashMap<String, ModuleDoc>,
    pub global_examples: Vec<String>,
}

impl ApiDoc {
    pub fn new(name: String, version: String) -> Self {
        ApiDoc {
            name,
            version,
            modules: HashMap::new(),
            global_examples: Vec::new(),
        }
    }

    /// Add module
    pub fn add_module(&mut self, module: ModuleDoc) -> Result<(), String> {
        if self.modules.contains_key(&module.name) {
            return Err(format!("Module {} already exists", module.name));
        }
        self.modules.insert(module.name.clone(), module);
        Ok(())
    }

    /// Get module
    pub fn get_module(&self, name: &str) -> Option<ModuleDoc> {
        self.modules.get(name).cloned()
    }

    /// Add global example
    pub fn add_example(&mut self, example: String) {
        self.global_examples.push(example);
    }

    /// List modules
    pub fn list_modules(&self) -> Vec<ModuleDoc> {
        self.modules.values().cloned().collect()
    }

    /// Module count
    pub fn module_count(&self) -> usize {
        self.modules.len()
    }

    /// Get total items
    pub fn total_items(&self) -> usize {
        self.modules.values().map(|m| m.item_count()).sum()
    }

    /// Get overall coverage
    pub fn overall_coverage(&self) -> f32 {
        if self.modules.is_empty() {
            0.0
        } else {
            let total_coverage: f32 = self.modules.values()
                .map(|m| m.coverage())
                .sum();
            total_coverage / self.modules.len() as f32
        }
    }

    /// Search items by name
    pub fn search(&self, query: &str) -> Vec<DocBlock> {
        self.modules.values()
            .flat_map(|m| m.items.iter().cloned())
            .filter(|item| item.name.contains(query))
            .collect()
    }

    /// Get all items of type
    pub fn get_items_by_type(&self, doc_type: &DocType) -> Vec<DocBlock> {
        self.modules.values()
            .flat_map(|m| m.items.iter().cloned())
            .filter(|item| item.doc_type == *doc_type)
            .collect()
    }
}

/// Example
#[derive(Clone, Debug)]
pub struct Example {
    pub name: String,
    pub description: String,
    pub code: String,
    pub tags: Vec<String>,
}

impl Example {
    pub fn new(name: String, code: String) -> Self {
        Example {
            name,
            description: String::new(),
            code,
            tags: Vec::new(),
        }
    }

    /// Set description
    pub fn with_description(mut self, desc: String) -> Self {
        self.description = desc;
        self
    }

    /// Add tag
    pub fn add_tag(mut self, tag: String) -> Self {
        self.tags.push(tag);
        self
    }
}

/// Example collection
#[derive(Clone, Debug)]
pub struct ExampleCollection {
    pub examples: HashMap<String, Example>,
    pub categories: HashMap<String, Vec<String>>, // category -> example names
}

impl ExampleCollection {
    pub fn new() -> Self {
        ExampleCollection {
            examples: HashMap::new(),
            categories: HashMap::new(),
        }
    }

    /// Add example
    pub fn add_example(&mut self, example: Example, category: String) -> Result<(), String> {
        if self.examples.contains_key(&example.name) {
            return Err(format!("Example {} already exists", example.name));
        }

        let name = example.name.clone();
        self.examples.insert(name.clone(), example);
        self.categories.entry(category)
            .or_insert_with(Vec::new)
            .push(name);

        Ok(())
    }

    /// Get example
    pub fn get_example(&self, name: &str) -> Option<Example> {
        self.examples.get(name).cloned()
    }

    /// Get examples by category
    pub fn get_by_category(&self, category: &str) -> Vec<Example> {
        self.categories.get(category)
            .iter()
            .flat_map(|names| {
                names.iter()
                    .filter_map(|name| self.examples.get(name).cloned())
            })
            .collect()
    }

    /// List categories
    pub fn list_categories(&self) -> Vec<String> {
        self.categories.keys().cloned().collect()
    }

    /// Example count
    pub fn example_count(&self) -> usize {
        self.examples.len()
    }

    /// Search examples
    pub fn search(&self, query: &str) -> Vec<Example> {
        self.examples.values()
            .filter(|ex| {
                ex.name.contains(query)
                    || ex.description.contains(query)
                    || ex.tags.iter().any(|tag| tag.contains(query))
            })
            .cloned()
            .collect()
    }
}

impl Default for ExampleCollection {
    fn default() -> Self {
        Self::new()
    }
}

/// Documentation generator
pub struct DocumentationGenerator {
    pub api_doc: ApiDoc,
    pub examples: ExampleCollection,
}

impl DocumentationGenerator {
    pub fn new(name: String, version: String) -> Self {
        DocumentationGenerator {
            api_doc: ApiDoc::new(name, version),
            examples: ExampleCollection::new(),
        }
    }

    /// Generate markdown for module
    pub fn generate_module_markdown(&self, module_name: &str) -> Option<String> {
        let module = self.api_doc.get_module(module_name)?;
        
        let mut md = String::new();
        md.push_str(&format!("# {}\n\n", module.name));
        md.push_str(&format!("{}\n\n", module.description));

        if !module.get_functions().is_empty() {
            md.push_str("## Functions\n\n");
            for func in module.get_functions() {
                md.push_str(&format!("### {}\n\n", func.name));
                md.push_str(&format!("{}\n\n", func.description));
            }
        }

        Some(md)
    }

    /// Generate API reference
    pub fn generate_api_reference(&self) -> String {
        let mut md = String::new();
        md.push_str(&format!("# API Reference - {}\n\n", self.api_doc.name));
        md.push_str(&format!("Version: {}\n\n", self.api_doc.version));

        for module in self.api_doc.list_modules() {
            md.push_str(&format!("## {}\n\n", module.name));
            md.push_str(&format!("{}\n\n", module.description));
            md.push_str(&format!("**Items:** {}\n\n", module.item_count()));
        }

        md
    }

    /// Get documentation statistics
    pub fn get_statistics(&self) -> DocStatistics {
        DocStatistics {
            total_modules: self.api_doc.module_count(),
            total_items: self.api_doc.total_items(),
            total_examples: self.examples.example_count(),
            coverage: self.api_doc.overall_coverage(),
        }
    }
}

/// Documentation statistics
#[derive(Clone, Debug)]
pub struct DocStatistics {
    pub total_modules: usize,
    pub total_items: usize,
    pub total_examples: usize,
    pub coverage: f32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_doc_type_as_str() {
        assert_eq!(DocType::Function.as_str(), "function");
        assert_eq!(DocType::Struct.as_str(), "struct");
    }

    #[test]
    fn test_doc_block_creation() {
        let doc = DocBlock::new("my_func".to_string(), DocType::Function);
        assert_eq!(doc.name, "my_func");
    }

    #[test]
    fn test_doc_block_with_description() {
        let doc = DocBlock::new("func".to_string(), DocType::Function)
            .with_description("Does something".to_string());
        assert_eq!(doc.description, "Does something");
    }

    #[test]
    fn test_doc_block_add_parameter() {
        let doc = DocBlock::new("func".to_string(), DocType::Function)
            .add_parameter("x".to_string(), "i32".to_string());
        assert_eq!(doc.param_count(), 1);
    }

    #[test]
    fn test_doc_block_with_return_type() {
        let doc = DocBlock::new("func".to_string(), DocType::Function)
            .with_return_type("i32".to_string());
        assert_eq!(doc.return_type, Some("i32".to_string()));
    }

    #[test]
    fn test_doc_block_add_example() {
        let doc = DocBlock::new("func".to_string(), DocType::Function)
            .add_example("let x = func();".to_string());
        assert_eq!(doc.example_count(), 1);
    }

    #[test]
    fn test_doc_block_is_documented() {
        let doc = DocBlock::new("func".to_string(), DocType::Function)
            .with_description("Does something".to_string())
            .with_summary("Summary".to_string());
        assert!(doc.is_documented());
    }

    #[test]
    fn test_module_doc_creation() {
        let module = ModuleDoc::new("math".to_string(), "lib::math".to_string());
        assert_eq!(module.name, "math");
    }

    #[test]
    fn test_module_doc_add_item() {
        let mut module = ModuleDoc::new("math".to_string(), "lib::math".to_string());
        module.add_item(DocBlock::new("add".to_string(), DocType::Function));
        assert_eq!(module.item_count(), 1);
    }

    #[test]
    fn test_module_doc_get_functions() {
        let mut module = ModuleDoc::new("math".to_string(), "lib::math".to_string());
        module.add_item(DocBlock::new("add".to_string(), DocType::Function));
        module.add_item(DocBlock::new("Point".to_string(), DocType::Struct));
        let functions = module.get_functions();
        assert_eq!(functions.len(), 1);
    }

    #[test]
    fn test_module_doc_coverage() {
        let mut module = ModuleDoc::new("math".to_string(), "lib::math".to_string());
        module.add_item(DocBlock::new("func1".to_string(), DocType::Function)
            .with_description("desc".to_string())
            .with_summary("summary".to_string()));
        module.add_item(DocBlock::new("func2".to_string(), DocType::Function));
        assert_eq!(module.coverage(), 50.0);
    }

    #[test]
    fn test_api_doc_creation() {
        let api = ApiDoc::new("mylib".to_string(), "1.0.0".to_string());
        assert_eq!(api.name, "mylib");
    }

    #[test]
    fn test_api_doc_add_module() {
        let mut api = ApiDoc::new("mylib".to_string(), "1.0.0".to_string());
        let module = ModuleDoc::new("math".to_string(), "lib::math".to_string());
        assert!(api.add_module(module).is_ok());
        assert_eq!(api.module_count(), 1);
    }

    #[test]
    fn test_api_doc_search() {
        let mut api = ApiDoc::new("mylib".to_string(), "1.0.0".to_string());
        let mut module = ModuleDoc::new("math".to_string(), "lib::math".to_string());
        module.add_item(DocBlock::new("add".to_string(), DocType::Function));
        module.add_item(DocBlock::new("subtract".to_string(), DocType::Function));
        api.add_module(module).unwrap();

        let results = api.search("add");
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_api_doc_get_items_by_type() {
        let mut api = ApiDoc::new("mylib".to_string(), "1.0.0".to_string());
        let mut module = ModuleDoc::new("math".to_string(), "lib::math".to_string());
        module.add_item(DocBlock::new("add".to_string(), DocType::Function));
        module.add_item(DocBlock::new("Point".to_string(), DocType::Struct));
        api.add_module(module).unwrap();

        let functions = api.get_items_by_type(&DocType::Function);
        assert_eq!(functions.len(), 1);
    }

    #[test]
    fn test_example_creation() {
        let example = Example::new("hello".to_string(), "println!(\"hello\");".to_string());
        assert_eq!(example.name, "hello");
    }

    #[test]
    fn test_example_with_description() {
        let example = Example::new("hello".to_string(), "code".to_string())
            .with_description("A hello example".to_string());
        assert_eq!(example.description, "A hello example");
    }

    #[test]
    fn test_example_add_tag() {
        let example = Example::new("hello".to_string(), "code".to_string())
            .add_tag("beginner".to_string());
        assert_eq!(example.tags.len(), 1);
    }

    #[test]
    fn test_example_collection_add() {
        let mut collection = ExampleCollection::new();
        let example = Example::new("hello".to_string(), "code".to_string());
        assert!(collection.add_example(example, "basics".to_string()).is_ok());
    }

    #[test]
    fn test_example_collection_get_by_category() {
        let mut collection = ExampleCollection::new();
        let example = Example::new("hello".to_string(), "code".to_string());
        collection.add_example(example, "basics".to_string()).unwrap();

        let examples = collection.get_by_category("basics");
        assert_eq!(examples.len(), 1);
    }

    #[test]
    fn test_example_collection_search() {
        let mut collection = ExampleCollection::new();
        let example = Example::new("hello".to_string(), "code".to_string())
            .with_description("A greeting".to_string());
        collection.add_example(example, "basics".to_string()).unwrap();

        let results = collection.search("greeting");
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_documentation_generator_creation() {
        let gen = DocumentationGenerator::new("mylib".to_string(), "1.0.0".to_string());
        assert_eq!(gen.api_doc.name, "mylib");
    }

    #[test]
    fn test_documentation_generator_statistics() {
        let gen = DocumentationGenerator::new("mylib".to_string(), "1.0.0".to_string());
        let stats = gen.get_statistics();
        assert_eq!(stats.total_modules, 0);
    }
}
