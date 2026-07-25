//! **Widget renderer** — walks a [`UiPatch`] widget tree and draws each widget
//! to a [`Framebuffer`].

use super::framebuffer::Framebuffer;
use super::layout::{ComputedLayout, LayoutNode, compute_layout};
#[allow(unused_imports)]
use super::style::JustifyContent;
use super::patch::*;
use super::style::{Color, Theme, BoxEdges, Unit, FlexDirection};

// ── Theme ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct RenderTheme {
    pub bg: Color,
    pub fg: Color,
    pub primary: Color,
    pub primary_text: Color,
    pub secondary: Color,
    pub surface: Color,
    pub surface_text: Color,
    pub border: Color,
    pub muted: Color,
    pub danger: Color,
    pub success: Color,
    pub warning: Color,
    pub font_scale: u32,
    pub corner_radius: u32,
}

impl RenderTheme {
    pub fn from_theme(theme: &Theme) -> Self {
        Self {
            bg: theme.background,
            fg: theme.on_background,
            primary: theme.primary,
            primary_text: theme.on_primary,
            secondary: theme.secondary,
            surface: theme.surface,
            surface_text: theme.on_surface,
            border: Color::LIGHT_GRAY,
            muted: Color::GRAY,
            danger: Color::RED,
            success: Color::GREEN,
            warning: Color::YELLOW,
            font_scale: 1,
            corner_radius: 4,
        }
    }
    pub fn light() -> Self { Self::from_theme(&Theme::light()) }
    pub fn dark()  -> Self { Self::from_theme(&Theme::dark()) }
}
// ── Layout mapping ───────────────────────────────────────────────────────

