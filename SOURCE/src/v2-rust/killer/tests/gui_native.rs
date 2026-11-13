//! Native widgets (`window(...)`, `message_box(...)`): the headless model everywhere, and real Win32 controls on
//! Windows (created but never shown, so a test run does not flash windows on a desktop; skipped when the session
//! has no desktop to create a window on).
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT: AtomicUsize = AtomicUsize::new(0);

fn run(src: &str, envs: &[(&str, &str)], remove: &[&str]) -> (bool, String, String) {
    let dir = std::env::temp_dir().join(format!("killer_gn_{}_{}", std::process::id(), NEXT.fetch_add(1, Ordering::SeqCst)));
    std::fs::create_dir_all(&dir).unwrap();
    let file: PathBuf = dir.join("t.killer");
    std::fs::write(&file, src).unwrap();
    let mut c = Command::new(env!("CARGO_BIN_EXE_killer"));
    c.arg("run").arg(&file);
    for (k, v) in envs {
        c.env(k, v);
    }
    for k in remove {
        c.env_remove(k);
    }
    let out = c.output().unwrap();
    let _ = std::fs::remove_dir_all(&dir);
    (out.status.success(), String::from_utf8_lossy(&out.stdout).replace("\r\n", "\n"), String::from_utf8_lossy(&out.stderr).replace("\r\n", "\n"))
}

const APP: &str = r#"win = window("Demo", 300, 300)
win.label("Total: 0", "total")
win.input("who", "Ann")
win.checkbox("loud", "Shout")
win.list("fruit", ["apple", "pear"], fn(w) { println("picked", w.get("fruit")) })
win.combo("size", ["S", "M"])
count = [0]
win.button("Add", fn(w) {
  count[0] = count[0] + 1
  w.set("total", "Total: " + str(count[0]))
}, "add")
win.button("Quit", fn(w) { w.close() }, "quit")
println(gui_win_mode())
win.simulate_click("add")
win.simulate_click("add")
win.simulate_click("loud")
win.simulate_click("fruit", "pear")
win.simulate_click("size", "M")
win.set("who", "Bob")
println(win.get("who"), win.get("loud"), win.get("fruit"), win.get("size"))
win.simulate_click("quit")
win.simulate_click("add")
n = win.run()
println(n, win.get("total"), win.get("who"), win.get("loud"))
println(message_box("T", "text", "yesno"))
"#;

#[test]
fn headless_widgets_are_driven_by_simulated_clicks() {
    let (ok, out, err) = run(APP, &[("KILLER_GUI_HEADLESS", "1")], &[]);
    assert!(ok, "{err}");
    // run() stops at "quit": the click on "add" queued after it is never handled
    assert_eq!(out, "headless\nBob true pear M\npicked pear\n6 Total: 2 Bob true\nyes\n");
}

#[test]
fn headless_errors_are_clear() {
    let src = "win = window(\"x\", 200, 100)\nwin.button(\"A\")\ntry { win.get(\"nope\") } catch e { println(e) }\ntry { win.button(\"A\") } catch e { println(e) }\ntry { win.simulate_click(\"A\", 1); win.input(\"i\"); win.simulate_click(\"i\") } catch e { println(e) }\n";
    let (ok, out, err) = run(src, &[("KILLER_GUI_HEADLESS", "1")], &[]);
    assert!(ok, "{err}");
    assert!(out.contains("no widget named \"nope\""), "{out}");
    assert!(out.contains("already has a widget named \"A\""), "{out}");
    assert!(out.contains("only a button, checkbox, list or combo can be clicked"), "{out}");
}

#[test]
fn run_with_a_timeout_returns_without_events() {
    let (ok, out, err) = run("win = window(\"x\")\nwin.button(\"B\", fn(w) { println(\"never\") })\nprintln(win.run(1))\n", &[("KILLER_GUI_HEADLESS", "1")], &[]);
    assert!(ok, "{err}");
    assert_eq!(out, "0\n");
}

#[cfg(windows)]
#[test]
fn real_win32_controls_behave_like_the_model() {
    let (ok, out, err) = run(APP.replace("message_box(\"T\", \"text\", \"yesno\")", "\"skip\"").as_str(), &[("KILLER_GUI_HIDDEN", "1")], &["KILLER_GUI_HEADLESS"]);
    if !ok && err.contains("cannot create a window") {
        eprintln!("skipped: no desktop in this session: {err}");
        return;
    }
    assert!(ok, "{err}");
    assert_eq!(out, "win32\nBob true pear M\npicked pear\n6 Total: 2 Bob true\nskip\n");
}
