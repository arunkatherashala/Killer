//! Ghost VM CLI — create, assemble, sign, inspect, and run `.ghst` capsules.
//!
//! Assembly source can be `-` for stdin. Signing uses `GHOST_VM_SECRET` or `--key-file`.

use std::env;
use std::fs;
use std::io::{self, Read, Write};
use killer_native::ghost_vm::{
    self, sign_capsule, verify_capsule, Capsule, GhostHost, InteractiveHost, RunStatus, MAX_RAM,
    VM_REVISION, DebugMode, run_ex, save_recovery, load_recovery,
    GhostError,
    SYS_FPRINT, SYS_STR_LOWER, SYS_STR_EQ,
};

const VERSION: &str = "3.0.0";

#[derive(Debug, Default)]
struct CliOpts {
    fuel: Option<u32>,
    ram: Option<usize>,
    dry_run: bool,
    require_sig: bool,
    key_file: Option<String>,
    reset: bool,
    greet: Option<String>,
    generations: Option<u32>,
    population: Option<u32>,
    mutation_rate: Option<u32>,
    sandbox: Option<String>,
    allow_http: bool,
    allow_files: bool,
    allow_env: bool,
    timeout: Option<u64>,
    trace: bool,
    step: bool,
    quiet: bool,
}

fn parse_opts(args: &[String]) -> (CliOpts, Vec<String>) {
    let mut opts = CliOpts::default();
    let mut rest = Vec::new();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--fuel" if i + 1 < args.len() => {
                opts.fuel = args[i + 1].parse().ok();
                i += 2;
            }
            "--ram" if i + 1 < args.len() => {
                opts.ram = args[i + 1].parse().ok();
                i += 2;
            }
            "--key-file" if i + 1 < args.len() => {
                opts.key_file = Some(args[i + 1].clone());
                i += 2;
            }
            "--greet" if i + 1 < args.len() => {
                opts.greet = Some(args[i + 1].clone());
                i += 2;
            }
            "--dry-run" => {
                opts.dry_run = true;
                i += 1;
            }
            "--require-sig" => {
                opts.require_sig = true;
                i += 1;
            }
            "--reset" => {
                opts.reset = true;
                i += 1;
            }
            "--generations" if i + 1 < args.len() => {
                opts.generations = args[i + 1].parse().ok();
                i += 2;
            }
            "--population" if i + 1 < args.len() => {
                opts.population = args[i + 1].parse().ok();
                i += 2;
            }
            "--mutation-rate" if i + 1 < args.len() => {
                opts.mutation_rate = args[i + 1].parse().ok();
                i += 2;
            }
            "--sandbox" if i + 1 < args.len() => {
                opts.sandbox = Some(args[i + 1].clone());
                i += 2;
            }
            "--timeout" if i + 1 < args.len() => {
                opts.timeout = args[i + 1].parse().ok();
                i += 2;
            }
            "--allow-http" => { opts.allow_http = true; i += 1; }
            "--allow-files" => { opts.allow_files = true; i += 1; }
            "--allow-env" => { opts.allow_env = true; i += 1; }
            "--trace" => { opts.trace = true; i += 1; }
            "--step" => { opts.step = true; i += 1; }
            "--quiet" => { opts.quiet = true; i += 1; }
            "--version" => {
                println!("Ghost VM v{VERSION} (GHST capsule format v3)");
                std::process::exit(0);
            }
            other => {
                rest.push(other.to_string());
                i += 1;
            }
        }
    }
    (opts, rest)
}

fn load_secret(opts: &CliOpts) -> io::Result<Vec<u8>> {
    if let Some(path) = &opts.key_file {
        let k = fs::read(path)?;
        if k.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "key file is empty",
            ));
        }
        return Ok(k);
    }
    match env::var("GHOST_VM_SECRET") {
        Ok(s) if !s.is_empty() => Ok(s.into_bytes()),
        _ => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "set GHOST_VM_SECRET or pass --key-file",
        )),
    }
}