fn widget_to_layout(widget: &Widget, index: usize) -> LayoutNode {
    let id = widget_id(widget).unwrap_or_else(|| format!("w_{}", index));
    match widget {
        Widget::Column { children, .. } => {
            let cn: Vec<LayoutNode> = children.iter().enumerate()
                .map(|(i, c)| widget_to_layout(c, i)).collect();
            LayoutNode { id, direction: FlexDirection::Column, children: cn,
                padding: BoxEdges::symmetric(8.0, 8.0), gap: 6.0, ..LayoutNode::default() }
        }
        Widget::Row { children, gap, .. } => {
            let cn: Vec<LayoutNode> = children.iter().enumerate()
                .map(|(i, c)| widget_to_layout(c, i)).collect();
            LayoutNode { id, direction: FlexDirection::Row, children: cn, gap: *gap, ..LayoutNode::default() }
        }
        Widget::Grid { children, gap, .. } => {
            let cn: Vec<LayoutNode> = children.iter().enumerate()
                .map(|(i, c)| widget_to_layout(c, i)).collect();
            LayoutNode { id, direction: FlexDirection::Row, children: cn, gap: *gap, ..LayoutNode::default() }
        }
        Widget::Card { children, .. } => {
            let cn: Vec<LayoutNode> = children.iter().enumerate()
                .map(|(i, c)| widget_to_layout(c, i)).collect();
            LayoutNode { id, direction: FlexDirection::Column, children: cn,
                padding: BoxEdges::symmetric(12.0, 12.0), gap: 8.0, ..LayoutNode::default() }
        }
        Widget::Form { children, .. } | Widget::ScrollView { children, .. } => {
            let cn: Vec<LayoutNode> = children.iter().enumerate()
                .map(|(i, c)| widget_to_layout(c, i)).collect();
            LayoutNode { id, direction: FlexDirection::Column, children: cn,
                padding: BoxEdges::symmetric(4.0, 4.0), gap: 6.0, ..LayoutNode::default() }
        }
        Widget::Dialog { children, .. } => {
            let cn: Vec<LayoutNode> = children.iter().enumerate()
                .map(|(i, c)| widget_to_layout(c, i)).collect();
            LayoutNode { id, direction: FlexDirection::Column, children: cn,
                padding: BoxEdges::symmetric(16.0, 16.0), gap: 8.0, ..LayoutNode::default() }
        }
        Widget::Tabs { children, .. } => {
            let cn: Vec<LayoutNode> = children.iter().enumerate()
                .map(|(i, c)| widget_to_layout(c, i)).collect();
            LayoutNode { id, direction: FlexDirection::Column, children: cn,
                height: Unit::Px(200.0), ..LayoutNode::default() }
        }
        Widget::Accordion { sections, .. } => {
            let cn: Vec<LayoutNode> = sections.iter().enumerate()
                .map(|(i, s)| widget_to_layout(&s.content, i)).collect();
            LayoutNode { id, direction: FlexDirection::Column, children: cn, gap: 2.0, ..LayoutNode::default() }
        }
        Widget::Label { text, .. } => {
            let tw = Framebuffer::measure_text(text, 1) as f64 + 4.0;
            LayoutNode { id, width: Unit::Px(tw), height: Unit::Px(16.0), ..LayoutNode::default() }
        }
        Widget::Button { label, .. } => {
            let tw = Framebuffer::measure_text(label, 1) as f64 + 20.0;
            LayoutNode { id, width: Unit::Px(tw), height: Unit::Px(28.0), ..LayoutNode::default() }
        }
        Widget::Slider { .. }    => LayoutNode { id, width: Unit::Px(200.0), height: Unit::Px(22.0), ..LayoutNode::default() },
        Widget::Toggle { .. }    => LayoutNode { id, width: Unit::Px(100.0), height: Unit::Px(22.0), ..LayoutNode::default() },
        Widget::TextInput { .. } => LayoutNode { id, width: Unit::Px(200.0), height: Unit::Px(28.0), ..LayoutNode::default() },
        Widget::TextArea { rows, .. } => LayoutNode { id, width: Unit::Px(200.0), height: Unit::Px(14.0 * (*rows).max(2) as f64 + 12.0), ..LayoutNode::default() },
        Widget::Select { .. }    => LayoutNode { id, width: Unit::Px(160.0), height: Unit::Px(28.0), ..LayoutNode::default() },
        Widget::Checkbox { .. }  => LayoutNode { id, width: Unit::Px(120.0), height: Unit::Px(22.0), ..LayoutNode::default() },
        Widget::RadioGroup { options, .. } => LayoutNode { id, width: Unit::Px(140.0), height: Unit::Px(22.0 * options.len().max(1) as f64), ..LayoutNode::default() },
        Widget::ProgressBar { .. } => LayoutNode { id, width: Unit::Px(200.0), height: Unit::Px(16.0), ..LayoutNode::default() },
        Widget::Badge { text, .. } => {
            let tw = Framebuffer::measure_text(text, 1) as f64 + 14.0;
            LayoutNode { id, width: Unit::Px(tw), height: Unit::Px(20.0), ..LayoutNode::default() }
        }
        Widget::DatePicker { .. } | Widget::ColorPicker { .. } => LayoutNode { id, width: Unit::Px(160.0), height: Unit::Px(28.0), ..LayoutNode::default() },
        Widget::Pagination { .. } => LayoutNode { id, width: Unit::Px(200.0), height: Unit::Px(24.0), ..LayoutNode::default() },
        Widget::Breadcrumb { items, .. } => {
            let tw: f64 = items.iter().map(|b| Framebuffer::measure_text(&b.label, 1) as f64 + 18.0).sum();
            LayoutNode { id, width: Unit::Px(tw), height: Unit::Px(22.0), ..LayoutNode::default() }
        }
        Widget::Divider { .. }   => LayoutNode { id, width: Unit::Auto, height: Unit::Px(2.0), flex_grow: 1.0, ..LayoutNode::default() },
        Widget::Spacer { size, .. } => LayoutNode { id, width: Unit::Px(*size), height: Unit::Px(*size), ..LayoutNode::default() },
        Widget::Table { rows, .. } => {
            let h = 24.0 + rows.len().min(10) as f64 * 20.0;
            LayoutNode { id, width: Unit::Px(400.0), height: Unit::Px(h), ..LayoutNode::default() }
        }
        Widget::Icon { size, .. } | Widget::Spinner { size, .. } => LayoutNode { id, width: Unit::Px(*size), height: Unit::Px(*size), ..LayoutNode::default() },
        Widget::Avatar { size, .. }  => LayoutNode { id, width: Unit::Px(*size), height: Unit::Px(*size), ..LayoutNode::default() },
        Widget::Image { width, height, .. } => LayoutNode { id, width: Unit::Px(*width), height: Unit::Px(*height), ..LayoutNode::default() },
        Widget::Alert { .. } | Widget::Snackbar { .. } => LayoutNode { id, width: Unit::Auto, height: Unit::Px(36.0), flex_grow: 1.0, ..LayoutNode::default() },
        _ => LayoutNode { id, width: Unit::Px(100.0), height: Unit::Px(24.0), ..LayoutNode::default() },
    }
}
fn widget_id(w: &Widget) -> Option<String> {
    Some(match w {
        Widget::Label { id, .. } | Widget::Button { id, .. } | Widget::Slider { id, .. }
        | Widget::Toggle { id, .. } | Widget::Icon { id, .. }
        | Widget::TextInput { id, .. } | Widget::TextArea { id, .. }
        | Widget::Select { id, .. } | Widget::Checkbox { id, .. }
        | Widget::RadioGroup { id, .. } | Widget::DatePicker { id, .. }
        | Widget::ColorPicker { id, .. } | Widget::FileUpload { id, .. }
        | Widget::Form { id, .. } | Widget::Table { id, .. }
        | Widget::List { id, .. } | Widget::Badge { id, .. }
        | Widget::ProgressBar { id, .. } | Widget::Spinner { id, .. }
        | Widget::Avatar { id, .. } | Widget::Tooltip { id, .. }
        | Widget::TreeView { id, .. } | Widget::Pagination { id, .. }
        | Widget::Tabs { id, .. } | Widget::Accordion { id, .. }
        | Widget::Breadcrumb { id, .. } | Widget::Menu { id, .. }
        | Widget::NavSidebar { id, .. }
        | Widget::Column { id, .. } | Widget::Row { id, .. }
        | Widget::Grid { id, .. } | Widget::Card { id, .. }
        | Widget::Divider { id, .. } | Widget::Spacer { id, .. }
        | Widget::ScrollView { id, .. }
        | Widget::Dialog { id, .. } | Widget::Snackbar { id, .. }
        | Widget::Alert { id, .. }
        | Widget::Image { id, .. } | Widget::Canvas { id, .. }
        => id.clone(),
    })
}

