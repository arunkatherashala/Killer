//! **Component Library** — Pre-built Material Design component catalog.
//!
//! Factory functions producing fully styled, accessible `Widget` trees.
//! Equivalent to MUI (React) / Angular Material / Ant Design.
//! Every component returns Widget trees ready to render.

use super::patch::*;

// ══════════════════════════════════════════════════════════════════════════════
// Material Theme Tokens
// ══════════════════════════════════════════════════════════════════════════════

/// Material Design color palette tokens.
#[derive(Debug, Clone)]
pub struct MaterialTheme {
    pub primary: String,
    pub secondary: String,
    pub error_color: String,
    pub warning: String,
    pub info: String,
    pub success: String,
    pub background: String,
    pub surface: String,
    pub on_primary: String,
    pub on_surface: String,
    pub border_radius: f64,
    pub spacing_unit: f64,
}

impl Default for MaterialTheme {
    fn default() -> Self {
        MaterialTheme {
            primary: "#1976d2".into(),
            secondary: "#9c27b0".into(),
            error_color: "#d32f2f".into(),
            warning: "#ed6c02".into(),
            info: "#0288d1".into(),
            success: "#2e7d32".into(),
            background: "#ffffff".into(),
            surface: "#f5f5f5".into(),
            on_primary: "#ffffff".into(),
            on_surface: "#212121".into(),
            border_radius: 4.0,
            spacing_unit: 8.0,
        }
    }
}