fn ghost_vm_log_enabled() -> bool {
    match env::var("GHOST_VM_LOG") {
        Ok(v) => v == "1" || v.eq_ignore_ascii_case("true"),
        Err(_) => false,
    }
}

fn print_usage() {
    eprintln!(
        "Ghost VM v{VERSION} (GHST capsule format v3, VM rev {rev})\n\
         \n\
         Commands:\n\
           init <name>                     Create a new .ghost template\n\
           assemble <in.ghost> <out.ghst>  Assemble .ghost to .ghst binary\n\
           run [--trace|--step] <file>     Run a .ghst capsule\n\
           world <file>                    Run with WorldHost (file/HTTP/time)\n\
           compile <file.gl>               Compile GhostLang to .ghost assembly\n\
           compile-run <file.gl>           Compile + assemble + run\n\
           sign <file> <key>               HMAC-sign a capsule\n\
           verify <file> <key>             Verify capsule signature\n\
           dump <file>                     Disassemble a .ghst capsule\n\
           evolve <in> [out]               Optimize a capsule\n\
           recover <file>                  Resume from .recover checkpoint\n\
           live <file>                     Interactive live mode\n\
           help                            Show this help\n\
           --version                       Show version\n\
         \n\
         Flags: --trace --step --quiet --reset --fuel N --ram N\n\
                --sandbox PATH --allow-http --allow-files --allow-env --timeout SECS\n\
         \n\
         Assembly: one mnemonic per line; # comments. Labels: `name:` at line start.\n\
         Directives: .ram N  .fuel N  .caps VALUE  .data ADDR \"string\"\n\
         Opcodes: nop push pop dup swap rot add sub mul div mod eq lt gt\n\
                  fconst fadd fsub fmul fdiv itof ftoi\n\
         Memory: load U16  store U16  Control: jmp jmpif  System: syscall U8  halt",
        VERSION = VERSION, rev = VM_REVISION
    );
}

struct CliHost {
    log: bool,
}

impl CliHost {
    fn new(log: bool) -> Self {
        Self { log }
    }
}

impl GhostHost for CliHost {
    fn syscall(
        &mut self,
        id: u8,
        capsule: &mut Capsule,
    ) -> Result<bool, ghost_vm::GhostError> {
        match id {
            ghost_vm::SYS_NOP => Ok(true),
            ghost_vm::SYS_HOST_LOG => {
                let v = capsule
                    .stack
                    .pop()
                    .ok_or(ghost_vm::GhostError::StackUnderflow)?;
                if self.log {
                    eprintln!("[ghost] SYS_HOST_LOG: {v}");
                }
                Ok(true)
            }
            ghost_vm::SYS_CHECKPOINT => {
                if self.log {
                    eprintln!(
                        "[ghost] SYS_CHECKPOINT: pc={} stack_depth={}",
                        capsule.pc,
                        capsule.stack.len()
                    );
                }
                Ok(true)
            }
            ghost_vm::SYS_PRINT_NUM => {
                let v = capsule
                    .stack
                    .pop()
                    .ok_or(ghost_vm::GhostError::StackUnderflow)?;
                println!("{v}");
                Ok(true)
            }
            ghost_vm::SYS_PRINT_STR => {
                let len = capsule
                    .stack
                    .pop()
                    .ok_or(ghost_vm::GhostError::StackUnderflow)? as usize;
                let addr = capsule
                    .stack
                    .pop()
                    .ok_or(ghost_vm::GhostError::StackUnderflow)? as usize;
                if addr + len > capsule.ram.len() {
                    return Err(ghost_vm::GhostError::RamOutOfBounds);
                }
                let _ = io::stdout().write_all(&capsule.ram[addr..addr + len]);
                let _ = io::stdout().flush();
                Ok(true)
            }
            ghost_vm::SYS_PRINT_CHAR => {
                let v = capsule
                    .stack
                    .pop()
                    .ok_or(ghost_vm::GhostError::StackUnderflow)?;
                let ch = (v & 0x7F) as u8;
                let _ = io::stdout().write_all(&[ch]);
                let _ = io::stdout().flush();
                Ok(true)
            }
            SYS_FPRINT => {
                let v = capsule.stack.pop().ok_or(ghost_vm::GhostError::StackUnderflow)?;
                let f = f64::from_bits(v as u64);
                println!("{f}");
                Ok(true)
            }
            SYS_STR_LOWER => {
                let len = capsule.stack.pop().ok_or(ghost_vm::GhostError::StackUnderflow)? as usize;
                let addr = capsule.stack.pop().ok_or(ghost_vm::GhostError::StackUnderflow)? as usize;
                if addr.saturating_add(len) > capsule.ram.len() {
                    capsule.stack.push(0);
                } else {
                    for i in addr..addr + len { capsule.ram[i] = capsule.ram[i].to_ascii_lowercase(); }
                    capsule.stack.push(len as i64);
                }
                Ok(true)
            }
            SYS_STR_EQ => {
                let len2 = capsule.stack.pop().ok_or(ghost_vm::GhostError::StackUnderflow)? as usize;
                let addr2 = capsule.stack.pop().ok_or(ghost_vm::GhostError::StackUnderflow)? as usize;
                let len1 = capsule.stack.pop().ok_or(ghost_vm::GhostError::StackUnderflow)? as usize;
                let addr1 = capsule.stack.pop().ok_or(ghost_vm::GhostError::StackUnderflow)? as usize;
                let eq = len1 == len2
                    && addr1.saturating_add(len1) <= capsule.ram.len()
                    && addr2.saturating_add(len2) <= capsule.ram.len()
                    && capsule.ram[addr1..addr1 + len1] == capsule.ram[addr2..addr2 + len2];
                capsule.stack.push(if eq { 1 } else { 0 });
                Ok(true)
            }
            _ => {
                if self.log {
                    eprintln!("[ghost] unhandled syscall {id} in batch mode, stopping");
                }
                Ok(false)
            }
        }
    }
}