// ── Widget painters ──────────────────────────────────────────────────────────

fn draw_widget(fb: &mut Framebuffer, w: &Widget, l: &ComputedLayout, t: &RenderTheme) {
    let x = l.x as i32;
    let y = l.y as i32;
    let lw = l.width.max(1.0) as u32;
    let lh = l.height.max(1.0) as u32;
    match w {
        Widget::Label { text, .. } => {
            fb.draw_text(text, x, y + 2, t.fg, t.font_scale);
        }
        Widget::Button { label, variant, disabled, .. } => {
            let bg = if *disabled { t.muted } else { match variant {
                ButtonVariant::Primary => t.primary, ButtonVariant::Secondary => t.secondary,
                ButtonVariant::Outline => t.bg, ButtonVariant::Text => Color::TRANSPARENT,
                ButtonVariant::Danger => t.danger,
            }};
            let fg = if *disabled { t.surface_text }
                     else if matches!(variant, ButtonVariant::Outline | ButtonVariant::Text) { t.primary }
                     else { t.primary_text };
            fb.fill_rounded_rect(x, y, lw, lh, t.corner_radius, bg);
            if matches!(variant, ButtonVariant::Outline) { fb.stroke_rect(x, y, lw, lh, t.primary); }
            let tw = Framebuffer::measure_text(label, t.font_scale);
            fb.draw_text(label, x + (lw as i32 - tw) / 2, y + (lh as i32 - 10 * t.font_scale as i32) / 2, fg, t.font_scale);
        }
        Widget::Slider { label, min, max, value, .. } => {
            let ty = y + lh as i32 / 2 - 2;
            fb.fill_rect(x, ty, lw, 4, t.border);
            let f = if *max > *min { ((*value - *min) / (*max - *min)).clamp(0.0, 1.0) } else { 0.0 };
            let fw = (f * lw as f64) as u32;
            fb.fill_rect(x, ty, fw, 4, t.primary);
            fb.fill_circle(x + fw as i32, y + lh as i32 / 2, 6, t.primary);
            if !label.is_empty() { fb.draw_text(label, x + lw as i32 + 4, y + 4, t.muted, t.font_scale); }
        }
        Widget::Toggle { label, on, .. } => {
            let bg = if *on { t.primary } else { t.border };
            fb.fill_rounded_rect(x, y, 40, 20, 10, bg);
            fb.fill_circle(if *on { x + 30 } else { x + 10 }, y + 10, 8, Color::WHITE);
            fb.draw_text(label, x + 46, y + 4, t.fg, t.font_scale);
        }
        Widget::Icon { name, size, .. } => {
            let s = (*size).max(12.0) as u32;
            fb.fill_rounded_rect(x, y, s, s, 4, t.secondary);
            let ch = name.chars().next().unwrap_or('?').to_string();
            fb.draw_text(&ch, x + s as i32 / 2 - 3, y + s as i32 / 2 - 5, Color::WHITE, t.font_scale);
        }
        Widget::TextInput { value, placeholder, label: lbl, .. } => {
            if !lbl.is_empty() { fb.draw_text(lbl, x, y - 12, t.muted, t.font_scale); }
            fb.fill_rounded_rect(x, y, lw, lh, t.corner_radius, t.surface);
            fb.stroke_rect(x, y, lw, lh, t.border);
            let (d, c) = if value.is_empty() { (placeholder.as_str(), t.muted) } else { (value.as_str(), t.surface_text) };
            fb.draw_text(d, x + 6, y + 8, c, t.font_scale);
        }
        Widget::TextArea { value, label: lbl, rows, .. } => {
            if !lbl.is_empty() { fb.draw_text(lbl, x, y - 12, t.muted, t.font_scale); }
            fb.fill_rounded_rect(x, y, lw, lh, t.corner_radius, t.surface);
            fb.stroke_rect(x, y, lw, lh, t.border);
            let c = if value.is_empty() { t.muted } else { t.surface_text };
            for (i, line) in value.lines().take(*rows as usize).enumerate() {
                fb.draw_text(line, x + 6, y + 6 + i as i32 * 14, c, t.font_scale);
            }
        }
        Widget::Select { label: lbl, selected, .. } => {
            if !lbl.is_empty() { fb.draw_text(lbl, x, y - 12, t.muted, t.font_scale); }
            fb.fill_rounded_rect(x, y, lw, lh, t.corner_radius, t.surface);
            fb.stroke_rect(x, y, lw, lh, t.border);
            fb.draw_text(selected.as_deref().unwrap_or("---"), x + 6, y + 8, t.surface_text, t.font_scale);
            let ax = x + lw as i32 - 16;
            fb.fill_triangle(ax, y + lh as i32 / 2 - 2, ax + 10, y + lh as i32 / 2 - 2, ax + 5, y + lh as i32 / 2 + 4, t.muted);
        }
        Widget::Checkbox { label, checked, .. } => {
            let bs: u32 = 16;
            let by = y + (lh as i32 - bs as i32) / 2;
            fb.stroke_rect(x, by, bs, bs, t.border);
            if *checked {
                fb.fill_rect(x + 2, by + 2, bs - 4, bs - 4, t.primary);
                fb.line(x + 3, by + 8, x + 6, by + 12, Color::WHITE);
                fb.line(x + 6, by + 12, x + 12, by + 4, Color::WHITE);
            }
            fb.draw_text(label, x + bs as i32 + 6, y + 4, t.fg, t.font_scale);
        }
        Widget::RadioGroup { label: lbl, options, selected, .. } => {
            if !lbl.is_empty() { fb.draw_text(lbl, x, y - 12, t.muted, t.font_scale); }
            for (i, opt) in options.iter().enumerate() {
                let oy = y + i as i32 * 22;
                fb.stroke_circle(x + 8, oy + 10, 8, t.border);
                if selected.as_deref() == Some(opt.as_str()) { fb.fill_circle(x + 8, oy + 10, 5, t.primary); }
                fb.draw_text(opt, x + 22, oy + 4, t.fg, t.font_scale);
            }
        }
        Widget::DatePicker { value, label: lbl, .. } => {
            if !lbl.is_empty() { fb.draw_text(lbl, x, y - 12, t.muted, t.font_scale); }
            fb.fill_rounded_rect(x, y, lw, lh, t.corner_radius, t.surface);
            fb.stroke_rect(x, y, lw, lh, t.border);
            let (d, c) = if value.is_empty() { ("YYYY-MM-DD", t.muted) } else { (value.as_str(), t.surface_text) };
            fb.draw_text(d, x + 6, y + 8, c, t.font_scale);
        }
        Widget::ColorPicker { value, label: lbl, .. } => {
            if !lbl.is_empty() { fb.draw_text(lbl, x, y - 12, t.muted, t.font_scale); }
            fb.fill_rounded_rect(x, y, lw, lh, t.corner_radius, t.surface);
            fb.stroke_rect(x, y, lw, lh, t.border);
            let sw = Color::from_hex(value).unwrap_or(t.primary);
            fb.fill_rect(x + 4, y + 4, 18, lh.saturating_sub(8), sw);
            fb.draw_text(value, x + 28, y + 8, t.surface_text, t.font_scale);
        }
        Widget::ProgressBar { value, max, variant, .. } => {
            let f = if *max > 0.0 { (*value / *max).clamp(0.0, 1.0) } else { 0.0 };
            match variant {
                ProgressVariant::Linear => {
                    fb.fill_rounded_rect(x, y, lw, lh, 4, t.border);
                    let fw = (f * lw as f64) as u32;
                    if fw > 0 { fb.fill_rounded_rect(x, y, fw, lh, 4, t.primary); }
                    let p = format!("{:.0}%", f * 100.0);
                    fb.draw_text(&p, x + (lw as i32 - Framebuffer::measure_text(&p, t.font_scale)) / 2, y + 2, t.primary_text, t.font_scale);
                }
                ProgressVariant::Circular => {
                    let r = lh.min(lw) / 2;
                    fb.stroke_circle(x + r as i32, y + r as i32, r, t.border);
                    let p = format!("{:.0}%", f * 100.0);
                    let tw = Framebuffer::measure_text(&p, t.font_scale);
                    fb.draw_text(&p, x + r as i32 - tw / 2, y + r as i32 - 5, t.primary, t.font_scale);
                }
            }
        }
        Widget::Badge { text, color, .. } => {
            let bg = Color::from_hex(color).unwrap_or(t.primary);
            let tw = Framebuffer::measure_text(text, t.font_scale) as u32 + 12;
            fb.fill_rounded_rect(x, y, tw, lh, lh / 2, bg);
            fb.draw_text(text, x + 6, y + 4, Color::WHITE, t.font_scale);
        }
        Widget::Spinner { size, .. } => {
            let r = (*size as u32).max(8) / 2;
            fb.stroke_circle(x + r as i32, y + r as i32, r, t.border);
            fb.fill_circle(x + r as i32 * 2, y + r as i32, 3, t.primary);
        }
        Widget::Avatar { text, size, .. } => {
            let r = (*size as u32).max(16) / 2;
            fb.fill_circle(x + r as i32, y + r as i32, r, t.primary);
            let tw = Framebuffer::measure_text(text, t.font_scale);
            fb.draw_text(text, x + r as i32 - tw / 2, y + r as i32 - 5, Color::WHITE, t.font_scale);
        }
        Widget::Tooltip { text, child, .. } => {
            draw_widget(fb, child, l, t);
            let tw = Framebuffer::measure_text(text, t.font_scale) as u32 + 12;
            fb.fill_rounded_rect(x, y - 20, tw, 18, 4, Color::rgba(50, 50, 50, 230));
            fb.draw_text(text, x + 6, y - 16, Color::WHITE, t.font_scale);
        }
        Widget::Breadcrumb { items, .. } => {
            let mut bx = x;
            for (i, item) in items.iter().enumerate() {
                if i > 0 { fb.draw_text(">", bx, y + 4, t.muted, t.font_scale); bx += 12; }
                let last = i == items.len() - 1;
                let c = if last { t.fg } else { t.primary };
                let tw = fb.draw_text(&item.label, bx, y + 4, c, t.font_scale);
                if !last { fb.hline(bx, y + 14, tw as u32, t.primary); }
                bx += tw + 6;
            }
        }
        Widget::Pagination { total_pages, current_page, .. } => {
            let mut px = x;
            fb.draw_text("<", px, y + 4, t.primary, t.font_scale); px += 16;
            let s = (*current_page).max(1);
            let e = (*total_pages).min(s + 4);
            for p in s..=e {
                let (bg, tc) = if p == *current_page { (t.primary, t.primary_text) } else { (t.surface, t.fg) };
                fb.fill_rect(px, y, 24, 20, bg);
                fb.draw_text(&format!("{}", p), px + 6, y + 4, tc, t.font_scale);
                px += 26;
            }
            fb.draw_text(">", px, y + 4, t.primary, t.font_scale);
        }
        Widget::Tabs { labels, active, .. } => {
            let mut tx = x;
            for (i, label) in labels.iter().enumerate() {
                let tw = Framebuffer::measure_text(label, t.font_scale) as u32 + 16;
                let (bg, tc) = if i == *active { (t.primary, t.primary_text) } else { (t.surface, t.surface_text) };
                fb.fill_rect(tx, y, tw, 26, bg);
                fb.draw_text(label, tx + 8, y + 7, tc, t.font_scale);
                tx += tw as i32 + 2;
            }
            fb.stroke_rect(x, y + 26, lw, lh.saturating_sub(26), t.border);
        }
        Widget::Table { headers, rows, sortable, .. } => {
            let cw = if headers.is_empty() { lw } else { lw / headers.len() as u32 };
            fb.fill_rect(x, y, lw, 24, t.primary);
            let mut hx = x;
            for hdr in headers {
                fb.draw_text(hdr, hx + 4, y + 6, t.primary_text, t.font_scale);
                if *sortable { fb.fill_triangle(hx + cw as i32 - 12, y + 10, hx + cw as i32 - 6, y + 10, hx + cw as i32 - 9, y + 6, t.primary_text); }
                hx += cw as i32;
            }
            for (ri, row) in rows.iter().take(10).enumerate() {
                let ry = y + 24 + ri as i32 * 20;
                fb.fill_rect(x, ry, lw, 20, if ri % 2 == 0 { t.surface } else { t.bg });
                let mut cx = x;
                for cell in row.iter().take(headers.len()) { fb.draw_text(cell, cx + 4, ry + 4, t.surface_text, t.font_scale); cx += cw as i32; }
            }
        }
        Widget::Accordion { sections, .. } => {
            let mut sy = y;
            for s in sections {
                let a = if s.open { "v" } else { ">" };
                fb.fill_rounded_rect(x, sy, lw, 28, t.corner_radius, t.surface);
                fb.stroke_rect(x, sy, lw, 28, t.border);
                fb.draw_text(a, x + 6, sy + 8, t.muted, t.font_scale);
                fb.draw_text(&s.title, x + 20, sy + 8, t.fg, t.font_scale);
                sy += 30;
            }
        }
        Widget::TreeView { nodes, .. } => {
            let mut ty = y;
            draw_tree_nodes(fb, nodes, x, &mut ty, 0, t);
        }
        Widget::Dialog { title, open, .. } => {
            if !*open { return; }
            fb.fill_rect(0, 0, fb.width, fb.height, Color::rgba(0, 0, 0, 128));
            let (mw, mh) = (lw.min(fb.width - 40), lh.min(fb.height - 40));
            let (mx, my) = ((fb.width as i32 - mw as i32) / 2, (fb.height as i32 - mh as i32) / 2);
            fb.fill_rounded_rect(mx, my, mw, mh, 8, t.surface);
            fb.fill_rect(mx, my, mw, 32, t.primary);
            fb.draw_text(title, mx + 12, my + 10, t.primary_text, t.font_scale);
            fb.draw_text("X", mx + mw as i32 - 20, my + 10, t.primary_text, t.font_scale);
        }
        Widget::Alert { message, severity, dismissible, .. } => {
            let bg = sev_color(severity, t).lighten(0.7);
            fb.fill_rounded_rect(x, y, lw, lh, 4, bg);
            fb.vline(x, y, lh, sev_color(severity, t));
            fb.draw_text(message, x + 8, y + 10, t.fg, t.font_scale);
            if *dismissible { fb.draw_text("X", x + lw as i32 - 16, y + 10, t.muted, t.font_scale); }
        }
        Widget::Snackbar { message, severity, open, .. } => {
            if !*open { return; }
            fb.fill_rounded_rect(x, y, lw, lh, 4, sev_color(severity, t));
            fb.draw_text(message, x + 8, y + 10, Color::WHITE, t.font_scale);
        }
        Widget::Image { alt, .. } => { fb.fill_rect(x, y, lw, lh, t.border); fb.draw_text(&format!("[{}]", alt), x + 4, y + lh as i32 / 2 - 5, t.muted, t.font_scale); }
        Widget::Canvas { .. } => { fb.stroke_rect(x, y, lw, lh, t.border); fb.draw_text("Canvas", x + 4, y + lh as i32 / 2 - 5, t.muted, t.font_scale); }
        Widget::List { items, ordered, .. } => {
            for (i, item) in items.iter().take(15).enumerate() {
                let iy = y + i as i32 * 16;
                let prefix = if *ordered { format!("{}.", i + 1) } else { String::from("*") };
                fb.draw_text(&prefix, x, iy + 2, t.muted, t.font_scale);
                fb.draw_text(item, x + 20, iy + 2, t.fg, t.font_scale);
            }
        }
        Widget::Menu { items, .. } => {
            for (i, item) in items.iter().enumerate() {
                let iy = y + i as i32 * 28;
                fb.fill_rect(x, iy, lw, 26, if item.disabled { t.muted } else { t.surface });
                fb.draw_text(&item.label, x + 8, iy + 7, if item.disabled { t.border } else { t.fg }, t.font_scale);
            }
        }
        Widget::NavSidebar { items, active, .. } => {
            for (i, item) in items.iter().enumerate() {
                let iy = y + i as i32 * 32;
                let a = active.as_deref() == Some(item.id.as_str());
                fb.fill_rect(x, iy, lw, 30, if a { t.primary } else { t.surface });
                fb.draw_text(&item.label, x + 12, iy + 8, if a { t.primary_text } else { t.fg }, t.font_scale);
            }
        }
        Widget::FileUpload { label, .. } => { fb.stroke_rect(x, y, lw, lh, t.border); fb.draw_text(label, x + 8, y + lh as i32 / 2 - 5, t.muted, t.font_scale); }
        Widget::Column { .. } | Widget::Row { .. } | Widget::Grid { .. } | Widget::Form { .. } | Widget::ScrollView { .. } => {}
        Widget::Card { title: Some(ti), .. } => {
            fb.fill_rounded_rect(x, y, lw, lh, t.corner_radius + 2, t.surface);
            fb.stroke_rect(x, y, lw, lh, t.border);
            fb.draw_text(ti, x + 12, y + 10, t.fg, t.font_scale);
            fb.hline(x + 8, y + 22, lw.saturating_sub(16), t.border);
        }
        Widget::Card { title: None, .. } => {}
        Widget::Divider { vertical, .. } => { if *vertical { fb.vline(x, y, lh, t.border); } else { fb.hline(x, y, lw, t.border); } }
        Widget::Spacer { .. } => {}
    }
}
fn sev_color(s: &Severity, t: &RenderTheme) -> Color {
    match s {
        Severity::Info => t.primary, Severity::Success => t.success,
        Severity::Warning => t.warning, Severity::Error => t.danger,
    }
}

