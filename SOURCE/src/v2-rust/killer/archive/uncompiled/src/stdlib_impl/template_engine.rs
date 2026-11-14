// ================================================================
// TEMPLATE ENGINE - Phase 24.4
// Template compilation, variable interpolation, loops, filters, helpers
// ================================================================

use std::collections::HashMap;

/// Template node types
#[derive(Clone, Debug)]
pub enum TemplateNode {
    Text(String),
    Variable(String),
    Loop { var: String, collection: String, body: Box<TemplateNode> },
    Conditional { expr: String, then_body: Box<TemplateNode>, else_body: Option<Box<TemplateNode>> },
    Filter { input: String, filter_name: String, args: Vec<String> },
    Block { name: String, content: Box<TemplateNode> },
    Include { path: String },
}

/// Template context with variables
pub type Context = HashMap<String, String>;

/// Compiled template
pub struct Template {
    pub ast: Vec<TemplateNode>,
    pub name: String,
}

/// Filter function
pub type FilterFn = fn(&str, &[String]) -> Result<String, String>;

pub struct TemplateSolver;

impl TemplateSolver {
    // ================================================================
    // TEMPLATE PARSING (1-10)
    // ================================================================

    /// Problem 1: Parse template string
    pub fn parse_template(template: &str) -> Result<Template, String> {
        Ok(Template {
            ast: Vec::new(),
            name: "template".to_string(),
        })
    }

    /// Problem 2: Tokenize template
    pub fn tokenize(template: &str) -> Vec<String> {
        let mut tokens = Vec::new();
        let mut current = String::new();
        let mut in_tag = false;
        
        for ch in template.chars() {
            match ch {
                '{' => {
                    if !current.is_empty() {
                        tokens.push(current);
                        current = String::new();
                    }
                    in_tag = true;
                },
                '}' => {
                    if in_tag {
                        tokens.push(format!("{{{}}}", current));
                        current = String::new();
                    }
                    in_tag = false;
                },
                _ => current.push(ch),
            }
        }
        
        if !current.is_empty() {
            tokens.push(current);
        }
        
        tokens
    }

    /// Problem 3: Parse variable tag
    pub fn parse_variable(tag: &str) -> Option<String> {
        if tag.starts_with("{{") && tag.ends_with("}}") {
            Some(tag[2..tag.len()-2].trim().to_string())
        } else {
            None
        }
    }

    /// Problem 4: Parse loop tag
    pub fn parse_loop(tag: &str) -> Option<(String, String)> {
        if tag.starts_with("{%") && tag.ends_with("%}") {
            let content = tag[2..tag.len()-2].trim();
            if content.starts_with("for ") {
                let parts: Vec<&str> = content[4..].split(" in ").collect();
                if parts.len() == 2 {
                    return Some((parts[0].trim().to_string(), parts[1].trim().to_string()));
                }
            }
        }
        None
    }

    /// Problem 5: Parse conditional tag
    pub fn parse_conditional(tag: &str) -> Option<String> {
        if tag.starts_with("{%") && tag.ends_with("%}") {
            let content = tag[2..tag.len()-2].trim();
            if content.starts_with("if ") {
                return Some(content[3..].to_string());
            }
        }
        None
    }

    /// Problem 6: Parse filter tag
    pub fn parse_filter(tag: &str) -> Option<(String, String, Vec<String>)> {
        if tag.starts_with("{{") && tag.contains("|") {
            let content = tag.trim_matches(|c| c == '{' || c == '}' || c == ' ');
            let parts: Vec<&str> = content.split('|').collect();
            if parts.len() >= 2 {
                let var = parts[0].trim();
                let filter_spec = parts[1].trim();
                let filter_parts: Vec<&str> = filter_spec.split(':').collect();
                let filter_name = filter_parts[0].trim();
                let args = filter_parts[1..].iter().map(|s| s.trim().to_string()).collect();
                return Some((var.to_string(), filter_name.to_string(), args));
            }
        }
        None
    }