fn dump_capsule(path: &str, bytes: &[u8]) -> io::Result<()> {
    let c = Capsule::decode(bytes).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    println!("file: {path}");
    println!("format_version: {}", c.format_version);
    println!("capabilities: 0x{:08x}", c.capabilities);
    println!("vm_revision: {}", c.vm_revision);
    println!("fuel_per_touch: {}", c.fuel_per_touch);
    println!("pc: {}", c.pc);
    println!("stack ({}): {:?}", c.stack.len(), c.stack);
    println!("ram_len: {}", c.ram.len());
    println!("code_len: {}", c.code.len());
    match &c.signature {
        None => println!("signature: none"),
        Some(s) => println!("signature: {} bytes (HMAC-SHA256 trailer)", s.len()),
    }
    println!("--- code ---");
    print!("{}", ghost_vm::disassemble_code(&c.code));
    Ok(())
}

fn read_assembly_input(path: &str) -> io::Result<String> {
    if path == "-" {
        let mut buf = String::new();
        io::stdin().read_to_string(&mut buf)?;
        Ok(buf)
    } else {
        fs::read_to_string(path)
    }
}

fn save_capsule(path: &str, capsule: &Capsule) -> io::Result<()> {
    let out = capsule
        .encode()
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    fs::write(path, out)
}

fn run_live(path: &str, opts: &CliOpts) -> io::Result<()> {
    let bytes = fs::read(path)?;
    let mut capsule =
        Capsule::decode(&bytes).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

    if capsule.pc as usize >= capsule.code.len() {
        capsule.pc = 0;
    }

    let fuel_per_touch = opts.fuel.unwrap_or(capsule.fuel_per_touch);

    if let Some(greet) = &opts.greet {
        println!("{greet}");
    }

    // Install Ctrl+C handler via a flag
    let running = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));
    let r = running.clone();
    let _ = ctrlc_flag(&r);

    let mut host = InteractiveHost::new();

    loop {
        if !running.load(std::sync::atomic::Ordering::Relaxed) {
            println!("[goodbye]");
            save_capsule(path, &capsule)?;
            return Ok(());
        }

        let status = ghost_vm::run(&mut capsule, &mut host, Some(fuel_per_touch))
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

        match status {
            RunStatus::Yielded => {
                // VM yielded control; loop back for another touch
                continue;
            }
            RunStatus::Halted => {
                println!("[ghost has stopped]");
                save_capsule(path, &capsule)?;
                return Ok(());
            }
            RunStatus::FuelExhausted => {
                // Refuel and continue
                continue;
            }
            RunStatus::Stopped => {
                println!("[ghost stopped by host]");
                save_capsule(path, &capsule)?;
                return Ok(());
            }
        }
    }
}

