//! Ghost Hive CLI — 1M-agent evolution engine for Ghost VM capsules.
//!
//! Commands:
//!   new <file.hive>     — create a new hive with random seed population
//!   run <file.hive>     — evolve for N generations (resumes from saved state)
//!   status <file.hive>  — print hive stats
//!   export <file.hive> <out.ghst> — export best agent as a Ghost capsule
//!   inject <file.hive> <in.ghst>  — inject a capsule into the population

use std::env;
use std::fs;
use std::io;
use killer_native::ghost_hive::{HiveEngine, HiveConfig, FitnessMode};
use killer_native::ghost_vm::{self, Capsule};

#[derive(Debug, Default)]
struct Opts {
    population: Option<usize>,
    generations: Option<u64>,
    threads: Option<usize>,
    report_every: Option<u64>,
    fuel: Option<u32>,
    mutation_rate: Option<f64>,
    elite_ratio: Option<f64>,
    crossover_rate: Option<f64>,
    max_code_len: Option<usize>,
    mode: Option<String>,
    target: Option<i32>,
    seed_ghst: Option<String>,
}

fn parse_opts(args: &[String]) -> (Opts, Vec<String>) {
    let mut opts = Opts::default();
    let mut rest = Vec::new();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--pop" | "--population" if i + 1 < args.len() => {
                opts.population = args[i + 1].parse().ok();
                i += 2;
            }
            "--generations" if i + 1 < args.len() => {
                opts.generations = args[i + 1].parse().ok();
                i += 2;
            }
            "--threads" if i + 1 < args.len() => {
                opts.threads = args[i + 1].parse().ok();
                i += 2;
            }
            "--report-every" if i + 1 < args.len() => {
                opts.report_every = args[i + 1].parse().ok();
                i += 2;
            }
            "--fuel" if i + 1 < args.len() => {
                opts.fuel = args[i + 1].parse().ok();
                i += 2;
            }
            "--mutation-rate" if i + 1 < args.len() => {
                opts.mutation_rate = args[i + 1].parse::<f64>().ok();
                i += 2;
            }
            "--elite-ratio" if i + 1 < args.len() => {
                opts.elite_ratio = args[i + 1].parse::<f64>().ok();
                i += 2;
            }
            "--crossover-rate" if i + 1 < args.len() => {
                opts.crossover_rate = args[i + 1].parse::<f64>().ok();
                i += 2;
            }
            "--max-code-len" if i + 1 < args.len() => {
                opts.max_code_len = args[i + 1].parse().ok();
                i += 2;
            }
            "--mode" if i + 1 < args.len() => {
                opts.mode = Some(args[i + 1].clone());
                i += 2;
            }
            "--target" if i + 1 < args.len() => {
                opts.target = args[i + 1].parse().ok();
                i += 2;
            }
            "--seed" if i + 1 < args.len() => {
                opts.seed_ghst = Some(args[i + 1].clone());
                i += 2;
            }
            other => {
                rest.push(other.to_string());
                i += 1;
            }
        }
    }
    (opts, rest)
}

fn build_config(opts: &Opts) -> HiveConfig {
    let fitness_fn = match opts.mode.as_deref() {
        Some("target") => FitnessMode::Target(opts.target.unwrap_or(42)),
        Some("longevity") => FitnessMode::Longevity,
        Some("diversity") => FitnessMode::Diversity,
        _ => FitnessMode::MaxOutput,
    };

    HiveConfig {
        population_size: opts.population.unwrap_or(1000),
        elite_ratio: opts.elite_ratio.unwrap_or(0.10),
        mutation_rate: opts.mutation_rate.unwrap_or(0.05),
        crossover_rate: opts.crossover_rate.unwrap_or(0.70),
        max_code_len: opts.max_code_len.unwrap_or(256),
        fuel_per_eval: opts.fuel.unwrap_or(1000),
        fitness_fn,
        num_threads: opts.threads.unwrap_or(num_cpus()),
    }
}

fn num_cpus() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4)
}

