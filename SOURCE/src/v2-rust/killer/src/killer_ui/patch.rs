//! **Phase A** — declarative UI patch: windows, widgets, events (no GPU).
//!
//! 30+ widget types covering forms, data display, navigation, layout, feedback,
//! and media — competitive with Angular Material / React MUI / Ant Design widget sets.

use std::collections::HashMap;

/// Single UI event from widgets (IDs match `WidgetId` keys in patch).
#[derive(Debug, Clone, PartialEq)]
pub enum UiEvent {
    ButtonClicked(String),
    SliderChanged { id: String, value: f64 },
    ToggleChanged { id: String, on: bool },
    InputChanged { id: String, value: String },
    SelectChanged { id: String, value: String },
    CheckboxChanged { id: String, checked: bool },
    RadioSelected { id: String, value: String },
    TabChanged { id: String, index: usize },
    AccordionToggled { id: String, index: usize, open: bool },
    FormSubmitted { id: String },
    MenuItemClicked { id: String, item: String },
    TreeNodeToggled { id: String, node_id: String, expanded: bool },
    DateSelected { id: String, value: String },
    ColorSelected { id: String, value: String },
    DialogClosed { id: String, result: String },
    PaginationChanged { id: String, page: usize },
}

pub type WidgetId = String;

/// 30+ widget types for full framework parity.
#[derive(Debug, Clone)]
pub enum Widget {
    // ── Basic ────────────────────────────────────────────────────────────
    /// Static text label.
    Label { id: WidgetId, text: String },
    /// Clickable button.
    Button { id: WidgetId, label: String, variant: ButtonVariant, disabled: bool },
    /// Continuous slider.
    Slider { id: WidgetId, label: String, min: f64, max: f64, value: f64 },
    /// Boolean toggle switch.
    Toggle { id: WidgetId, label: String, on: bool },
    /// Icon (name-based, resolved by renderer).
    Icon { id: WidgetId, name: String, size: f64 },

    // ── Form inputs ──────────────────────────────────────────────────────
    /// Single-line text input.
    TextInput { id: WidgetId, label: String, value: String, placeholder: String, input_type: InputType },
    /// Multi-line text area.
    TextArea { id: WidgetId, label: String, value: String, rows: u32 },
    /// Dropdown select.
    Select { id: WidgetId, label: String, options: Vec<SelectOption>, selected: Option<String> },
    /// Checkbox.
    Checkbox { id: WidgetId, label: String, checked: bool },
    /// Radio button group.
    RadioGroup { id: WidgetId, label: String, options: Vec<String>, selected: Option<String> },
    /// Date picker.
    DatePicker { id: WidgetId, label: String, value: String },
    /// Color picker.
    ColorPicker { id: WidgetId, label: String, value: String },
    /// File upload area.
    FileUpload { id: WidgetId, label: String, accept: String, multiple: bool },
    /// Form container (groups fields, handles submit).
    Form { id: WidgetId, children: Vec<Widget> },

    // ── Data display ─────────────────────────────────────────────────────
    /// Data table with headers and rows.
    Table { id: WidgetId, headers: Vec<String>, rows: Vec<Vec<String>>, sortable: bool },
    /// Simple list of items.
    List { id: WidgetId, items: Vec<String>, ordered: bool },
    /// Key-value badge/chip.
    Badge { id: WidgetId, text: String, color: String },
    /// Circular or linear progress indicator.
    ProgressBar { id: WidgetId, value: f64, max: f64, variant: ProgressVariant },
    /// Loading spinner.
    Spinner { id: WidgetId, size: f64 },
    /// Avatar (initials or image placeholder).
    Avatar { id: WidgetId, text: String, src: Option<String>, size: f64 },
    /// Tooltip (wraps another widget).
    Tooltip { id: WidgetId, text: String, child: Box<Widget> },
    /// Tree view with nested nodes.
    TreeView { id: WidgetId, nodes: Vec<TreeNode> },
    /// Pagination control.
    Pagination { id: WidgetId, total_pages: usize, current_page: usize },

    // ── Navigation ───────────────────────────────────────────────────────
    /// Tab strip with tab labels.
    Tabs { id: WidgetId, labels: Vec<String>, active: usize, children: Vec<Widget> },
    /// Accordion (collapsible sections).
    Accordion { id: WidgetId, sections: Vec<AccordionSection> },
    /// Breadcrumb trail.
    Breadcrumb { id: WidgetId, items: Vec<BreadcrumbItem> },
    /// Menu (vertical list of actions).
    Menu { id: WidgetId, items: Vec<MenuItem> },
    /// Navigation sidebar with links.
    NavSidebar { id: WidgetId, items: Vec<NavItem>, active: Option<String> },