fn draw_tree_nodes(fb: &mut Framebuffer, nodes: &[TreeNode], x: i32, ty: &mut i32, depth: i32, t: &RenderTheme) {
    for node in nodes {
        let indent = depth * 16;
        let arrow = if node.children.is_empty() { " " } else if node.expanded { "v" } else { ">" };
        fb.draw_text(arrow, x + indent, *ty + 2, t.muted, t.font_scale);
        fb.draw_text(&node.label, x + indent + 14, *ty + 2, t.fg, t.font_scale);
        *ty += 18;
        if node.expanded { draw_tree_nodes(fb, &node.children, x, ty, depth + 1, t); }
    }
}

// ── Full render pipeline ─────────────────────────────────────────────────────

pub fn render_patch(fb: &mut Framebuffer, patch: &UiPatch, theme: &RenderTheme) {
    fb.clear(theme.bg);
    fb.fill_rect(0, 0, fb.width, 30, theme.primary);
    if let Some(win) = patch.windows.first() {
        fb.draw_text(&win.title, 10, 8, theme.primary_text, theme.font_scale);
    }
    fb.draw_text("_ [] X", fb.width as i32 - 70, 8, theme.primary_text, theme.font_scale);
    let oy = 30;
    for win in &patch.windows {
        let mut ln = widget_to_layout(&win.root, 0);
        compute_layout(&mut ln, (fb.width - 20) as f64, (fb.height - oy as u32 - 20) as f64);
        render_recursive(fb, &win.root, &ln, 10, oy + 10, theme);
    }
}