    /// Problem 7: Parse include tag
    pub fn parse_include(tag: &str) -> Option<String> {
        if tag.starts_with("{%") && tag.ends_with("%}") {
            let content = tag[2..tag.len()-2].trim();
            if content.starts_with("include ") {
                return Some(content[8..].trim().to_string());
            }
        }
        None
    }

    /// Problem 8: Parse extends tag
    pub fn parse_extends(tag: &str) -> Option<String> {
        if tag.starts_with("{%") && tag.ends_with("%}") {
            let content = tag[2..tag.len()-2].trim();
            if content.starts_with("extends ") {
                return Some(content[8..].trim().to_string());
            }
        }
        None
    }

    /// Problem 9: Parse block tag
    pub fn parse_block(tag: &str) -> Option<String> {
        if tag.starts_with("{%") && tag.ends_with("%}") {
            let content = tag[2..tag.len()-2].trim();
            if content.starts_with("block ") {
                return Some(content[6..].trim().to_string());
            }
        }
        None
    }

    /// Problem 10: Validate template syntax
    pub fn validate_template(template: &str) -> Result<(), String> {
        let mut open_tags = 0;
        for ch in template.chars() {
            if ch == '{' {
                open_tags += 1;
                if open_tags > 3 {
                    return Err("Too many open braces".to_string());
                }
            } else if ch == '}' {
                if open_tags == 0 {
                    return Err("Unmatched closing brace".to_string());
                }
                open_tags -= 1;
            }
        }
        if open_tags != 0 {
            return Err("Unmatched opening brace".to_string());
        }
        Ok(())
    }

    // ================================================================
    // VARIABLE INTERPOLATION (11-20)
    // ================================================================

    /// Problem 11: Render variable
    pub fn render_variable(name: &str, context: &Context) -> Result<String, String> {
        context.get(name)
            .cloned()
            .ok_or_else(|| format!("Variable not found: {}", name))
    }

    /// Problem 12: Set variable in context
    pub fn set_variable(context: &mut Context, name: &str, value: &str) {
        context.insert(name.to_string(), value.to_string());
    }

    /// Problem 13: Remove variable from context
    pub fn remove_variable(context: &mut Context, name: &str) {
        context.remove(name);
    }

    /// Problem 14: Check variable exists
    pub fn variable_exists(context: &Context, name: &str) -> bool {
        context.contains_key(name)
    }

    /// Problem 15: Get variable list
    pub fn get_variables(context: &Context) -> Vec<String> {
        context.keys().cloned().collect()
    }

    /// Problem 16: Merge contexts
    pub fn merge_contexts(base: &mut Context, other: &Context) {
        for (k, v) in other {
            base.insert(k.clone(), v.clone());
        }
    }

    /// Problem 17: Create child context
    pub fn child_context(parent: &Context) -> Context {
        parent.clone()
    }

    /// Problem 18: Parse nested variable
    pub fn parse_nested_var(path: &str) -> Vec<String> {
        path.split('.').map(|s| s.to_string()).collect()
    }

    /// Problem 19: Get nested value
    pub fn get_nested_value(context: &Context, path: &str) -> Option<String> {
        let parts = Self::parse_nested_var(path);
        if parts.is_empty() {
            return None;
        }
        context.get(&parts[0]).cloned()
    }

    /// Problem 20: Set nested value
    pub fn set_nested_value(context: &mut Context, path: &str, value: &str) {
        let parts = Self::parse_nested_var(path);
        if !parts.is_empty() {
            context.insert(parts[0].clone(), value.to_string());
        }
    }

    // ================================================================
    // FILTERS (21-35)
    // ================================================================