    // ── Layout containers ────────────────────────────────────────────────
    /// Vertical stack.
    Column { id: WidgetId, children: Vec<Widget> },
    /// Horizontal stack.
    Row { id: WidgetId, children: Vec<Widget>, gap: f64 },
    /// Grid layout.
    Grid { id: WidgetId, children: Vec<Widget>, columns: u32, gap: f64 },
    /// Card container with optional header/footer.
    Card { id: WidgetId, title: Option<String>, children: Vec<Widget>, elevated: bool },
    /// Divider line (horizontal or vertical).
    Divider { id: WidgetId, vertical: bool },
    /// Spacer (empty space with a size).
    Spacer { id: WidgetId, size: f64 },
    /// Scrollable container.
    ScrollView { id: WidgetId, children: Vec<Widget>, max_height: f64 },

    // ── Feedback / overlay ───────────────────────────────────────────────
    /// Modal dialog.
    Dialog { id: WidgetId, title: String, children: Vec<Widget>, open: bool },
    /// Snackbar / toast notification.
    Snackbar { id: WidgetId, message: String, severity: Severity, open: bool },
    /// Alert banner.
    Alert { id: WidgetId, message: String, severity: Severity, dismissible: bool },

    // ── Media ────────────────────────────────────────────────────────────
    /// Image.
    Image { id: WidgetId, src: String, alt: String, width: f64, height: f64 },
    /// Canvas for custom drawing.
    Canvas { id: WidgetId, width: f64, height: f64 },
}