fn render_recursive(fb: &mut Framebuffer, w: &Widget, ln: &LayoutNode, ox: i32, oy: i32, t: &RenderTheme) {
    let shifted = ComputedLayout {
        x: ln.computed.x + ox as f64, y: ln.computed.y + oy as f64,
        width: ln.computed.width, height: ln.computed.height,
    };
    draw_widget(fb, w, &shifted, t);
    let children = widget_children(w);
    let (cox, coy) = (shifted.x as i32, shifted.y as i32);
    for (i, child) in children.iter().enumerate() {
        if i < ln.children.len() {
            render_recursive(fb, child, &ln.children[i], cox, coy, t);
        }
    }
}

fn widget_children(w: &Widget) -> Vec<&Widget> {
    match w {
        Widget::Column { children, .. } | Widget::Row { children, .. }
        | Widget::Grid { children, .. } | Widget::Card { children, .. }
        | Widget::Form { children, .. } | Widget::ScrollView { children, .. }
        | Widget::Dialog { children, .. } => children.iter().collect(),
        Widget::Tabs { children, active, .. } => if *active < children.len() { vec![&children[*active]] } else { vec![] },
        Widget::Tooltip { child, .. } => vec![child.as_ref()],
        _ => vec![],
    }
}

#[allow(dead_code)]
fn count_nodes(w: &Widget) -> usize {
    1 + widget_children(w).iter().map(|c| count_nodes(c)).sum::<usize>()
}

pub fn demo_all_widgets() -> UiPatch { UiPatch::demo_full() }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_demo_no_panic() {
        let patch = demo_all_widgets();
        let mut fb = Framebuffer::new(800, 600);
        render_patch(&mut fb, &patch, &RenderTheme::light());
        assert!(fb.pixels.iter().any(|&b| b != 0));
    }

    #[test]
    fn render_dark_theme() {
        let mut fb = Framebuffer::new(640, 480);
        render_patch(&mut fb, &demo_all_widgets(), &RenderTheme::dark());
        assert!(!fb.pixels.is_empty());
    }

    #[test]
    fn widget_id_extraction() {
        let w = Widget::Button { id: "b".into(), label: "X".into(), variant: ButtonVariant::Primary, disabled: false };
        assert_eq!(widget_id(&w), Some("b".into()));
    }

    #[test]
    fn count_nodes_nested() {
        let w = Widget::Column { id: "c".into(), children: vec![
            Widget::Label { id: "a".into(), text: "A".into() },
            Widget::Row { id: "r".into(), gap: 8.0, children: vec![Widget::Label { id: "b".into(), text: "B".into() }] },
        ]};
        assert_eq!(count_nodes(&w), 4);
    }
}