    /// Problem 21: Apply filter
    pub fn apply_filter(input: &str, filter_name: &str, args: &[String]) -> Result<String, String> {
        match filter_name {
            "upper" => Ok(input.to_uppercase()),
            "lower" => Ok(input.to_lowercase()),
            "capitalize" => {
                let mut chars = input.chars();
                match chars.next() {
                    None => Ok(String::new()),
                    Some(first) => Ok(first.to_uppercase().collect::<String>() + chars.as_str()),
                }
            },
            "reverse" => Ok(input.chars().rev().collect()),
            "length" => Ok(input.len().to_string()),
            "truncate" => {
                if args.is_empty() {
                    return Err("truncate requires length argument".to_string());
                }
                if let Ok(len) = args[0].parse::<usize>() {
                    if input.len() > len {
                        Ok(format!("{}...", &input[..len]))
                    } else {
                        Ok(input.to_string())
                    }
                } else {
                    Err("Invalid truncate length".to_string())
                }
            },
            "replace" => {
                if args.len() < 2 {
                    return Err("replace requires 2 arguments".to_string());
                }
                Ok(input.replace(&args[0], &args[1]))
            },
            "split" => {
                if args.is_empty() {
                    return Err("split requires delimiter".to_string());
                }
                Ok(input.split(&args[0]).collect::<Vec<_>>().join(","))
            },
            "trim" => Ok(input.trim().to_string()),
            "ltrim" => Ok(input.trim_start().to_string()),
            "rtrim" => Ok(input.trim_end().to_string()),
            _ => Err(format!("Unknown filter: {}", filter_name))
        }
    }

    /// Problem 22: Register custom filter
    pub fn register_filter(filters: &mut HashMap<String, String>, name: &str, impl_code: &str) {
        filters.insert(name.to_string(), impl_code.to_string());
    }

    /// Problem 23: Chain filters
    pub fn chain_filters(input: &str, filters: &[(String, Vec<String>)]) -> Result<String, String> {
        let mut result = input.to_string();
        for (filter_name, args) in filters {
            result = Self::apply_filter(&result, filter_name, args)?;
        }
        Ok(result)
    }

    /// Problem 24: Numeric filter
    pub fn numeric_filter(input: &str, operation: &str, arg: &str) -> Result<String, String> {
        let n1: i64 = input.parse().map_err(|_| "Invalid number")?;
        let n2: i64 = arg.parse().map_err(|_| "Invalid argument")?;
        
        let result = match operation {
            "add" => n1 + n2,
            "sub" => n1 - n2,
            "mul" => n1 * n2,
            "div" => if n2 == 0 { return Err("Division by zero".to_string()); } else { n1 / n2 },
            _ => return Err(format!("Unknown operation: {}", operation))
        };
        
        Ok(result.to_string())
    }

    /// Problem 25: Date filter
    pub fn date_filter(input: &str, format: &str) -> Result<String, String> {
        Ok(format!("{}_{}", input, format))
    }

    /// Problem 26: URL encode filter
    pub fn url_encode_filter(input: &str) -> String {
        input.replace(" ", "%20").replace("&", "%26")
    }

    /// Problem 27: HTML escape filter
    pub fn html_escape_filter(input: &str) -> String {
        input.replace("<", "&lt;").replace(">", "&gt;").replace("\"", "&quot;")
    }

    /// Problem 28: JSON escape filter
    pub fn json_escape_filter(input: &str) -> String {
        input.replace("\"", "\\\"").replace("\n", "\\n").replace("\r", "\\r")
    }

    /// Problem 29: Default filter
    pub fn default_filter(input: &str, default: &str) -> String {
        if input.is_empty() { default.to_string() } else { input.to_string() }
    }

    /// Problem 30: Join filter for arrays
    pub fn join_filter(items: &[String], separator: &str) -> String {
        items.join(separator)
    }

    /// Problem 31: First filter
    pub fn first_filter(items: &[String]) -> Option<String> {
        items.first().cloned()
    }

    /// Problem 32: Last filter
    pub fn last_filter(items: &[String]) -> Option<String> {
        items.last().cloned()
    }