fn print_usage() {
    eprintln!(
        "ghost_hive — Ghost Hive evolution engine\n\
         \n\
         Commands:\n\
           new <file.hive>     [--pop N] [--mode max|target|longevity|diversity] [--target V]\n\
                                [--fuel N] [--mutation-rate R] [--crossover-rate R] [--elite-ratio R]\n\
                                [--max-code-len N] [--threads T] [--seed <file.ghst>]\n\
           run <file.hive>     [--generations N] [--threads T] [--report-every R]\n\
           status <file.hive>\n\
           export <file.hive> <out.ghst>\n\
           inject <file.hive> <in.ghst>\n\
         \n\
         Defaults: --pop 1000, --generations 100, --fuel 1000, --threads <num_cpus>,\n\
                   --report-every 10, --mutation-rate 0.05, --crossover-rate 0.70,\n\
                   --elite-ratio 0.10, --max-code-len 256, --mode max\n\
         \n\
         Fitness modes:\n\
           max        — maximize stack top after halt (default)\n\
           target     — minimize distance to --target value\n\
           longevity  — maximize fuel consumed before halt\n\
           diversity  — maximize unique outputs across inputs -5..5"
    );
}

fn save_hive(path: &str, engine: &HiveEngine) -> io::Result<()> {
    let bytes = engine.encode();
    fs::write(path, bytes)
}

fn load_hive(path: &str) -> io::Result<HiveEngine> {
    let bytes = fs::read(path)?;
    HiveEngine::decode(&bytes).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}

fn cmd_new(path: &str, opts: &Opts) -> io::Result<()> {
    let config = build_config(opts);
    let pop_size = config.population_size;

    let engine = if let Some(seed_path) = &opts.seed_ghst {
        let seed_bytes = fs::read(seed_path)?;
        let capsule = Capsule::decode(&seed_bytes)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        HiveEngine::with_seed(config, capsule.code)
    } else {
        HiveEngine::new(config)
    };

    save_hive(path, &engine)?;
    println!("Created {path} with {pop_size} agents");
    Ok(())
}

fn cmd_run(path: &str, opts: &Opts) -> io::Result<()> {
    let mut engine = load_hive(path)?;

    if let Some(t) = opts.threads {
        engine.config.num_threads = t;
    }

    let generations = opts.generations.unwrap_or(100);
    let report_every = opts.report_every.unwrap_or(10);
    let autosave_every: u64 = 100;

    let running = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));
    let r = running.clone();
    // Best-effort Ctrl+C
    std::thread::spawn(move || { let _ = &r; });

    let start_gen = engine.generation;
    println!(
        "Resuming from gen {} — running {} more generations ({} agents, {} threads)",
        start_gen, generations, engine.config.population_size, engine.config.num_threads,
    );

    for g in 0..generations {
        if !running.load(std::sync::atomic::Ordering::Relaxed) {
            println!("\n[interrupted — saving]");
            break;
        }

        let report = engine.evolve_generation();

        if (g + 1) % report_every == 0 || g == 0 || g == generations - 1 {
            println!("{report}");
        }

        if (g + 1) % autosave_every == 0 {
            save_hive(path, &engine)?;
        }
    }

    // Print hall of fame
    if !engine.hall_of_fame.is_empty() {
        println!();
        let show = engine.hall_of_fame.len().min(5);
        for (i, agent) in engine.hall_of_fame.iter().take(show).enumerate() {
            let code_hex: String = agent.code.iter().take(16)
                .map(|b| format!("{b:02x}"))
                .collect::<Vec<_>>().join(" ");
            let suffix = if agent.code.len() > 16 { "..." } else { "" };
            println!(
                "[HALL OF FAME] #{}: fitness={}, age={}, code=[{code_hex}{suffix}]",
                i + 1, agent.fitness, agent.age,
            );
        }
    }

    save_hive(path, &engine)?;
    println!("\nSaved to {path} (gen {})", engine.generation);
    Ok(())
}

