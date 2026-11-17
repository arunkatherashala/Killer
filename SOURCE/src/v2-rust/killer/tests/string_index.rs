//! String indexing, len and slicing by character: ASCII, 2/3/4-byte characters, combining marks,
//! negative and out-of-range indices, derived strings and strings sent to spawned tasks.
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

static NEXT: AtomicUsize = AtomicUsize::new(0);

fn run(src: &str) -> String {
    let dir: PathBuf = std::env::temp_dir().join(format!("killer_str_{}_{}", std::process::id(), NEXT.fetch_add(1, Ordering::SeqCst)));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("main.killer");
    std::fs::write(&path, src).unwrap();
    let mut child = Command::new(env!("CARGO_BIN_EXE_killer_super"))
        .arg(&path)
        .arg("--run")
        .current_dir(&dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(120);
    loop {
        if child.try_wait().unwrap().is_some() {
            break;
        }
        if Instant::now() > deadline {
            let _ = child.kill();
            let _ = std::fs::remove_dir_all(&dir);
            panic!("script hung (killed after 120s):\n{src}");
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    let out = child.wait_with_output().unwrap();
    let _ = std::fs::remove_dir_all(&dir);
    format!("{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr))
        .replace("\r\n", "\n")
        .trim()
        .to_string()
}

#[test]
fn unicode_indexing_len_and_slices() {
    let src = r#"
t = "aé€😀b" + "é" + "z"
println(len(t))
for i in range(len(t)) { println(i, t[i]) }
println(t[-1])
println(t[-2])
println(t[-8])
println(t[-9])
println(t[8])
println(t[100])
println(t[1:4])
println(t[2:3])
println(t[3:4])
println(t[:2])
println(t[-3:])
println(t[4:100])
println(t[5:2])
e = ""
println(len(e))
println(e[0])
println(e[-1])
println(e[0:3])
u = t + t
println(len(u))
println(u[8])
println(u[15])
println(u[-1])
j = ",".join(["é", "€", "😀"])
println(len(j), j[2], j[4])
r = "aéb".replace("é", "😀😀")
println(len(r), r[1], r[2], r[3])
f = f"{t[1]}{t[3]}-{len(t)}"
println(f, len(f), f[1])
sl = t[1:5]
println(len(sl), sl[0], sl[-1])
big = "x" * 5000 + "é" + "y" * 5000
println(len(big), big[5000], big[5001], big[-1], big[4999])
println(big[4998:5003])
c = "😀" * 100
println(len(c), c[99], c[-100], c[100])
"#;
    let expected = r#"8
0 a
1 é
2 €
3 😀
4 b
5 e
6 ́
7 z
z
́
a
null
null
null
é€😀
€
😀
aé
éz
béz

0
null
null

16
a
z
z
5 € 😀
4 😀 😀 b
é😀-8 4 😀
4 é b
10001 é y y x
xxéyy
100 😀 😀 null"#;
    assert_eq!(run(src), expected);
}

#[test]
fn strings_sent_over_channels_and_to_tasks() {
    let src = r#"
t = "aé€😀b" * 3
ch = chan_new()
fn worker(c, s) {
  chan_send(c, [len(s), s[1], s[3], s[-1], s[-15], s[15], s[2:4]])
  chan_send(c, s + "é")
  return 0
}
h = async_spawn(worker, ch, t)
r = chan_recv(ch)
println(r)
q = chan_recv(ch)
println(len(q), q[-1], q[15])
async_await(h)
ch2 = chan_new()
chan_send(ch2, t)
fn w2(c) {
  s = chan_recv(c)
  return s[-2] + str(len(s))
}
println(async_await(async_spawn(w2, ch2)))
a = "q" * 3000
println(len(a), a[2999], a[3000], a[-3000], a[-3001])
"#;
    let expected = r#"[15, é, 😀, b, a, null, €😀]
16 é é
😀15
3000 q null q null"#;
    assert_eq!(run(src), expected);
}