/// Best-effort Ctrl+C flag using std only. Sets the bool to false on signal.
fn ctrlc_flag(flag: &std::sync::Arc<std::sync::atomic::AtomicBool>) -> Result<(), ()> {
    // std doesn't have signal handling; we use a simple approach:
    // spawn a thread that reads a line — if the main loop is blocked in stdin
    // the user can type Ctrl+C which terminates the process. We just catch panics.
    let f = flag.clone();
    std::thread::spawn(move || {
        // This thread doesn't actually do anything useful on Windows without
        // external crates. The AtomicBool is only set false if this thread
        // somehow detects a signal (it won't with pure std). Ctrl+C will
        // terminate the process directly on most platforms, which is acceptable.
        let _ = f; // keep alive
    });
    Ok(())
}

fn run_evolve(path: &str, out_path_opt: Option<&str>, opts: &CliOpts) -> io::Result<()> {
    use ghost_vm::NullHost;

    let bytes = fs::read(path)?;
    let seed_capsule =
        Capsule::decode(&bytes).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

    let generations = opts.generations.unwrap_or(100);
    let population = opts.population.unwrap_or(32).max(2) as usize;
    let fuel = opts.fuel.unwrap_or(10_000);
    let mutation_rate = opts.mutation_rate.unwrap_or(3).max(1) as usize;

    let mut rng: u32 = fuel.wrapping_mul(7).wrapping_add(42);
    let mut next_rng = || -> u32 {
        rng = rng.wrapping_mul(1103515245).wrapping_add(12345);
        (rng >> 16) & 0x7FFF
    };

    let mutate = |code: &[u8], rate: usize, rng: &mut dyn FnMut() -> u32| -> Vec<u8> {
        let mut out = code.to_vec();
        if out.is_empty() {
            return out;
        }
        for _ in 0..rate {
            let idx = rng() as usize % out.len();
            let kind = rng() % 4;
            match kind {
                0 => out[idx] = (rng() & 0xFF) as u8,                     // change byte
                1 => out[idx] = ghost_vm::OP_NOP,                         // insert NOP
                2 if out.len() > 1 => { out.remove(idx); }                // delete byte
                _ => out[idx] = ((out[idx] as u32).wrapping_add(rng())) as u8, // tweak
            }
        }
        if out.is_empty() || *out.last().unwrap() != ghost_vm::OP_HALT {
            out.push(ghost_vm::OP_HALT);
        }
        out
    };

    let score_capsule = |code: &[u8], fuel: u32| -> i64 {
        let mut c = Capsule::with_ram_and_fuel(MAX_RAM, fuel);
        c.code = code.to_vec();
        let mut h = NullHost;
        let _ = ghost_vm::run(&mut c, &mut h, Some(fuel));
        c.stack.last().copied().unwrap_or(i64::MIN)
    };

    let crossover = |a: &[u8], b: &[u8], rng: &mut dyn FnMut() -> u32| -> Vec<u8> {
        if a.is_empty() || b.is_empty() {
            return a.to_vec();
        }
        let split_a = rng() as usize % a.len();
        let split_b = rng() as usize % b.len();
        let mut child = a[..split_a].to_vec();
        child.extend_from_slice(&b[split_b..]);
        if child.is_empty() || *child.last().unwrap() != ghost_vm::OP_HALT {
            child.push(ghost_vm::OP_HALT);
        }
        if child.len() > ghost_vm::MAX_CODE {
            child.truncate(ghost_vm::MAX_CODE - 1);
            child.push(ghost_vm::OP_HALT);
        }
        child
    };

    let mut pop: Vec<Vec<u8>> = Vec::with_capacity(population);
    for _ in 0..population {
        pop.push(mutate(&seed_capsule.code, mutation_rate, &mut next_rng));
    }

    let mut best_code = seed_capsule.code.clone();
    let mut best_score = score_capsule(&best_code, fuel);

    for gen in 0..generations {
        let mut scored: Vec<(i64, Vec<u8>)> = pop
            .iter()
            .map(|code| (score_capsule(code, fuel), code.clone()))
            .collect();
        scored.sort_by(|a, b| b.0.cmp(&a.0));

        let gen_best = scored[0].0;
        let gen_avg: i64 = scored.iter().map(|(s, _)| *s as i64).sum::<i64>() / population as i64;

        if gen_best > best_score {
            best_score = gen_best;
            best_code = scored[0].1.clone();
        }

        println!(
            "gen {}: best={}, avg={}, code_len={}",
            gen, gen_best, gen_avg, scored[0].1.len()
        );

        let keep = (population / 4).max(1);
        let parents: Vec<Vec<u8>> = scored[..keep].iter().map(|(_, c)| c.clone()).collect();

        pop.clear();
        for p in &parents {
            pop.push(p.clone());
        }
        while pop.len() < population {
            let p1 = &parents[next_rng() as usize % parents.len()];
            let p2 = &parents[next_rng() as usize % parents.len()];
            let child = crossover(p1, p2, &mut next_rng);
            pop.push(mutate(&child, mutation_rate, &mut next_rng));
        }
    }

    let out_path = out_path_opt.map(|s| s.to_string())
        .unwrap_or_else(|| path.replace(".ghst", "_evolved.ghst"));
    let mut result = seed_capsule.clone();
    result.code = best_code;
    result.pc = 0;
    result.stack.clear();
    result.signature = None;
    let out_bytes = result
        .encode()
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    fs::write(&out_path, out_bytes)?;
    println!(
        "\nEvolution complete! Best score: {best_score}, saved to {out_path}"
    );
    Ok(())
}