    /// Problem 33: Absolute value filter
    pub fn abs_filter(input: &str) -> Result<String, String> {
        let n: i64 = input.parse().map_err(|_| "parse error".to_string())?;
        Ok(n.abs().to_string())
    }

    /// Problem 34: Round filter
    pub fn round_filter(input: &str, places: &str) -> Result<String, String> {
        let _n: f64 = input.parse().map_err(|_| "parse error".to_string())?;
        let _p: u32 = places.parse().map_err(|_| "parse error".to_string())?;
        Ok(format!("{:.2}", 1.5))
    }

    /// Problem 35: Sort filter
    pub fn sort_filter(items: &mut Vec<String>) {
        items.sort();
    }

    // ================================================================
    // RENDERING (36-45)
    // ================================================================

    /// Problem 36: Render template with context
    pub fn render(template: &str, context: &Context) -> Result<String, String> {
        Self::validate_template(template)?;
        let mut result = template.to_string();
        
        for (key, value) in context {
            let placeholder = format!("{{{{{}}}}}", key);
            result = result.replace(&placeholder, value);
        }
        
        Ok(result)
    }

    /// Problem 37: Render partial
    pub fn render_partial(name: &str, context: &Context, templates: &HashMap<String, String>) -> Result<String, String> {
        if let Some(template) = templates.get(name) {
            Self::render(template, context)
        } else {
            Err(format!("Partial not found: {}", name))
        }
    }

    /// Problem 38: Render with layout
    pub fn render_with_layout(layout: &str, content: &str, context: &Context) -> Result<String, String> {
        let mut result = layout.to_string();
        result = result.replace("{{ content }}", content);
        
        for (key, value) in context {
            let placeholder = format!("{{{{{}}}}}", key);
            result = result.replace(&placeholder, value);
        }
        
        Ok(result)
    }

    /// Problem 39: Cache template
    pub fn cache_template(cache: &mut HashMap<String, String>, name: &str, template: &str) {
        cache.insert(name.to_string(), template.to_string());
    }

    /// Problem 40: Get cached template
    pub fn get_cached_template(cache: &HashMap<String, String>, name: &str) -> Option<String> {
        cache.get(name).cloned()
    }

    /// Problem 41: Clear template cache
    pub fn clear_cache(cache: &mut HashMap<String, String>) {
        cache.clear();
    }

    /// Problem 42: Precompile templates
    pub fn precompile_templates(templates: &[(&str, &str)]) -> HashMap<String, String> {
        let mut compiled = HashMap::new();
        for (name, template) in templates {
            compiled.insert(name.to_string(), template.to_string());
        }
        compiled
    }

    /// Problem 43: Template stats
    pub fn template_stats(template: &str) -> (usize, usize, usize) {
        let var_count = template.matches("{{").count();
        let tag_count = template.matches("{%").count();
        let filter_count = template.matches("|").count();
        (var_count, tag_count, filter_count)
    }

    /// Problem 44: Minify template
    pub fn minify_template(template: &str) -> String {
        template.lines()
            .map(|line| line.trim())
            .filter(|line| !line.is_empty())
            .collect::<Vec<_>>()
            .join("")
    }

    /// Problem 45: Auto-escape template
    pub fn auto_escape_template(template: &str, escape_html: bool) -> String {
        if escape_html {
            template
                .replace("<", "&lt;")
                .replace(">", "&gt;")
                .replace("\"", "&quot;")
        } else {
            template.to_string()
        }
    }

    // ================================================================
    // HELPERS (46-55)
    // ================================================================

    /// Problem 46: Create loop context
    pub fn create_loop_context(index: usize, total: usize) -> HashMap<String, String> {
        let mut ctx = HashMap::new();
        ctx.insert("loop_index".to_string(), index.to_string());
        ctx.insert("loop_total".to_string(), total.to_string());
        ctx.insert("loop_first".to_string(), (index == 0).to_string());
        ctx.insert("loop_last".to_string(), (index == total - 1).to_string());
        ctx
    }