impl MaterialTheme {
    pub fn dark() -> Self {
        MaterialTheme {
            primary: "#90caf9".into(),
            secondary: "#ce93d8".into(),
            error_color: "#f44336".into(),
            warning: "#ffa726".into(),
            info: "#29b6f6".into(),
            success: "#66bb6a".into(),
            background: "#121212".into(),
            surface: "#1e1e1e".into(),
            on_primary: "#000000".into(),
            on_surface: "#ffffff".into(),
            border_radius: 4.0,
            spacing_unit: 8.0,
        }
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// AppBar — Top navigation bar
// ══════════════════════════════════════════════════════════════════════════════

/// Material AppBar / Toolbar.
pub fn app_bar(title: &str, actions: Vec<Widget>) -> Widget {
    let mut children = vec![
        Widget::Label { id: "appbar_title".into(), text: title.into() },
        Widget::Spacer { id: "appbar_spacer".into(), size: 1.0 },
    ];
    children.extend(actions);
    Widget::Row { id: "appbar".into(), children, gap: 8.0 }
}

/// AppBar with navigation drawer toggle.
pub fn app_bar_with_drawer(title: &str, on_menu: &str) -> Widget {
    Widget::Row {
        id: "appbar".into(),
        children: vec![
            Widget::Button { id: on_menu.into(), label: "☰".into(), variant: ButtonVariant::Text, disabled: false },
            Widget::Label { id: "appbar_title".into(), text: title.into() },
        ],
        gap: 8.0,
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Drawer — Side navigation panel
// ══════════════════════════════════════════════════════════════════════════════

/// Material navigation drawer.
pub fn drawer(items: Vec<(&str, &str)>, active: Option<&str>) -> Widget {
    let nav_items: Vec<NavItem> = items.iter().map(|(label, path)| {
        NavItem {
            id: format!("nav-{}", label.to_lowercase().replace(' ', "-")),
            label: label.to_string(),
            path: path.to_string(),
            icon: None,
        }
    }).collect();
    Widget::NavSidebar {
        id: "drawer".into(),
        items: nav_items,
        active: active.map(|s| s.into()),
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Chip — Compact element for tags, filters
// ══════════════════════════════════════════════════════════════════════════════

/// Material Chip (tag/filter).
pub fn chip(label: &str, color: &str) -> Widget {
    Widget::Badge { id: format!("chip_{}", label), text: label.into(), color: color.into() }
}

/// Chip group.
pub fn chip_group(chips: Vec<(&str, &str)>) -> Widget {
    let children = chips.into_iter().map(|(l, c)| chip(l, c)).collect();
    Widget::Row { id: "chip_group".into(), children, gap: 4.0 }
}

// ══════════════════════════════════════════════════════════════════════════════
// FAB — Floating Action Button
// ══════════════════════════════════════════════════════════════════════════════

/// Floating Action Button.
pub fn fab(icon: &str, action_id: &str) -> Widget {
    Widget::Button {
        id: action_id.into(),
        label: icon.into(),
        variant: ButtonVariant::Primary,
        disabled: false,
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Stepper — Multi-step wizard
// ══════════════════════════════════════════════════════════════════════════════

/// Stepper step definition.
pub struct Step {
    pub label: String,
    pub content: Vec<Widget>,
    pub optional: bool,
}

/// Material Stepper (wizard).
pub fn stepper(steps: Vec<Step>, active_step: usize) -> Widget {
    let labels: Vec<String> = steps.iter().enumerate().map(|(i, s)| {
        if i == active_step { format!("● {}", s.label) }
        else if i < active_step { format!("✓ {}", s.label) }
        else { format!("○ {}", s.label) }
    }).collect();

    let mut children: Vec<Widget> = vec![
        Widget::Row {
            id: "stepper_header".into(),
            children: labels.iter().enumerate().map(|(i, l)| {
                Widget::Label { id: format!("step_{}", i), text: l.clone() }
            }).collect(),
            gap: 16.0,
        },
        Widget::Divider { id: "stepper_div".into(), vertical: false },
    ];

    if active_step < steps.len() {
        children.extend(steps[active_step].content.clone());
    }

    Widget::Column { id: "stepper".into(), children }
}

// ══════════════════════════════════════════════════════════════════════════════
// DataGrid — Advanced data table with sorting, filtering, pagination
// ══════════════════════════════════════════════════════════════════════════════

/// Column definition for DataGrid.
#[derive(Debug, Clone)]
pub struct DataGridColumn {
    pub field: String,
    pub header: String,
    pub sortable: bool,
    pub width: Option<f64>,
}

/// Build a full DataGrid with pagination.
pub fn data_grid(
    columns: &[DataGridColumn],
    rows: &[Vec<String>],
    page: usize,
    page_size: usize,
) -> Widget {
    let total_pages = (rows.len() + page_size - 1) / page_size;
    let start = page * page_size;
    let end = (start + page_size).min(rows.len());
    let page_rows: Vec<Vec<String>> = rows[start..end].to_vec();
    let headers: Vec<String> = columns.iter().map(|c| c.header.clone()).collect();

    Widget::Column {
        id: "data_grid".into(),
        children: vec![
            Widget::Table {
                id: "dg_table".into(),
                headers,
                rows: page_rows,
                sortable: columns.iter().any(|c| c.sortable),
            },
            Widget::Pagination {
                id: "dg_pagination".into(),
                total_pages,
                current_page: page,
            },
        ],
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// BottomNavigation
// ══════════════════════════════════════════════════════════════════════════════

/// Mobile bottom navigation bar.
pub fn bottom_nav(items: Vec<(&str, &str)>, active: usize) -> Widget {
    let children: Vec<Widget> = items.iter().enumerate().map(|(i, (label, icon))| {
        let variant = if i == active { ButtonVariant::Primary } else { ButtonVariant::Text };
        Widget::Button {
            id: format!("bnav_{}", i),
            label: format!("{} {}", icon, label),
            variant,
            disabled: false,
        }
    }).collect();
    Widget::Row { id: "bottom_nav".into(), children, gap: 0.0 }
}

// ══════════════════════════════════════════════════════════════════════════════
// SnackbarQueue — Queued toast notifications
// ══════════════════════════════════════════════════════════════════════════════

/// Manages a queue of snackbar messages.
pub struct SnackbarQueue {
    messages: Vec<(String, Severity, u64)>, // (msg, severity, duration_ms)
}

impl SnackbarQueue {
    pub fn new() -> Self { SnackbarQueue { messages: Vec::new() } }

    pub fn push(&mut self, msg: &str, severity: Severity, duration_ms: u64) {
        self.messages.push((msg.into(), severity, duration_ms));
    }

    pub fn pop(&mut self) -> Option<Widget> {
        if self.messages.is_empty() { return None; }
        let (msg, severity, _dur) = self.messages.remove(0);
        Some(Widget::Snackbar {
            id: format!("snackbar_{}", self.messages.len()),
            message: msg,
            severity,
            open: true,
        })
    }

    pub fn len(&self) -> usize { self.messages.len() }
    pub fn is_empty(&self) -> bool { self.messages.is_empty() }
}

impl Default for SnackbarQueue {
    fn default() -> Self { Self::new() }
}

// ══════════════════════════════════════════════════════════════════════════════
// Login Form — Pre-built auth form
// ══════════════════════════════════════════════════════════════════════════════

/// Pre-built login form.
pub fn login_form(logo_text: &str) -> Widget {
    Widget::Card {
        id: "login_card".into(),
        title: Some(logo_text.into()),
        elevated: true,
        children: vec![
            Widget::Form {
                id: "login_form".into(),
                children: vec![
                    Widget::TextInput {
                        id: "login_email".into(), label: "Email".into(),
                        value: String::new(), placeholder: "Enter email".into(),
                        input_type: InputType::Email,
                    },
                    Widget::TextInput {
                        id: "login_password".into(), label: "Password".into(),
                        value: String::new(), placeholder: "Enter password".into(),
                        input_type: InputType::Password,
                    },
                    Widget::Button {
                        id: "login_submit".into(), label: "Sign In".into(),
                        variant: ButtonVariant::Primary, disabled: false,
                    },
                ],
            },
        ],
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Dashboard Layout — Common app shell
// ══════════════════════════════════════════════════════════════════════════════

/// Standard dashboard layout: AppBar + Sidebar + Content.
pub fn dashboard_layout(title: &str, nav_items: Vec<(&str, &str)>, content: Vec<Widget>) -> Widget {
    Widget::Column {
        id: "dashboard".into(),
        children: vec![
            app_bar(title, vec![]),
            Widget::Row {
                id: "dashboard_body".into(),
                children: vec![
                    drawer(nav_items, None),
                    Widget::Column { id: "dashboard_content".into(), children: content },
                ],
                gap: 0.0,
            },
        ],
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Empty State — Placeholder when no data
// ══════════════════════════════════════════════════════════════════════════════

/// Empty state placeholder.
pub fn empty_state(icon: &str, title: &str, description: &str, action: Option<(&str, &str)>) -> Widget {
    let mut children = vec![
        Widget::Icon { id: "empty_icon".into(), name: icon.into(), size: 64.0 },
        Widget::Label { id: "empty_title".into(), text: title.into() },
        Widget::Label { id: "empty_desc".into(), text: description.into() },
    ];
    if let Some((label, id)) = action {
        children.push(Widget::Button {
            id: id.into(), label: label.into(),
            variant: ButtonVariant::Primary, disabled: false,
        });
    }
    Widget::Column { id: "empty_state".into(), children }
}

// ══════════════════════════════════════════════════════════════════════════════
// Confirm Dialog
// ══════════════════════════════════════════════════════════════════════════════

/// Confirmation dialog with OK/Cancel buttons.
pub fn confirm_dialog(title: &str, message: &str, open: bool) -> Widget {
    Widget::Dialog {
        id: "confirm_dialog".into(),
        title: title.into(),
        open,
        children: vec![
            Widget::Label { id: "confirm_msg".into(), text: message.into() },
            Widget::Row {
                id: "confirm_actions".into(),
                children: vec![
                    Widget::Button { id: "confirm_cancel".into(), label: "Cancel".into(), variant: ButtonVariant::Text, disabled: false },
                    Widget::Button { id: "confirm_ok".into(), label: "OK".into(), variant: ButtonVariant::Primary, disabled: false },
                ],
                gap: 8.0,
            },
        ],
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Stat Card — Dashboard metric card
// ══════════════════════════════════════════════════════════════════════════════

/// Dashboard stat card (metric + trend).
pub fn stat_card(title: &str, value: &str, trend: Option<&str>) -> Widget {
    let mut children = vec![
        Widget::Label { id: format!("stat_{}_title", title), text: title.into() },
        Widget::Label { id: format!("stat_{}_value", title), text: value.into() },
    ];
    if let Some(t) = trend {
        children.push(Widget::Badge {
            id: format!("stat_{}_trend", title),
            text: t.into(),
            color: if t.starts_with('+') { "#4caf50".into() } else { "#f44336".into() },
        });
    }
    Widget::Card { id: format!("stat_{}", title), title: None, children, elevated: true }
}

// ══════════════════════════════════════════════════════════════════════════════
// Tests
// ══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_app_bar() {
        let bar = app_bar("My App", vec![]);
        if let Widget::Row { children, .. } = &bar {
            assert!(children.len() >= 2);
        } else { panic!("Expected Row"); }
    }

    #[test]
    fn create_drawer() {
        let d = drawer(vec![("Home", "/"), ("Settings", "/settings")], Some("/"));
        if let Widget::NavSidebar { items, active, .. } = &d {
            assert_eq!(items.len(), 2);
            assert_eq!(active.as_deref(), Some("/"));
        } else { panic!("Expected NavSidebar"); }
    }

    #[test]
    fn create_stepper() {
        let steps = vec![
            Step { label: "Step 1".into(), content: vec![Widget::Label { id: "s1".into(), text: "First".into() }], optional: false },
            Step { label: "Step 2".into(), content: vec![Widget::Label { id: "s2".into(), text: "Second".into() }], optional: false },
        ];
        let s = stepper(steps, 0);
        if let Widget::Column { children, .. } = &s {
            assert!(children.len() >= 3); // header + divider + content
        } else { panic!("Expected Column"); }
    }

    #[test]
    fn create_data_grid() {
        let cols = vec![
            DataGridColumn { field: "name".into(), header: "Name".into(), sortable: true, width: None },
            DataGridColumn { field: "age".into(), header: "Age".into(), sortable: false, width: None },
        ];
        let rows = vec![
            vec!["Alice".into(), "30".into()],
            vec!["Bob".into(), "25".into()],
        ];
        let g = data_grid(&cols, &rows, 0, 10);
        if let Widget::Column { children, .. } = &g {
            assert_eq!(children.len(), 2); // table + pagination
        } else { panic!("Expected Column"); }
    }

    #[test]
    fn snackbar_queue() {
        let mut q = SnackbarQueue::new();
        q.push("Saved!", Severity::Success, 3000);
        q.push("Error!", Severity::Error, 5000);
        assert_eq!(q.len(), 2);
        let first = q.pop().unwrap();
        if let Widget::Snackbar { message, severity, .. } = &first {
            assert_eq!(message, "Saved!");
            assert_eq!(*severity, Severity::Success);
        }
        assert_eq!(q.len(), 1);
    }

    #[test]
    fn create_login_form() {
        let form = login_form("My App");
        if let Widget::Card { title, children, .. } = &form {
            assert_eq!(title.as_deref(), Some("My App"));
            assert_eq!(children.len(), 1); // Form
        } else { panic!("Expected Card"); }
    }

    #[test]
    fn create_dashboard() {
        let d = dashboard_layout("Admin", vec![("Home", "/")], vec![
            Widget::Label { id: "content".into(), text: "Welcome".into() },
        ]);
        if let Widget::Column { children, .. } = &d {
            assert_eq!(children.len(), 2); // appbar + body
        } else { panic!("Expected Column"); }
    }

    #[test]
    fn create_empty_state() {
        let e = empty_state("inbox", "No messages", "Check back later", Some(("Refresh", "refresh_btn")));
        if let Widget::Column { children, .. } = &e {
            assert_eq!(children.len(), 4); // icon + title + desc + button
        } else { panic!("Expected Column"); }
    }

    #[test]
    fn create_confirm_dialog() {
        let d = confirm_dialog("Delete?", "Are you sure?", true);
        if let Widget::Dialog { open, children, .. } = &d {
            assert!(*open);
            assert_eq!(children.len(), 2); // message + actions
        } else { panic!("Expected Dialog"); }
    }

    #[test]
    fn create_stat_card() {
        let c = stat_card("Revenue", "$42K", Some("+12%"));
        if let Widget::Card { children, .. } = &c {
            assert_eq!(children.len(), 3); // title + value + trend
        } else { panic!("Expected Card"); }
    }

    #[test]
    fn material_theme_dark() {
        let dark = MaterialTheme::dark();
        assert_eq!(dark.background, "#121212");
        assert_eq!(dark.on_surface, "#ffffff");
    }

    #[test]
    fn chip_group_creation() {
        let cg = chip_group(vec![("Rust", "#dea584"), ("Go", "#00add8")]);
        if let Widget::Row { children, .. } = &cg {
            assert_eq!(children.len(), 2);
        } else { panic!("Expected Row"); }
    }

    #[test]
    fn bottom_nav_creation() {
        let nav = bottom_nav(vec![("Home", "🏠"), ("Search", "🔍"), ("Profile", "👤")], 0);
        if let Widget::Row { children, .. } = &nav {
            assert_eq!(children.len(), 3);
        } else { panic!("Expected Row"); }
    }
}