fn run_world(path: &str, opts: &CliOpts) -> io::Result<()> {
    use killer_native::ghost_world::WorldHost;

    let bytes = fs::read(path)?;
    let mut capsule =
        Capsule::decode(&bytes).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

    if capsule.pc as usize >= capsule.code.len() {
        capsule.pc = 0;
    }

    let fuel_per_touch = opts.fuel.unwrap_or(capsule.fuel_per_touch);

    if let Some(greet) = &opts.greet {
        println!("{greet}");
    }

    let running = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));
    let r = running.clone();
    let _ = ctrlc_flag(&r);

    let mut host = WorldHost::new();
    host.allow_files = opts.allow_files;
    host.allow_http = opts.allow_http;
    host.allow_env = opts.allow_env;
    host.argv = env::args().collect();
    if let Some(sandbox) = &opts.sandbox {
        host.sandbox_root = Some(std::path::PathBuf::from(sandbox));
    }
    if let Some(timeout) = opts.timeout {
        host.http_timeout_ms = timeout * 1000;
    }

    loop {
        if !running.load(std::sync::atomic::Ordering::Relaxed) {
            println!("[goodbye]");
            save_capsule(path, &capsule)?;
            return Ok(());
        }

        let status = ghost_vm::run(&mut capsule, &mut host, Some(fuel_per_touch))
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

        match status {
            RunStatus::Yielded | RunStatus::FuelExhausted => continue,
            RunStatus::Halted => {
                println!("[ghost has stopped]");
                save_capsule(path, &capsule)?;
                return Ok(());
            }
            RunStatus::Stopped => {
                println!("[ghost stopped by host]");
                save_capsule(path, &capsule)?;
                return Ok(());
            }
        }
    }
}

