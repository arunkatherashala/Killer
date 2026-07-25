//! **Phase B** — native window shell with real rendering.
//!
//! Uses Win32 API directly (zero external crates) for window creation + GDI blit.
//! Software renderer: widget tree → framebuffer → StretchDIBits to screen.

use super::framebuffer::Framebuffer;
use super::graph::OperatorGraph;
use super::patch::UiPatch;
use super::render_widgets::{self, RenderTheme};

#[cfg(target_os = "windows")]
use super::window_win32;

/// Summarize `cook_floats()` for CLI / future window title.
pub fn cook_summary(graph: &OperatorGraph) -> String {
    match graph.cook_floats() {
        Ok(m) => {
            let mut pairs: Vec<_> = m.into_iter().collect();
            pairs.sort_by(|a, b| a.0.cmp(&b.0));
            pairs
                .into_iter()
                .map(|(k, v)| format!("{}={:.4}", k, v))
                .collect::<Vec<_>>()
                .join("  ")
        }
        Err(e) => format!("cook error: {:?}", e),
    }
}

/// Open a real desktop window and render the default demo patch at ~60 FPS.
///
/// On Windows: creates a Win32 window, renders the widget gallery, pumps events.
/// On non-Windows: falls back to the stderr stub.
pub fn run_demo_window(summary: String) -> std::io::Result<()> {
    run_window_with_patch(None, &summary)
}

/// Open a window rendering a specific [`UiPatch`]. Pass `None` for the demo gallery.
pub fn run_window_with_patch(patch: Option<&UiPatch>, summary: &str) -> std::io::Result<()> {
    #[cfg(target_os = "windows")]
    {
        let demo_patch;
        let patch = match patch {
            Some(p) => p,
            None => {
                demo_patch = render_widgets::demo_all_widgets();
                &demo_patch
            }
        };

        let title = if !summary.is_empty() {
            format!("Killer UI — {}", summary)
        } else {
            "Killer UI".to_string()
        };

        let hwnd = window_win32::create_window(&title, 900, 650)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

        let theme = RenderTheme::light();
        let mut fb = Framebuffer::new(900, 650);

        // Initial render
        render_widgets::render_patch(&mut fb, patch, &theme);
        window_win32::blit_with_getdc(hwnd, &fb);

        // Event loop
        while window_win32::pump_messages() {
            let events = window_win32::drain_events();

            let mut needs_repaint = false;
            for ev in &events {
                match ev {
                    window_win32::WinEvent::Paint | window_win32::WinEvent::Timer => {
                        needs_repaint = true;
                    }
                    window_win32::WinEvent::Resize { width, height } => {
                        if *width > 0 && *height > 0 {
                            fb = Framebuffer::new(*width, *height);
                            needs_repaint = true;
                        }
                    }
                    window_win32::WinEvent::Close => {
                        return Ok(());
                    }
                    _ => {}
                }
            }

            if needs_repaint {
                let (w, h) = window_win32::window_size();
                if fb.width != w || fb.height != h {
                    fb = Framebuffer::new(w, h);
                }
                render_widgets::render_patch(&mut fb, patch, &theme);
                window_win32::blit_with_getdc(hwnd, &fb);
            }
        }

        Ok(())
    }

    #[cfg(not(target_os = "windows"))]
    {
        eprintln!("killer_ui Phase B: native window requires Windows (Win32 GDI backend).");
        eprintln!("  {}", summary);
        Ok(())
    }
}