fn cmd_status(path: &str) -> io::Result<()> {
    let engine = load_hive(path)?;

    let mode_str = match &engine.config.fitness_fn {
        FitnessMode::MaxOutput => "MaxOutput",
        FitnessMode::Target(v) => &format!("Target({v})"),
        FitnessMode::Longevity => "Longevity",
        FitnessMode::Diversity => "Diversity",
        FitnessMode::TestSuite { inputs, .. } => &format!("TestSuite({} cases)", inputs.len()),
    };

    println!("file: {path}");
    println!("generation: {}", engine.generation);
    println!("population: {}", engine.population.len());
    println!("hall_of_fame: {}", engine.hall_of_fame.len());
    println!("fitness_mode: {mode_str}");
    println!("fuel_per_eval: {}", engine.config.fuel_per_eval);
    println!("max_code_len: {}", engine.config.max_code_len);
    println!("mutation_rate: {:.2}%", engine.config.mutation_rate * 100.0);
    println!("crossover_rate: {:.2}%", engine.config.crossover_rate * 100.0);
    println!("elite_ratio: {:.2}%", engine.config.elite_ratio * 100.0);

    if let Some(best) = engine.best_agent() {
        let code_hex: String = best.code.iter().take(20)
            .map(|b| format!("{b:02x}"))
            .collect::<Vec<_>>().join(" ");
        let suffix = if best.code.len() > 20 { "..." } else { "" };
        println!("\nbest agent:");
        println!("  fitness: {}", best.fitness);
        println!("  age: {}", best.age);
        println!("  mutations: {}", best.mutations);
        println!("  code_len: {}", best.code.len());
        println!("  code: [{code_hex}{suffix}]");
        println!("  disassembly:");
        let dis = ghost_vm::disassemble_code(&best.code);
        for line in dis.lines().take(20) {
            println!("    {line}");
        }
        if dis.lines().count() > 20 {
            println!("    ...");
        }
    }

    Ok(())
}

fn cmd_export(hive_path: &str, out_path: &str) -> io::Result<()> {
    let engine = load_hive(hive_path)?;
    let capsule = engine.export_best().ok_or_else(|| {
        io::Error::new(io::ErrorKind::NotFound, "no agents in hive")
    })?;
    let bytes = capsule.encode()
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    fs::write(out_path, bytes)?;
    println!("Exported best agent to {out_path} (code {} bytes)", capsule.code.len());
    Ok(())
}

fn cmd_inject(hive_path: &str, ghst_path: &str) -> io::Result<()> {
    let mut engine = load_hive(hive_path)?;
    let bytes = fs::read(ghst_path)?;
    let capsule = Capsule::decode(&bytes)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    engine.inject(capsule.code);
    save_hive(hive_path, &engine)?;
    println!("Injected capsule from {ghst_path} into {hive_path}");
    Ok(())
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
        "new" => {
            let path = rest.first().ok_or_else(|| {
                eprintln!("new: missing path");
                io::Error::new(io::ErrorKind::InvalidInput, "missing path")
            })?;
            cmd_new(path, &opts)?;
        }
        "run" => {
            let path = rest.first().ok_or_else(|| {
                eprintln!("run: missing path");
                io::Error::new(io::ErrorKind::InvalidInput, "missing path")
            })?;
            cmd_run(path, &opts)?;
        }
        "status" => {
            let path = rest.first().ok_or_else(|| {
                eprintln!("status: missing path");
                io::Error::new(io::ErrorKind::InvalidInput, "missing path")
            })?;
            cmd_status(path)?;
        }
        "export" => {
            if rest.len() < 2 {
                eprintln!("export: need <file.hive> <out.ghst>");
                std::process::exit(1);
            }
            cmd_export(&rest[0], &rest[1])?;
        }
        "inject" => {
            if rest.len() < 2 {
                eprintln!("inject: need <file.hive> <in.ghst>");
                std::process::exit(1);
            }
            cmd_inject(&rest[0], &rest[1])?;
        }
        _ => {
            eprintln!("unknown command: {cmd}");
            print_usage();
            std::process::exit(1);
        }
    }

    Ok(())
}