fn setup_ctrlc_handler(_flag: &std::sync::Arc<std::sync::atomic::AtomicBool>) {
    // Ctrl+C flag is checked in the VM run loop.
    // Without external deps, real signal handling is platform-specific.
    // The flag can be set manually in tests via run_ex().
}

fn main() -> io::Result<()> {
    let argv: Vec<String> = env::args().collect();
    if argv.len() < 2 {
        print_usage();
        std::process::exit(1);
    }

    let cmd = argv[1].as_str();
    if matches!(cmd, "help" | "--help" | "-h") {
        print_usage();
        return Ok(());
    }

    let (opts, rest) = parse_opts(&argv[2..]);

    match cmd {
        "init" => {
            let path = rest.first().ok_or_else(|| {
                eprintln!("init: missing path");
                io::Error::new(io::ErrorKind::InvalidInput, "missing path")
            })?;
            let ram = opts.ram.unwrap_or(MAX_RAM).min(MAX_RAM);
            let fuel = opts.fuel.unwrap_or(10_000);
            let c = Capsule::with_ram_and_fuel(ram, fuel);
            let bytes = c
                .encode()
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
            fs::write(path, bytes)?;
            println!("wrote {} (ram={ram} fuel={fuel})", path);
        }
        "assemble" => {
            if rest.len() < 2 {
                eprintln!("assemble: need <in.ghost|-> <out.ghst>");
                std::process::exit(1);
            }
            let src_text = read_assembly_input(&rest[0])?;
            let capsule = ghost_vm::assemble_capsule(&src_text)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
            let bytes = capsule
                .encode()
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
            fs::write(&rest[1], bytes)?;
            println!(
                "assembled {} → {} (code {} bytes)",
                if rest[0] == "-" { "stdin" } else { &rest[0] },
                rest[1],
                capsule.code.len()
            );
        }
        "sign" => {
            let path = rest.first().ok_or_else(|| {
                eprintln!("sign: missing path");
                io::Error::new(io::ErrorKind::InvalidInput, "missing path")
            })?;
            let key = load_secret(&opts)?;
            let bytes = fs::read(path)?;
            let mut capsule = Capsule::decode(&bytes)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
            sign_capsule(&mut capsule, &key)
                .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
            let out = capsule
                .encode()
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
            fs::write(path, out)?;
            println!("signed {path} (HMAC-SHA256)");
        }
        "verify" => {
            let path = rest.first().ok_or_else(|| {
                eprintln!("verify: missing path");
                io::Error::new(io::ErrorKind::InvalidInput, "missing path")
            })?;
            let key = load_secret(&opts)?;
            let bytes = fs::read(path)?;
            let capsule = Capsule::decode(&bytes)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
            verify_capsule(&capsule, &key)
                .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
            println!("OK: signature valid for {path}");
        }
        "dump" => {
            let path = rest.first().ok_or_else(|| {
                eprintln!("dump: missing path");
                io::Error::new(io::ErrorKind::InvalidInput, "missing path")
            })?;
            let bytes = fs::read(path)?;
            dump_capsule(path, &bytes)?;
        }
        "run" => {
            let path = rest.first().ok_or_else(|| {
                eprintln!("run: missing path");
                io::Error::new(io::ErrorKind::InvalidInput, "missing path")
            })?;
            let fuel = opts.fuel;
            let bytes = fs::read(path)?;
            let mut capsule = Capsule::decode(&bytes)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

            // FIX: re-run bug — if PC is at or past end of code, reset to 0
            if opts.reset || capsule.pc as usize >= capsule.code.len() {
                capsule.pc = 0;
            }

            if opts.require_sig {
                let key = load_secret(&opts)?;
                verify_capsule(&capsule, &key)
                    .map_err(|e| io::Error::new(io::ErrorKind::PermissionDenied, e))?;
            }
            let debug_mode = if opts.step { DebugMode::Step }
                else if opts.trace { DebugMode::Trace }
                else { DebugMode::None };

            let ctrlc_flag = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
            setup_ctrlc_handler(&ctrlc_flag);

            let mut host = CliHost::new(ghost_vm_log_enabled());
            let result = run_ex(&mut capsule, &mut host, fuel, Some(&ctrlc_flag), debug_mode);

            match &result {
                Err(GhostError::Interrupted) => {
                    let recover_path = format!("{path}.recover");
                    if let Err(e) = save_recovery(&capsule, &recover_path) {
                        eprintln!("failed to save recovery: {e}");
                    } else {
                        eprintln!("interrupted — state saved to {recover_path}");
                    }
                }
                _ => {}
            }
            let status = result.map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

            if !opts.dry_run {
                save_capsule(path, &capsule)?;
            }
            if !opts.quiet {
                match status {
                    RunStatus::Halted => println!("status: halted{}", if opts.dry_run {" (dry-run)"} else {" (saved)"}),
                    RunStatus::Stopped => println!("status: stopped{}", if opts.dry_run {" (dry-run)"} else {" (saved)"}),
                    RunStatus::FuelExhausted => println!("status: fuel exhausted{}", if opts.dry_run {" (dry-run)"} else {" (saved)"}),
                    RunStatus::Yielded => println!("status: yielded{}", if opts.dry_run {" (dry-run)"} else {" (saved)"}),
                }
            }
        }
        "live" => {
            let path = rest.first().ok_or_else(|| {
                eprintln!("live: missing path");
                io::Error::new(io::ErrorKind::InvalidInput, "missing path")
            })?;
            run_live(path, &opts)?;
        }
        "evolve" => {
            let path = rest.first().ok_or_else(|| {
                eprintln!("evolve: missing path");
                io::Error::new(io::ErrorKind::InvalidInput, "missing path")
            })?;
            let out = rest.get(1).map(|s| s.as_str());
            run_evolve(path, out, &opts)?;
        }
        "world" => {
            let path = rest.first().ok_or_else(|| {
                eprintln!("world: missing path");
                io::Error::new(io::ErrorKind::InvalidInput, "missing path")
            })?;
            run_world(path, &opts)?;
        }
        "recover" => {
            let path = rest.first().ok_or_else(|| {
                eprintln!("recover: missing path");
                io::Error::new(io::ErrorKind::InvalidInput, "missing path")
            })?;
            let bytes = fs::read(path)?;
            let mut capsule = Capsule::decode(&bytes)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
            let recover_path = format!("{path}.recover");
            load_recovery(&mut capsule, &recover_path)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
            println!("recovered from {recover_path}: pc={}, stack_depth={}", capsule.pc, capsule.stack.len());
            let mut host = CliHost::new(ghost_vm_log_enabled());
            let status = ghost_vm::run(&mut capsule, &mut host, opts.fuel)
                .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
            save_capsule(path, &capsule)?;
            println!("status: {status:?} (saved)");
        }
        "compile" => {
            let path = rest.first().ok_or_else(|| {
                eprintln!("compile: missing .gl file");
                io::Error::new(io::ErrorKind::InvalidInput, "missing path")
            })?;
            let src = fs::read_to_string(path)?;
            let asm = killer_native::ghost_lang::compile_ghost_lang(&src)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
            let out_path = path.replace(".gl", ".ghost");
            fs::write(&out_path, &asm)?;
            println!("compiled {path} → {out_path}");
        }
        "compile-run" => {
            let path = rest.first().ok_or_else(|| {
                eprintln!("compile-run: missing .gl file");
                io::Error::new(io::ErrorKind::InvalidInput, "missing path")
            })?;
            let src = fs::read_to_string(path)?;
            let asm = killer_native::ghost_lang::compile_ghost_lang(&src)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
            let mut capsule = ghost_vm::assemble_capsule(&asm)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
            let mut host = InteractiveHost::new();
            let status = ghost_vm::run(&mut capsule, &mut host, opts.fuel)
                .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
            if !opts.quiet { eprintln!("status: {status:?}"); }
        }
        "help" => { print_usage(); }
        _ => {
            eprintln!("unknown command: {cmd}");
            print_usage();
            std::process::exit(1);
        }
    }
    Ok(())
}