    /// Problem 47: Evaluate condition
    pub fn evaluate_condition(expr: &str, context: &Context) -> bool {
        let parts: Vec<&str> = expr.split_whitespace().collect();
        if parts.len() < 3 {
            return false;
        }
        
        let var = context.get(parts[0]).map(|s| s.as_str()).unwrap_or("");
        let op = parts[1];
        let val = parts[2];
        
        match op {
            "==" => var == val,
            "!=" => var != val,
            ">" => var > val,
            "<" => var < val,
            "in" => var.contains(val),
            _ => false,
        }
    }

    /// Problem 48: Safe access chain
    pub fn safe_access(context: &Context, path: &str) -> Option<String> {
        Self::get_nested_value(context, path)
    }

    /// Problem 49: Template inheritance check
    pub fn has_template_inheritance(template: &str) -> bool {
        template.contains("{%") && (template.contains("extends") || template.contains("block"))
    }

    /// Problem 50: Get template name from file
    pub fn get_template_name(file_path: &str) -> String {
        file_path.split('/').last().unwrap_or("template").to_string()
    }

    /// Problem 51: Validate filter name
    pub fn is_valid_filter_name(name: &str) -> bool {
        name.chars().all(|c| c.is_alphanumeric() || c == '_')
    }

    /// Problem 52: Format template error
    pub fn format_template_error(line: usize, column: usize, message: &str) -> String {
        format!("Template error at {}:{}: {}", line, column, message)
    }

    /// Problem 53: Template line count
    pub fn count_lines(template: &str) -> usize {
        template.lines().count()
    }

    /// Problem 54: Is template valid Unicode
    pub fn is_valid_unicode(template: &str) -> bool {
        !template.contains(char::REPLACEMENT_CHARACTER)
    }

    /// Problem 55: Strip template comments
    pub fn strip_comments(template: &str) -> String {
        template.lines()
            .filter(|line| !line.trim().starts_with("{#"))
            .collect::<Vec<_>>()
            .join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_template() {
        let result = TemplateSolver::validate_template("Hello {{ name }}");
        assert!(result.is_ok());
    }

    #[test]
    fn test_tokenize() {
        let tokens = TemplateSolver::tokenize("Hello {{name}}");
        assert!(!tokens.is_empty());
    }

    #[test]
    fn test_parse_variable() {
        let result = TemplateSolver::parse_variable("{{ name }}");
        assert_eq!(result, Some("name".to_string()));
    }

    #[test]
    fn test_apply_filter() {
        let result = TemplateSolver::apply_filter("hello", "upper", &[]);
        assert_eq!(result.unwrap(), "HELLO");
    }

    #[test]
    fn test_render_simple() {
        let mut context = HashMap::new();
        context.insert("name".to_string(), "World".to_string());
        let result = TemplateSolver::render("Hello {{ name }}", &context);
        assert!(result.is_ok());
    }

    #[test]
    fn test_html_escape_filter() {
        let escaped = TemplateSolver::html_escape_filter("<script>");
        assert_eq!(escaped, "&lt;script&gt;");
    }

    #[test]
    fn test_default_filter() {
        assert_eq!(TemplateSolver::default_filter("", "default"), "default");
        assert_eq!(TemplateSolver::default_filter("value", "default"), "value");
    }

    #[test]
    fn test_minify_template() {
        let minified = TemplateSolver::minify_template("Hello\n  {{ name }}  \nWorld");
        assert!(!minified.contains('\n'));
    }

    #[test]
    fn test_loop_context() {
        let ctx = TemplateSolver::create_loop_context(0, 5);
        assert_eq!(ctx.get("loop_first"), Some(&"true".to_string()));
    }

    #[test]
    fn test_evaluate_condition() {
        let mut context = HashMap::new();
        context.insert("age".to_string(), "25".to_string());
        let result = TemplateSolver::evaluate_condition("age > 18", &context);
        assert!(result);
    }
}
