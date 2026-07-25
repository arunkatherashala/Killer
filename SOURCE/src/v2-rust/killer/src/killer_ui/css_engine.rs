//! **CSS Engine** — CSS-in-JS runtime with scoped class generation.
//!
//! Styled components, keyframe animations, media queries, pseudo-classes,
//! atomic CSS generation, and critical CSS extraction.

use std::collections::HashMap;

// ══════════════════════════════════════════════════════════════════════════════
// CSS Property
// ══════════════════════════════════════════════════════════════════════════════

/// A CSS property-value pair.
#[derive(Debug, Clone, PartialEq)]
pub struct CssProp {
    pub property: String,
    pub value: String,
}

impl CssProp {
    pub fn new(prop: &str, val: &str) -> Self {
        CssProp { property: prop.into(), value: val.into() }
    }

    pub fn to_css(&self) -> String {
        format!("{}: {};", self.property, self.value)
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// CSS Rule & Selectors
// ══════════════════════════════════════════════════════════════════════════════

/// Pseudo-class/pseudo-element.
#[derive(Debug, Clone, PartialEq)]
pub enum PseudoSelector {
    Hover,
    Focus,
    Active,
    Disabled,
    FirstChild,
    LastChild,
    NthChild(String),
    Before,
    After,
    Placeholder,
    FocusVisible,
    FocusWithin,
}

impl PseudoSelector {
    pub fn to_css(&self) -> &str {
        match self {
            PseudoSelector::Hover => ":hover",
            PseudoSelector::Focus => ":focus",
            PseudoSelector::Active => ":active",
            PseudoSelector::Disabled => ":disabled",
            PseudoSelector::FirstChild => ":first-child",
            PseudoSelector::LastChild => ":last-child",
            PseudoSelector::NthChild(_) => ":nth-child",
            PseudoSelector::Before => "::before",
            PseudoSelector::After => "::after",
            PseudoSelector::Placeholder => "::placeholder",
            PseudoSelector::FocusVisible => ":focus-visible",
            PseudoSelector::FocusWithin => ":focus-within",
        }
    }
}

/// Media query breakpoint.
#[derive(Debug, Clone, PartialEq)]
pub struct MediaQuery {
    pub query: String,
    pub rules: Vec<CssProp>,
}

impl MediaQuery {
    pub fn min_width(px: u32) -> Self {
        MediaQuery { query: format!("(min-width: {px}px)"), rules: Vec::new() }
    }
    pub fn max_width(px: u32) -> Self {
        MediaQuery { query: format!("(max-width: {px}px)"), rules: Vec::new() }
    }
    pub fn prefers_dark() -> Self {
        MediaQuery { query: "(prefers-color-scheme: dark)".into(), rules: Vec::new() }
    }
    pub fn with_rules(mut self, rules: Vec<CssProp>) -> Self {
        self.rules = rules; self
    }
}

/// A CSS keyframe animation.
#[derive(Debug, Clone)]
pub struct CssKeyframes {
    pub name: String,
    pub steps: Vec<(String, Vec<CssProp>)>,  // ("0%", props), ("100%", props)
}

impl CssKeyframes {
    pub fn new(name: &str) -> Self { CssKeyframes { name: name.into(), steps: Vec::new() } }
    pub fn step(mut self, pct: &str, props: Vec<CssProp>) -> Self {
        self.steps.push((pct.into(), props)); self
    }
    pub fn to_css(&self) -> String {
        let body: Vec<String> = self.steps.iter().map(|(pct, props)| {
            let decls: Vec<String> = props.iter().map(|p| format!("    {}", p.to_css())).collect();
            format!("  {} {{\n{}\n  }}", pct, decls.join("\n"))
        }).collect();
        format!("@keyframes {} {{\n{}\n}}", self.name, body.join("\n"))
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Styled Component
// ══════════════════════════════════════════════════════════════════════════════

/// A styled component definition — generates scoped CSS class.
#[derive(Debug, Clone)]
pub struct StyledComponent {
    pub tag: String,
    pub base_props: Vec<CssProp>,
    pub pseudo_rules: Vec<(PseudoSelector, Vec<CssProp>)>,
    pub media_queries: Vec<MediaQuery>,
    pub generated_class: String,
}

impl StyledComponent {
    pub fn new(tag: &str) -> Self {
        StyledComponent {
            tag: tag.into(),
            base_props: Vec::new(),
            pseudo_rules: Vec::new(),
            media_queries: Vec::new(),
            generated_class: String::new(),
        }
    }

    pub fn prop(mut self, property: &str, value: &str) -> Self {
        self.base_props.push(CssProp::new(property, value)); self
    }

    pub fn pseudo(mut self, selector: PseudoSelector, props: Vec<CssProp>) -> Self {
        self.pseudo_rules.push((selector, props)); self
    }

    pub fn media(mut self, mq: MediaQuery) -> Self {
        self.media_queries.push(mq); self
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// CSS Engine (stylesheet manager)
// ══════════════════════════════════════════════════════════════════════════════

/// CSS-in-JS engine: manages styled components, generates scoped classes, emits CSS.
pub struct CssEngine {
    components: Vec<StyledComponent>,
    keyframes: Vec<CssKeyframes>,
    global_rules: Vec<(String, Vec<CssProp>)>,
    css_variables: HashMap<String, String>,
    next_id: u64,
}

impl CssEngine {
    pub fn new() -> Self {
        CssEngine {
            components: Vec::new(),
            keyframes: Vec::new(),
            global_rules: Vec::new(),
            css_variables: HashMap::new(),
            next_id: 1,
        }
    }

    /// Register a styled component → returns the generated scoped class name.
    pub fn styled(&mut self, mut component: StyledComponent) -> String {
        let class = format!("k-{}", self.next_id);
        self.next_id += 1;
        component.generated_class = class.clone();
        self.components.push(component);
        class
    }

    /// Add a keyframe animation.
    pub fn add_keyframes(&mut self, kf: CssKeyframes) {
        self.keyframes.push(kf);
    }

    /// Add a global CSS rule.
    pub fn add_global(&mut self, selector: &str, props: Vec<CssProp>) {
        self.global_rules.push((selector.into(), props));
    }

    /// Set a CSS variable.
    pub fn set_variable(&mut self, name: &str, value: &str) {
        self.css_variables.insert(name.into(), value.into());
    }

    /// Generate the full CSS stylesheet.
    pub fn emit_css(&self) -> String {
        let mut out = String::new();

        // CSS variables on :root
        if !self.css_variables.is_empty() {
            out.push_str(":root {\n");
            for (name, val) in &self.css_variables {
                out.push_str(&format!("  --{name}: {val};\n"));
            }
            out.push_str("}\n\n");
        }

        // Global rules
        for (selector, props) in &self.global_rules {
            out.push_str(&format!("{selector} {{\n"));
            for p in props { out.push_str(&format!("  {}\n", p.to_css())); }
            out.push_str("}\n\n");
        }

        // Keyframes
        for kf in &self.keyframes {
            out.push_str(&kf.to_css());
            out.push_str("\n\n");
        }

        // Scoped component styles
        for comp in &self.components {
            let cls = &comp.generated_class;
            // Base rules
            out.push_str(&format!(".{cls} {{\n"));
            for p in &comp.base_props { out.push_str(&format!("  {}\n", p.to_css())); }
            out.push_str("}\n");

            // Pseudo rules
            for (pseudo, props) in &comp.pseudo_rules {
                out.push_str(&format!(".{cls}{} {{\n", pseudo.to_css()));
                for p in props { out.push_str(&format!("  {}\n", p.to_css())); }
                out.push_str("}\n");
            }

            // Media queries
            for mq in &comp.media_queries {
                out.push_str(&format!("@media {} {{\n  .{cls} {{\n", mq.query));
                for p in &mq.rules { out.push_str(&format!("    {}\n", p.to_css())); }
                out.push_str("  }\n}\n");
            }
            out.push('\n');
        }
        out
    }

    /// Extract critical CSS (only styles for visible component classes).
    pub fn extract_critical(&self, visible_classes: &[&str]) -> String {
        let mut out = String::new();
        for comp in &self.components {
            if visible_classes.contains(&comp.generated_class.as_str()) {
                out.push_str(&format!(".{} {{\n", comp.generated_class));
                for p in &comp.base_props { out.push_str(&format!("  {}\n", p.to_css())); }
                out.push_str("}\n");
            }
        }
        out
    }

    pub fn component_count(&self) -> usize { self.components.len() }
    pub fn keyframe_count(&self) -> usize { self.keyframes.len() }
}

impl Default for CssEngine {
    fn default() -> Self { Self::new() }
}

// ══════════════════════════════════════════════════════════════════════════════
// Tests
// ══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn styled_component_basic() {
        let mut engine = CssEngine::new();
        let cls = engine.styled(StyledComponent::new("button")
            .prop("background", "#007bff")
            .prop("color", "white")
            .prop("padding", "8px 16px"));
        assert_eq!(cls, "k-1");
        let css = engine.emit_css();
        assert!(css.contains(".k-1"));
        assert!(css.contains("background: #007bff;"));
    }

    #[test]
    fn pseudo_classes() {
        let mut engine = CssEngine::new();
        engine.styled(StyledComponent::new("button")
            .prop("color", "blue")
            .pseudo(PseudoSelector::Hover, vec![CssProp::new("color", "red")])
            .pseudo(PseudoSelector::FocusVisible, vec![CssProp::new("outline", "2px solid blue")]));
        let css = engine.emit_css();
        assert!(css.contains(":hover"));
        assert!(css.contains(":focus-visible"));
    }

    #[test]
    fn media_queries() {
        let mut engine = CssEngine::new();
        engine.styled(StyledComponent::new("div")
            .prop("font-size", "16px")
            .media(MediaQuery::min_width(768).with_rules(vec![CssProp::new("font-size", "20px")])));
        let css = engine.emit_css();
        assert!(css.contains("@media (min-width: 768px)"));
        assert!(css.contains("font-size: 20px;"));
    }

    #[test]
    fn css_variables() {
        let mut engine = CssEngine::new();
        engine.set_variable("primary", "#007bff");
        engine.set_variable("radius", "4px");
        let css = engine.emit_css();
        assert!(css.contains(":root {"));
        assert!(css.contains("--primary: #007bff;"));
    }

    #[test]
    fn keyframes() {
        let mut engine = CssEngine::new();
        engine.add_keyframes(CssKeyframes::new("fadeIn")
            .step("0%", vec![CssProp::new("opacity", "0")])
            .step("100%", vec![CssProp::new("opacity", "1")]));
        let css = engine.emit_css();
        assert!(css.contains("@keyframes fadeIn"));
        assert!(css.contains("opacity: 0;"));
    }

    #[test]
    fn global_rules() {
        let mut engine = CssEngine::new();
        engine.add_global("body", vec![CssProp::new("margin", "0"), CssProp::new("font-family", "sans-serif")]);
        let css = engine.emit_css();
        assert!(css.contains("body {"));
        assert!(css.contains("margin: 0;"));
    }

    #[test]
    fn critical_css_extraction() {
        let mut engine = CssEngine::new();
        let c1 = engine.styled(StyledComponent::new("h1").prop("color", "red"));
        let _c2 = engine.styled(StyledComponent::new("h2").prop("color", "blue"));
        let critical = engine.extract_critical(&[c1.as_str()]);
        assert!(critical.contains("color: red;"));
        assert!(!critical.contains("color: blue;"));
    }

    #[test]
    fn scoped_class_uniqueness() {
        let mut engine = CssEngine::new();
        let c1 = engine.styled(StyledComponent::new("a").prop("color", "red"));
        let c2 = engine.styled(StyledComponent::new("a").prop("color", "blue"));
        assert_ne!(c1, c2);
    }

    #[test]
    fn dark_mode_media() {
        let mut engine = CssEngine::new();
        engine.styled(StyledComponent::new("div")
            .prop("background", "white")
            .media(MediaQuery::prefers_dark().with_rules(vec![CssProp::new("background", "#1a1a1a")])));
        let css = engine.emit_css();
        assert!(css.contains("prefers-color-scheme: dark"));
    }
}