// ── Supporting types ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Default)]
pub enum ButtonVariant {
    #[default]
    Primary,
    Secondary,
    Outline,
    Text,
    Danger,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub enum InputType {
    #[default]
    Text,
    Password,
    Email,
    Number,
    Search,
    Url,
    Tel,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub enum ProgressVariant {
    #[default]
    Linear,
    Circular,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub enum Severity {
    #[default]
    Info,
    Success,
    Warning,
    Error,
}

#[derive(Debug, Clone)]
pub struct SelectOption { pub label: String, pub value: String }

#[derive(Debug, Clone)]
pub struct TreeNode {
    pub id: String,
    pub label: String,
    pub children: Vec<TreeNode>,
    pub expanded: bool,
}

#[derive(Debug, Clone)]
pub struct AccordionSection {
    pub title: String,
    pub content: Widget,
    pub open: bool,
}

#[derive(Debug, Clone)]
pub struct BreadcrumbItem {
    pub label: String,
    pub path: Option<String>,
}

#[derive(Debug, Clone)]
pub struct MenuItem {
    pub id: String,
    pub label: String,
    pub icon: Option<String>,
    pub disabled: bool,
    pub children: Vec<MenuItem>,
}

#[derive(Debug, Clone)]
pub struct NavItem {
    pub id: String,
    pub label: String,
    pub path: String,
    pub icon: Option<String>,
}

#[derive(Debug, Clone)]
pub struct UiWindow {
    pub id: WidgetId,
    pub title: String,
    pub root: Widget,
}

/// Full patch: what you’d serialize / drive from Killer.
#[derive(Debug, Clone, Default)]
pub struct UiPatch {
    pub windows: Vec<UiWindow>,
    /// Opaque state bag for sliders/toggles when not fully derived from widgets (host sync).
    pub state: HashMap<String, f64>,
    pub toggles: HashMap<String, bool>,
    /// Text input values.
    pub text_state: HashMap<String, String>,
}

impl UiPatch {
    pub fn demo_a() -> Self {
        let root = Widget::Column {
            id: "root".into(),
            children: vec![
                Widget::Label {
                    id: "l1".into(),
                    text: "killer_ui — Phase A (model)".into(),
                },
                Widget::Button {
                    id: "btn_cook".into(),
                    label: "Cook graph".into(),
                    variant: ButtonVariant::Primary,
                    disabled: false,
                },
                Widget::Slider {
                    id: "gain".into(),
                    label: "Gain".into(),
                    min: 0.0,
                    max: 2.0,
                    value: 1.0,
                },
                Widget::Label {
                    id: "cook_lbl".into(),
                    text: "Cook sum = (headless)".into(),
                },
            ],
        };
        Self {
            windows: vec![UiWindow {
                id: "main".into(),
                title: "Killer UI".into(),
                root,
            }],
            state: HashMap::from([("gain".into(), 1.0)]),
            toggles: HashMap::new(),
            text_state: HashMap::new(),
        }
    }

    /// Extended demo showcasing the full 30+ widget set.
    pub fn demo_full() -> Self {
        let root = Widget::Column {
            id: "root".into(),
            children: vec![
                Widget::Card {
                    id: "header_card".into(),
                    title: Some("killer_ui — Full Widget Demo".into()),
                    elevated: true,
                    children: vec![
                        Widget::Breadcrumb {
                            id: "bc".into(),
                            items: vec![
                                BreadcrumbItem { label: "Home".into(), path: Some("/".into()) },
                                BreadcrumbItem { label: "Demo".into(), path: None },
                            ],
                        },
                    ],
                },
                Widget::Row {
                    id: "buttons_row".into(),
                    gap: 8.0,
                    children: vec![
                        Widget::Button { id: "btn_primary".into(), label: "Primary".into(), variant: ButtonVariant::Primary, disabled: false },
                        Widget::Button { id: "btn_secondary".into(), label: "Secondary".into(), variant: ButtonVariant::Secondary, disabled: false },
                        Widget::Button { id: "btn_outline".into(), label: "Outline".into(), variant: ButtonVariant::Outline, disabled: false },
                        Widget::Button { id: "btn_danger".into(), label: "Danger".into(), variant: ButtonVariant::Danger, disabled: false },
                        Widget::Button { id: "btn_disabled".into(), label: "Disabled".into(), variant: ButtonVariant::Primary, disabled: true },
                    ],
                },
                Widget::Divider { id: "div1".into(), vertical: false },
                Widget::Card {
                    id: "form_card".into(),
                    title: Some("Form Widgets".into()),
                    elevated: false,
                    children: vec![
                        Widget::Form {
                            id: "demo_form".into(),
                            children: vec![
                                Widget::TextInput { id: "name_input".into(), label: "Name".into(), value: String::new(), placeholder: "Enter your name".into(), input_type: InputType::Text },
                                Widget::TextInput { id: "email_input".into(), label: "Email".into(), value: String::new(), placeholder: "user@example.com".into(), input_type: InputType::Email },
                                Widget::TextArea { id: "bio_input".into(), label: "Bio".into(), value: String::new(), rows: 3 },
                                Widget::Select {
                                    id: "role_select".into(), label: "Role".into(),
                                    options: vec![
                                        SelectOption { label: "Viewer".into(), value: "viewer".into() },
                                        SelectOption { label: "Editor".into(), value: "editor".into() },
                                        SelectOption { label: "Admin".into(), value: "admin".into() },
                                    ],
                                    selected: Some("viewer".into()),
                                },
                                Widget::Checkbox { id: "terms_check".into(), label: "Accept terms".into(), checked: false },
                                Widget::RadioGroup {
                                    id: "plan_radio".into(), label: "Plan".into(),
                                    options: vec!["Free".into(), "Pro".into(), "Enterprise".into()],
                                    selected: Some("Free".into()),
                                },
                                Widget::DatePicker { id: "date_pick".into(), label: "Start date".into(), value: "2026-04-14".into() },
                                Widget::ColorPicker { id: "color_pick".into(), label: "Theme color".into(), value: "#1976d2".into() },
                                Widget::Toggle { id: "dark_toggle".into(), label: "Dark mode".into(), on: false },
                                Widget::Slider { id: "gain".into(), label: "Gain".into(), min: 0.0, max: 2.0, value: 1.0 },
                            ],
                        },
                    ],
                },
                Widget::Tabs {
                    id: "demo_tabs".into(),
                    labels: vec!["Data".into(), "Navigation".into(), "Feedback".into()],
                    active: 0,
                    children: vec![
                        Widget::Table {
                            id: "demo_table".into(),
                            headers: vec!["Name".into(), "Role".into(), "Status".into()],
                            rows: vec![
                                vec!["Alice".into(), "Admin".into(), "Active".into()],
                                vec!["Bob".into(), "Editor".into(), "Inactive".into()],
                            ],
                            sortable: true,
                        },
                        Widget::TreeView {
                            id: "demo_tree".into(),
                            nodes: vec![TreeNode {
                                id: "src".into(), label: "src/".into(), expanded: true,
                                children: vec![
                                    TreeNode { id: "main_rs".into(), label: "main.rs".into(), expanded: false, children: vec![] },
                                    TreeNode { id: "lib_rs".into(), label: "lib.rs".into(), expanded: false, children: vec![] },
                                ],
                            }],
                        },
                        Widget::Alert { id: "demo_alert".into(), message: "This is an info alert".into(), severity: Severity::Info, dismissible: true },
                    ],
                },
                Widget::Row {
                    id: "data_row".into(),
                    gap: 8.0,
                    children: vec![
                        Widget::Badge { id: "badge_ok".into(), text: "OK".into(), color: "green".into() },
                        Widget::Badge { id: "badge_err".into(), text: "Error".into(), color: "red".into() },
                        Widget::ProgressBar { id: "prog1".into(), value: 65.0, max: 100.0, variant: ProgressVariant::Linear },
                        Widget::Spinner { id: "spinner1".into(), size: 24.0 },
                        Widget::Avatar { id: "avatar1".into(), text: "SK".into(), src: None, size: 40.0 },
                    ],
                },
                Widget::Pagination { id: "pager".into(), total_pages: 10, current_page: 1 },
            ],
        };
        Self {
            windows: vec![UiWindow { id: "main".into(), title: "Killer UI — Full Demo".into(), root }],
            state: HashMap::from([("gain".into(), 1.0)]),
            toggles: HashMap::new(),
            text_state: HashMap::new(),
        }
    }
}
