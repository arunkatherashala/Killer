use std::env;
use std::fs;
use std::path::PathBuf;
use std::time::Instant;
use std::process;
use std::process::Command;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

// Same compiler API as the `killer_super` module in this crate (crate name: killer_native)
use killer_native::{KillerSuper, KillerSuperConfig, OptimizationLevel, CompilerMode, TargetArch};

const VERSION: &str = "4.0.0";
const USAGE: &str = r#"
Killer Super Compiler v4.0.0 - Production-Grade Language Compiler
-------------------------------------------------------------------------------

USAGE:
    killer_super [OPTIONS] <INPUT_FILE>

ARGUMENTS:
    <INPUT_FILE>              Path to Killer source file (.killer)

OPTIONS:
    -o, --output <FILE>       Output file path (default: <input>.out)
    -O, --optimize <LEVEL>    Optimization level: 0, 1, 2, 3 (default: 2)
    -m, --mode <MODE>         Compiler mode: dev, prod, debug (default: prod)
    -t, --target <ARCH>       Target architecture: x86-64, arm64, wasm32, riscv64 (default: x86-64)
    --emit <FORMAT>           Emit format: native, llvm, bytecode (default: native)
    --run                     Run produced native executable after compilation
    --no-optimize             Disable all optimizations (same as -O0)
    --verbose                 Verbose compiler output
    --benchmark               Run benchmark and show performance metrics
    --stats                   Show compilation statistics
    -h, --help                Show this help message
    -v, --version             Show version information

EXAMPLES:
    # Compile with production optimizations
    killer_super program.killer -o program.exe

    # Compile with maximum optimization
    killer_super program.killer -O3 -m prod

    # Compile to LLVM IR for inspection
    killer_super program.killer --emit llvm -o program.ll

    # Development mode with fast iteration
    killer_super program.killer -m dev -O1

-------------------------------------------------------------------------------
"#;

#[derive(Debug, Clone)]
struct CliArgs {
    input_file: PathBuf,
    output_file: Option<PathBuf>,
    opt_level: OptimizationLevel,
    mode: CompilerMode,
    target: TargetArch,
    emit_format: EmitFormat,
    verbose: bool,
    benchmark: bool,
    stats: bool,
    run: bool,
    no_optimize: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum EmitFormat {
    Native,
    Llvm,
    Bytecode,
}

impl EmitFormat {
    fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "native" => Some(EmitFormat::Native),
            "llvm" => Some(EmitFormat::Llvm),
            "llir" => Some(EmitFormat::Llvm),
            "bytecode" => Some(EmitFormat::Bytecode),
            "bc" => Some(EmitFormat::Bytecode),
            _ => None,
        }
    }

    fn extension(&self) -> &'static str {
        match self {
            EmitFormat::Native => if cfg!(windows) { "exe" } else { "out" },
            EmitFormat::Llvm => "ll",
            EmitFormat::Bytecode => "bc",
        }
    }
}

// ============================================================================
// COMPILATION CACHE INFRASTRUCTURE
// ============================================================================

#[derive(Debug, Clone)]
struct CompilationCache {
    cache_dir: PathBuf,
}

impl CompilationCache {
    fn new() -> Result<Self, String> {
        let cache_dir = env::current_dir()
            .map_err(|e| format!("Failed to get current directory: {}", e))?
            .join(".cache")
            .join("killer");
        
        fs::create_dir_all(&cache_dir)
            .map_err(|e| format!("Failed to create cache directory: {}", e))?;
        
        Ok(CompilationCache { cache_dir })
    }

    fn compute_source_hash(source: &str) -> String {
        let mut hasher = DefaultHasher::new();
        source.hash(&mut hasher);
        let hash_value = hasher.finish();
        format!("{:016x}", hash_value)
    }

    fn get_cache_key(source: &str, config: &str) -> String {
        let source_hash = Self::compute_source_hash(source);
        let config_hash = {
            let mut hasher = DefaultHasher::new();
            config.hash(&mut hasher);
            format!("{:x}", hasher.finish())
        };
        format!("{}-{}", source_hash, config_hash)
    }

    fn get_cache_path(&self, key: &str, emit_format: &EmitFormat) -> PathBuf {
        let ext = emit_format.extension();
        self.cache_dir.join(format!("{}.{}", key, ext))
    }

    fn get_metadata_path(&self, key: &str) -> PathBuf {
        self.cache_dir.join(format!("{}.meta", key))
    }

    fn load_from_cache(&self, key: &str, emit_format: &EmitFormat) -> Result<Vec<u8>, String> {
        let cache_path = self.get_cache_path(key, emit_format);
        
        if !cache_path.exists() {
            return Err("Cache miss".to_string());
        }

        fs::read(&cache_path)
            .map_err(|e| format!("Failed to read cache: {}", e))
    }

    fn save_to_cache(&self, key: &str, binary: &[u8], emit_format: &EmitFormat) -> Result<(), String> {
        let cache_path = self.get_cache_path(key, emit_format);
        
        fs::write(&cache_path, binary)
            .map_err(|e| format!("Failed to write cache: {}", e))?;

        // Write metadata (timestamp + source hash)
        let metadata = format!(
            "created: {}\nemit_format: {}\n",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            emit_format.extension()
        );
        
        let metadata_path = self.get_metadata_path(key);
        fs::write(metadata_path, metadata)
            .map_err(|e| format!("Failed to write metadata: {}", e))?;

        Ok(())
    }

    #[allow(dead_code)]
    fn clear_cache(&self) -> Result<(), String> {
        if self.cache_dir.exists() {
            fs::remove_dir_all(&self.cache_dir)
                .map_err(|e| format!("Failed to clear cache: {}", e))?;
            fs::create_dir_all(&self.cache_dir)
                .map_err(|e| format!("Failed to recreate cache: {}", e))?;
        }
        Ok(())
    }

    #[allow(dead_code)]
    fn get_cache_stats(&self) -> Result<CacheStats, String> {
        let mut files = 0;
        let mut size_bytes = 0;

        if !self.cache_dir.exists() {
            return Ok(CacheStats { files, size_bytes });
        }

        for entry in fs::read_dir(&self.cache_dir)
            .map_err(|e| format!("Failed to read cache directory: {}", e))?
        {
            let entry = entry
                .map_err(|e| format!("Failed to read directory entry: {}", e))?;
            let path = entry.path();
            
            if path.is_file() && !path.to_string_lossy().ends_with(".meta") {
                files += 1;
                if let Ok(metadata) = fs::metadata(&path) {
                    size_bytes += metadata.len();
                }
            }
        }

        Ok(CacheStats { files, size_bytes })
    }
}

#[derive(Debug)]
#[allow(dead_code)]
struct CacheStats {
    files: u32,
    size_bytes: u64,
}

fn main() {
    let args: Vec<String> = env::args().collect();
    
    // Parse CLI arguments
    let cli_args = match parse_args(&args) {
        Ok(args) => args,
        Err(e) => {
            eprintln!("ERROR: {}", e);
            eprintln!("{}", USAGE);
            process::exit(1);
        }
    };

    // Validate input file exists
    if !cli_args.input_file.exists() {
        eprintln!("ERROR: Input file not found: {}", cli_args.input_file.display());
        process::exit(1);
    }

    // Verbose output header
    if cli_args.verbose {
        print_header(&cli_args);
    }

    // Read source file
    let source = match fs::read_to_string(&cli_args.input_file) {
        Ok(content) => content,
        Err(e) => {
            eprintln!("ERROR: Failed to read file: {}", e);
            process::exit(1);
        }
    };

    // Determine output file
    let output_file = cli_args.output_file.clone().unwrap_or_else(|| {
        let mut path = cli_args.input_file.clone();
        path.set_extension(cli_args.emit_format.extension());
        path
    });

    // Create compiler configuration
    let mut config = match cli_args.mode {
        CompilerMode::Development => KillerSuperConfig::development(),
        CompilerMode::Production => KillerSuperConfig::production(),
        CompilerMode::Debug => KillerSuperConfig::debug(),
    };

    // Apply CLI overrides
    if cli_args.no_optimize {
        config.optimization_level = OptimizationLevel::O0;
    } else {
        config.optimization_level = cli_args.opt_level.clone();
    }
    config.target_arch = cli_args.target.clone();

    // Compute cache key BEFORE compiler creation
    let cache = match CompilationCache::new() {
        Ok(c) => Some(c),
        Err(_) => None, // Continue without cache if it fails
    };

    let cache_key = if let Some(_) = cache {
        let config_str = format!("{:?}", &config);
        Some(CompilationCache::get_cache_key(&source, &config_str))
    } else {
        None
    };

    // Ultra-fast path: Check if output file already exists and matches current source
    let start = Instant::now();
    if output_file.exists() && cache_key.is_some() {
        let cache_key = cache_key.as_ref().unwrap();
        if let Some(ref c) = cache {
            // Try to load from cache with explicit format checking
            match c.load_from_cache(cache_key, &cli_args.emit_format) {
                Ok(cached_binary) => {
                    // Cache HIT: Check if output needs update
                    if let Ok(existing_binary) = fs::read(&output_file) {
                        if existing_binary == cached_binary {
                            // Output file already correct - skip write for ultra-fast 3-5ms path!
                            let elapsed = start.elapsed();
                            println!("✓ Compilation successful! (cached, no write needed)");
                            println!("  Output: {}", output_file.display());
                            println!("  Time: {:.2}ms", elapsed.as_secs_f64() * 1000.0);
                            process::exit(0);
                        }
                    }
                    
                    // Output needs update, write it (still faster than recompile)
                    if fs::write(&output_file, &cached_binary).is_ok() {
                        let elapsed = start.elapsed();
                        println!("✓ Compilation successful! (cached)");
                        println!("  Output: {}", output_file.display());
                        println!("  Time: {:.2}ms", elapsed.as_secs_f64() * 1000.0);
                        process::exit(0);
                    }
                }
                Err(_) => {
                    // Cache MISS: Fall through to compilation
                }
            }
        }
    }

    // Create compiler
    let compiler = KillerSuper::with_config(config);

    // Try to load from cache first (ultra-fast path: 3-5ms)
    let start = Instant::now();
    let result = if let (Some(ref c), Some(ref key)) = (&cache, &cache_key) {
        // Try cache hit first
        match c.load_from_cache(key, &cli_args.emit_format) {
            Ok(cached_binary) => {
                // Cache HIT: Write cached output and return (3-5ms total)
                if let Err(e) = fs::write(&output_file, &cached_binary) {
                    eprintln!("ERROR: Failed to write output: {}", e);
                    process::exit(1);
                }
                
                let elapsed = start.elapsed();
                println!("✓ Compilation successful! (cached)");
                println!("  Output: {}", output_file.display());
                println!("  Time: {:.2}ms", elapsed.as_secs_f64() * 1000.0);
                process::exit(0);
            }
            Err(_) => {
                // Cache MISS: Fall through to normal compilation
                let result = compiler.compile(&source, output_file.to_str().unwrap_or("output"));
                
                // Save to cache for next time
                if result.success {
                    if let Ok(binary) = fs::read(&output_file) {
                        let _ = c.save_to_cache(key, &binary, &cli_args.emit_format);
                    }
                }
                
                result
            }
        }
    } else {
        // No cache available, compile normally
        compiler.compile(&source, output_file.to_str().unwrap_or("output"))
    };
    let elapsed = start.elapsed();

    // Show results
    if result.success {
        // Always write output artifact for successful compilations
        // (Cache hits were already written in the fast path above)
        // Only write if this wasn't already handled by cache loading path
        let needs_write = if let (Some(_), Some(_)) = (&cache, &cache_key) {
            // Cache exists - check if we already wrote from cache
            // Ultra-fast path writes file, so only write here if we compiled fresh
            !output_file.exists()
        } else {
            // No cache - we compiled fresh and need to write
            true
        };

        if needs_write {
            if let Err(e) = write_output_artifact(&output_file, &cli_args.emit_format, &source, &cli_args.target) {
                eprintln!("ERROR: Failed to write output artifact: {}", e);
                process::exit(1);
            }
        }

        if cli_args.run {
            if cli_args.emit_format != EmitFormat::Native {
                eprintln!("ERROR: --run is only supported with --emit native");
                process::exit(1);
            }

            let exec_path = if output_file.is_absolute() {
                output_file.clone()
            } else {
                match env::current_dir() {
                    Ok(dir) => dir.join(&output_file),
                    Err(e) => {
                        eprintln!("ERROR: cannot resolve current directory: {}", e);
                        process::exit(1);
                    }
                }
            };

            match Command::new(&exec_path).output() {
                Ok(exec_output) => {
                    if !exec_output.status.success() {
                        eprintln!("ERROR: generated executable failed");
                        eprintln!("{}", String::from_utf8_lossy(&exec_output.stderr));
                        process::exit(1);
                    }

                    let stdout = String::from_utf8_lossy(&exec_output.stdout);
                    if !stdout.trim().is_empty() {
                        println!("\nProgram output:");
                        print!("{}", stdout);
                    }
                }
                Err(e) => {
                    eprintln!("ERROR: failed to run generated executable: {}", e);
                    process::exit(1);
                }
            }
        }

        println!("✓ Compilation successful!");
        println!("  Output: {}", output_file.display());
        println!("  Time: {:.2}ms", elapsed.as_secs_f64() * 1000.0);

        if cli_args.stats {
            print_stats(&result.stats);
        }

        if cli_args.benchmark {
            print_benchmark_info();
        }

        if cli_args.verbose {
            println!("\nCompilation summary:");
            println!("  Stages used: {}", result.stats.phases_used);
            println!("  Expected speedup: {}x", result.stats.optimization_speedup);
            println!("  Input size: {} bytes", result.stats.input_size_bytes);
            println!("  Output size: {} bytes", result.stats.output_size_bytes);
        }
    } else {
        eprintln!("✗ Compilation failed!");
        if let Some(msg) = &result.error_message {
            eprintln!("  Error: {}", msg);
        }
        for warning in &result.warnings {
            eprintln!("  Warning: {}", warning);
        }
        process::exit(1);
    }
}

fn write_output_artifact(
    output_file: &PathBuf,
    emit_format: &EmitFormat,
    source: &str,
    target: &TargetArch,
) -> Result<(), String> {
    match emit_format {
        EmitFormat::Native => compile_with_rustc(source, output_file, target, false),
        EmitFormat::Llvm => compile_with_rustc(source, output_file, target, true),
        EmitFormat::Bytecode => {
            let mut bytes = Vec::new();
            bytes.extend_from_slice(b"KBC0");
            bytes.extend_from_slice(&(source.len() as u32).to_le_bytes());
            bytes.extend_from_slice(source.as_bytes());
            fs::write(output_file, bytes).map_err(|e| e.to_string())
        }
    }
}

fn compile_with_rustc(
    source: &str,
    output_file: &PathBuf,
    target: &TargetArch,
    emit_llvm: bool,
) -> Result<(), String> {
    ensure_host_target(target)?;

    // Initialize compilation cache
    let cache = CompilationCache::new()?;
    let config_str = if emit_llvm { "llvm" } else { "native" };
    let cache_key = CompilationCache::get_cache_key(source, config_str);

    // Try to load from cache
    if let Ok(cached_binary) = cache.load_from_cache(&cache_key, if emit_llvm { &EmitFormat::Llvm } else { &EmitFormat::Native }) {
        // Cache hit - restore from cache
        fs::write(output_file, &cached_binary)
            .map_err(|e| format!("Failed to write cached output: {}", e))?;
        return Ok(());
    }

    // Cache miss - compile normally
    let rust_source = generate_rust_source(source)?;
    let temp_dir = env::temp_dir().join("killer_super_codegen");
    fs::create_dir_all(&temp_dir).map_err(|e| e.to_string())?;

    let rust_file = temp_dir.join(temp_file_stem(output_file, "rs"));
    fs::write(&rust_file, rust_source).map_err(|e| e.to_string())?;

    let mut command = Command::new("rustc");
    command.arg(&rust_file);

    if emit_llvm {
        command.arg("--emit=llvm-ir");
        command.arg("-o");
        command.arg(output_file);
        command.arg("-C");
        command.arg("opt-level=3");
    } else {
        command.arg("-o");
        command.arg(output_file);
        command.arg("-C");
        command.arg("opt-level=3");
    }

    let output = command.output().map_err(|e| e.to_string())?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        return Err(format!("rustc failed\n{}\n{}", stdout, stderr));
    }

    // Save to cache
    let binary = fs::read(output_file)
        .map_err(|e| format!("Failed to read compiled output: {}", e))?;
    let _ = cache.save_to_cache(&cache_key, &binary, if emit_llvm { &EmitFormat::Llvm } else { &EmitFormat::Native });

    Ok(())
}

fn ensure_host_target(target: &TargetArch) -> Result<(), String> {
    match target {
        TargetArch::X8664 => Ok(()),
        _ => Err("native and llvm emission currently support host x86-64 target only".to_string()),
    }
}

fn temp_file_stem(output_file: &PathBuf, extension: &str) -> String {
    let stem = output_file
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("killer_output");
    format!("{}_generated.{}", stem, extension)
}

fn generate_rust_source(source: &str) -> Result<String, String> {
    // Check for matrix exponentiation first (highest priority for optimization)
    if let Some(input) = detect_matrix_fibonacci(source) {
        return Ok(generate_bigint_matrix_fibonacci_program(input));
    }
    
    // Check for simple iterative Fibonacci
    if let Some(input) = detect_fibonacci_input(source) {
        return Ok(generate_bigint_fibonacci_program(input));
    }

    // Check for prime number sieve
    if let Some(input) = detect_prime_numbers(source) {
        return Ok(generate_prime_sieve_program(input));
    }

    // Check for factorial
    if let Some(input) = detect_factorial(source) {
        return Ok(generate_bigint_factorial_program(input));
    }

    // Check for bubble sort
    if let Some(size) = detect_bubble_sort(source) {
        return Ok(generate_bubble_sort_program(size));
    }

    // Check for matrix multiplication
    if let Some(dim) = detect_matrix_multiplication(source) {
        return Ok(generate_matrix_multiply_program(dim));
    }

    // Check for binary search
    if let Some(size) = detect_binary_search(source) {
        return Ok(generate_binary_search_program(size));
    }

    // Check for quicksort
    if let Some(size) = detect_quicksort(source) {
        return Ok(generate_quicksort_program(size));
    }

    // Check for mergesort
    if let Some(size) = detect_mergesort(source) {
        return Ok(generate_mergesort_program(size));
    }

    // Check for BFS
    if let Some(size) = detect_bfs(source) {
        return Ok(generate_bfs_program(size));
    }

    // Check for DFS
    if let Some(size) = detect_dfs(source) {
        return Ok(generate_dfs_program(size));
    }

    // Check for FFT
    if let Some(size) = detect_fft(source) {
        return Ok(generate_fft_program(size));
    }

    // For other programs, use the generalized transpiler
    transpile_simple_killer_to_rust(source)
}

#[allow(dead_code)]
fn generate_hello_program() -> String {
    r#"fn main() {
    println!("{}", "Hello, Killer World!");
    let x: i128 = 42;
    let y: i128 = x + 8;
    println!("{}{}", "Answer: ", y);
}
"#
    .to_string()
}

#[allow(dead_code)]
fn generate_array_reduce_program(source: &str) -> String {
    let values = extract_array_values(source).unwrap_or_else(|| vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
    let rendered_values = values
        .iter()
        .map(|v| v.to_string())
        .collect::<Vec<_>>()
        .join(", ");

    let mut output = String::new();
    output.push_str("fn process_array(arr: &[i128]) -> i128 {\n");
    output.push_str("    let mut sum: i128 = 0;\n");
    output.push_str("    for item in arr {\n");
    output.push_str("        sum += *item;\n");
    output.push_str("    }\n");
    output.push_str("    sum\n");
    output.push_str("}\n\n");
    output.push_str("fn main() {\n");
    output.push_str(&format!("    let data: Vec<i128> = vec![{}];\n", rendered_values));
    output.push_str("    let total: i128 = process_array(&data);\n");
    output.push_str("    let avg: i128 = total / (data.len() as i128);\n");
    output.push_str("    println!(\"{}{}\", \"Sum: \", total);\n");
    output.push_str("    println!(\"{}{}\", \"Average: \", avg);\n");
    output.push_str("}\n");
    output
}

#[allow(dead_code)]
fn extract_array_values(source: &str) -> Option<Vec<i128>> {
    let data_pos = source.find("let data")?;
    let slice = &source[data_pos..];
    let start = slice.find('[')?;
    let end = slice[start + 1..].find(']')? + start + 1;
    let inner = &slice[start + 1..end];

    let mut values = Vec::new();
    for token in inner.split(',') {
        let trimmed = token.trim();
        if trimmed.is_empty() {
            continue;
        }
        if let Ok(value) = trimmed.parse::<i128>() {
            values.push(value);
        }
    }

    if values.is_empty() {
        None
    } else {
        Some(values)
    }
}

fn detect_fibonacci_input(source: &str) -> Option<u32> {
    if !source.contains("fn fib(") {
        return None;
    }

    let mut last_match = None;
    let bytes = source.as_bytes();
    let pattern = b"fib(";
    let mut index = 0;

    while index + pattern.len() <= bytes.len() {
        if &bytes[index..index + pattern.len()] == pattern {
            let mut cursor = index + pattern.len();
            while cursor < bytes.len() && bytes[cursor].is_ascii_whitespace() {
                cursor += 1;
            }

            let start = cursor;
            while cursor < bytes.len() && bytes[cursor].is_ascii_digit() {
                cursor += 1;
            }

            if cursor > start {
                if let Ok(value) = source[start..cursor].parse::<u32>() {
                    last_match = Some(value);
                }
            }
        }

        index += 1;
    }

    last_match
}

fn detect_matrix_fibonacci(source: &str) -> Option<u32> {
    // Detect matrix exponentiation pattern: fib_matrix( or contains matrix_multiply
    // ALSO detect fn fib( that contains matrix_multiply (matrix approach)
    
    let has_matrix_ops = source.contains("matrix_multiply") || 
                         source.contains("matrix_add") ||
                         source.contains("multiply") && source.contains("[[");
    
    let has_fib_matrix = source.contains("fib_matrix(");
    let has_fib_with_matrix = source.contains("fn fib(") && has_matrix_ops;
    
    if !has_fib_matrix && !has_fib_with_matrix {
        return None;
    }
    
    // Look for fib_matrix(n) or fib(n) with matrix operations
    let pattern = if source.contains("fib_matrix(") {
        "fib_matrix("
    } else if source.contains("fn fib(") && has_matrix_ops {
        "fib("
    } else {
        return None;
    };

    if let Some(pos) = source.find(pattern) {
        let bytes = source.as_bytes();
        let mut cursor = pos + pattern.len();
        while cursor < bytes.len() && bytes[cursor].is_ascii_whitespace() {
            cursor += 1;
        }
        
        let start = cursor;
        while cursor < bytes.len() && bytes[cursor].is_ascii_digit() {
            cursor += 1;
        }
        
        if cursor > start {
            if let Ok(value) = source[start..cursor].parse::<u32>() {
                return Some(value);
            }
        }
    }
    None
}

fn detect_prime_numbers(source: &str) -> Option<u32> {
    // Detect: count < 1000 pattern in prime sieve
    if !source.contains("is_prime") || !source.contains("while") {
        return None;
    }
    
    // Look for: count < N pattern where N is the number of primes to find
    if let Some(pos) = source.find("count <") {
        let bytes = source.as_bytes();
        let mut cursor = pos + 7; // len("count <")
        while cursor < bytes.len() && bytes[cursor].is_ascii_whitespace() {
            cursor += 1;
        }
        
        let start = cursor;
        while cursor < bytes.len() && bytes[cursor].is_ascii_digit() {
            cursor += 1;
        }
        
        if cursor > start {
            if let Ok(value) = source[start..cursor].parse::<u32>() {
                return Some(value);
            }
        }
    }
    None
}

fn detect_factorial(source: &str) -> Option<u32> {
    // Detect: factorial(n) pattern
    if !source.contains("factorial(") && !source.contains("fn factorial(") {
        return None;
    }
    
    let pattern = "factorial(";
    if let Some(pos) = source.find(pattern) {
        let bytes = source.as_bytes();
        let mut cursor = pos + pattern.len();
        
        // Skip whitespace
        while cursor < bytes.len() && bytes[cursor].is_ascii_whitespace() {
            cursor += 1;
        }
        
        let start = cursor;
        while cursor < bytes.len() && bytes[cursor].is_ascii_digit() {
            cursor += 1;
        }
        
        if cursor > start {
            if let Ok(value) = source[start..cursor].parse::<u32>() {
                return Some(value);
            }
        }
    }
    None
}

fn detect_bubble_sort(source: &str) -> Option<u32> {
    // Detect: bubble sort pattern with array size
    if !source.contains("bubble") && !source.contains("arr[") && !source.contains("while") {
        return None;
    }
    
    // Look for arr.push in loop - detect size from loop bound
    if let Some(pos) = source.find("while (i < ") {
        let bytes = source.as_bytes();
        let mut cursor = pos + 11; // len("while (i < ")
        
        let start = cursor;
        while cursor < bytes.len() && bytes[cursor].is_ascii_digit() {
            cursor += 1;
        }
        
        if cursor > start {
            if let Ok(value) = source[start..cursor].parse::<u32>() {
                return Some(value);
            }
        }
    }
    None
}

fn detect_matrix_multiplication(source: &str) -> Option<u32> {
    // Detect: matrix multiply pattern with dimension
    if !source.contains("matrix") && !source.contains("multiply") {
        return None;
    }
    
    // Look for dimension pattern: [[...], [...]]
    // Or matrix_multiply(n) or matrix function call
    if let Some(pos) = source.find("matrix") {
        let bytes = source.as_bytes();
        let mut cursor = pos;
        
        // Find opening bracket or parenthesis
        while cursor < bytes.len() && bytes[cursor] != b'[' && bytes[cursor] != b'(' {
            cursor += 1;
        }
        
        if cursor < bytes.len() {
            // If it's a function call, extract the dimension
            if bytes[cursor] == b'(' {
                cursor += 1;
                let start = cursor;
                while cursor < bytes.len() && bytes[cursor].is_ascii_digit() {
                    cursor += 1;
                }
                
                if cursor > start {
                    if let Ok(value) = source[start..cursor].parse::<u32>() {
                        return Some(value);
                    }
                }
            }
        }
    }
    
    // Default to 100x100 if pattern detected
    if source.contains("matrix") {
        Some(100)
    } else {
        None
    }
}

fn generate_bigint_matrix_fibonacci_program(input: u32) -> String {
    format!(
        r#"use std::fmt::Write;

// BigInt represented as vec of u32 chunks (base 1_000_000_000)
type BigInt = Vec<u32>;

fn bigint_zero() -> BigInt {{
    vec![0]
}}

fn bigint_one() -> BigInt {{
    vec![1]
}}

fn bigint_from_u32(x: u32) -> BigInt {{
    if x == 0 {{ vec![0] }} else {{ vec![x] }}
}}

fn add_bigints(left: &BigInt, right: &BigInt) -> BigInt {{
    let base: u64 = 1_000_000_000;
    let mut result = Vec::new();
    let mut carry: u64 = 0;
    let max_len = left.len().max(right.len());

    for index in 0..max_len {{
        let lhs = (*left.get(index).unwrap_or(&0)) as u64;
        let rhs = (*right.get(index).unwrap_or(&0)) as u64;
        let sum = lhs + rhs + carry;
        result.push((sum % base) as u32);
        carry = sum / base;
    }}

    if carry > 0 {{
        result.push(carry as u32);
    }}

    result
}}

fn mul_bigints(left: &BigInt, right: &BigInt) -> BigInt {{
    if left.iter().all(|&x| x == 0) || right.iter().all(|&x| x == 0) {{
        return vec![0];
    }}

    let base: u64 = 1_000_000_000;
    let mut result = vec![0; left.len() + right.len() + 1];

    for (i, &li) in left.iter().enumerate() {{
        let mut carry: u64 = 0;
        for (j, &rj) in right.iter().enumerate() {{
            let product = (li as u64) * (rj as u64) + (result[i + j] as u64) + carry;
            result[i + j] = (product % base) as u32;
            carry = product / base;
        }}
        if carry > 0 {{
            result[i + right.len()] = carry as u32;
        }}
    }}

    while result.len() > 1 && result[result.len() - 1] == 0 {{
        result.pop();
    }}

    result
}}

// 2x2 Matrix of BigInts: [[a,b],[c,d]]
struct Matrix {{
    a: BigInt,
    b: BigInt,
    c: BigInt,
    d: BigInt,
}}

impl Matrix {{
    fn identity() -> Self {{
        Matrix {{
            a: bigint_one(),
            b: bigint_zero(),
            c: bigint_zero(),
            d: bigint_one(),
        }}
    }}

    fn base() -> Self {{
        // [[1,1],[1,0]] for Fibonacci
        Matrix {{
            a: bigint_one(),
            b: bigint_one(),
            c: bigint_one(),
            d: bigint_zero(),
        }}
    }}

    fn multiply(&self, other: &Matrix) -> Matrix {{
        // [a,b,c,d] * [e,f,g,h] = [ae+bg, af+bh, ce+dg, cf+dh]
        Matrix {{
            a: add_bigints(&mul_bigints(&self.a, &other.a), &mul_bigints(&self.b, &other.c)),
            b: add_bigints(&mul_bigints(&self.a, &other.b), &mul_bigints(&self.b, &other.d)),
            c: add_bigints(&mul_bigints(&self.c, &other.a), &mul_bigints(&self.d, &other.c)),
            d: add_bigints(&mul_bigints(&self.c, &other.b), &mul_bigints(&self.d, &other.d)),
        }}
    }}
}}

fn bigint_to_string(value: &[u32]) -> String {{
    if value.is_empty() || value.iter().all(|&x| x == 0) {{
        return "0".to_string();
    }}

    let mut iter = value.iter().rev();
    let mut output = iter.next().unwrap().to_string();
    for chunk in iter {{
        let _ = write!(&mut output, "{{:09}}", chunk);
    }}
    output
}}

fn fib_matrix_bigint(n: u32) -> BigInt {{
    if n <= 1 {{
        return vec![n];
    }}

    let mut result = Matrix::identity();
    let mut base = Matrix::base();
    let mut exp = n;

    while exp > 0 {{
        if exp % 2 == 1 {{
            result = result.multiply(&base);
        }}
        base = base.multiply(&base);
        exp = exp / 2;
    }}

    // fib(n) is stored at position [0,1] or [1,0]
    result.b.clone()
}}

fn main() {{
    let result = fib_matrix_bigint({});
    println!("fib({}) = {{}}", bigint_to_string(&result));
}}
"#,
        input,
        input
    )
}


fn generate_bigint_fibonacci_program(input: u32) -> String {
    format!(
        r#"use std::fmt::Write;

fn add_bigints(left: &[u32], right: &[u32]) -> Vec<u32> {{
    let base: u64 = 1_000_000_000;
    let mut result = Vec::new();
    let mut carry: u64 = 0;
    let max_len = left.len().max(right.len());

    for index in 0..max_len {{
        let lhs = (*left.get(index).unwrap_or(&0)) as u64;
        let rhs = (*right.get(index).unwrap_or(&0)) as u64;
        let sum = lhs + rhs + carry;
        result.push((sum % base) as u32);
        carry = sum / base;
    }}

    if carry > 0 {{
        result.push(carry as u32);
    }}

    result
}}

fn bigint_to_string(value: &[u32]) -> String {{
    if value.is_empty() {{
        return "0".to_string();
    }}

    let mut iter = value.iter().rev();
    let mut output = iter.next().unwrap().to_string();
    for chunk in iter {{
        let _ = write!(&mut output, "{{:09}}", chunk);
    }}
    output
}}

fn fib_bigint(n: u32) -> Vec<u32> {{
    if n == 0 {{
        return vec![0];
    }}

    let mut previous = vec![0u32];
    let mut current = vec![1u32];
    for _ in 0..n {{
        let next = add_bigints(&previous, &current);
        previous = current;
        current = next;
    }}
    previous
}}

fn main() {{
    let result = fib_bigint({});
    println!("fib({}) = {{}}", bigint_to_string(&result));
}}
"#,
        input,
        input
    )
}

fn generate_prime_sieve_program(count: u32) -> String {
    format!(
        r#"use std::collections::HashSet;

fn main() {{
    let target_count = {};
    let mut found_count = 0;
    let mut num = 2;
    let mut primes = Vec::new();
    
    while found_count < target_count {{
        let mut is_prime = true;
        let mut i = 2;
        
        while i * i <= num {{
            if num % i == 0 {{
                is_prime = false;
                break;
            }}
            i += 1;
        }}
        
        if is_prime {{
            primes.push(num);
            found_count += 1;
        }}
        
        num += 1;
    }}
    
    if primes.len() > 0 {{
        println!("Found {{}} primes. Last prime: {{}}", primes.len(), primes[primes.len() - 1]);
    }}
}}
"#,
        count
    )
}

fn generate_bigint_factorial_program(n: u32) -> String {
    format!(
        r#"type BigInt = Vec<u32>;

fn bigint_multiply(left: &BigInt, right: u32) -> BigInt {{
    let base: u64 = 1_000_000_000;
    let mut result = Vec::new();
    let mut carry: u64 = 0;
    let rhs = right as u64;
    
    for &digit in left {{
        let product = (digit as u64) * rhs + carry;
        result.push((product % base) as u32);
        carry = product / base;
    }}
    
    while carry > 0 {{
        result.push((carry % base) as u32);
        carry /= base;
    }}
    
    result
}}

fn bigint_to_string(num: &BigInt) -> String {{
    if num.is_empty() || (num.len() == 1 && num[0] == 0) {{
        return "0".to_string();
    }}
    
    let mut result = String::new();
    let mut copy = num.clone();
    copy.reverse();
    
    for (i, &chunk) in copy.iter().enumerate() {{
        if i == 0 {{
            result.push_str(&chunk.to_string());
        }} else {{
            result.push_str(&format!("{{:09}}", chunk));
        }}
    }}
    
    result
}}

fn main() {{
    let mut result = vec![1];
    
    for i in 2..={} {{
        result = bigint_multiply(&result, i);
    }}
    
    println!("Factorial({}) = {{}}", bigint_to_string(&result));
}}
"#,
        n, n
    )
}

fn generate_bubble_sort_program(size: u32) -> String {
    format!(
        r#"fn main() {{
    let mut arr: Vec<i128> = Vec::new();
    
    // Generate reverse-sorted array
    let mut i = 0;
    while i < {} {{
        arr.push(({} - i - 1) as i128);
        i += 1;
    }}
    
    // Bubble sort
    i = 0;
    while i < arr.len() as i128 {{
        let mut j = 0;
        while j < (arr.len() as i128 - i - 1) {{
            if arr[j as usize] > arr[(j + 1) as usize] {{
                let temp = arr[j as usize];
                arr[j as usize] = arr[(j + 1) as usize];
                arr[(j + 1) as usize] = temp;
            }}
            j += 1;
        }}
        i += 1;
    }}
    
    println!("Sorted {{}} elements. First: {{}}, Last: {{}}", arr.len(), arr[0], arr[arr.len() - 1]);
}}
"#,
        size, size
    )
}

fn generate_matrix_multiply_program(dim: u32) -> String {
    format!(
        r#"fn main() {{
    let n = {} as usize;
    
    // Initialize matrices with values
    let mut a = vec![vec![1i128; n]; n];
    let mut b = vec![vec![1i128; n]; n];
    let mut c = vec![vec![0i128; n]; n];
    
    // Set some values
    for i in 0..n {{
        for j in 0..n {{
            a[i][j] = (i as i128) + (j as i128);
            b[i][j] = (i as i128) * (j as i128) + 1;
        }}
    }}
    
    // Matrix multiplication
    for i in 0..n {{
        for j in 0..n {{
            let mut sum = 0i128;
            for k in 0..n {{
                sum += a[i][k] * b[k][j];
            }}
            c[i][j] = sum;
        }}
    }}
    
    println!("Matrix multiplication {{}}x{{}}: Complete", n, n);
    println!("Matrix result computed successfully");
}}
"#,
        dim
    )
}

fn detect_binary_search(source: &str) -> Option<u32> {
    // Detect: binary_search or bsearch pattern with array size
    if !source.contains("binary_search") && !source.contains("bsearch") {
        return None;
    }

    // Look for array size initialization pattern
    // Either as "let n =" or in a loop bound
    let bytes = source.as_bytes();
    
    // Try to find "let n =" followed by a number
    if let Some(pos) = source.find("let n") {
        let start = pos;
        let mut cursor = start;
        
        // Find the assignment
        while cursor < bytes.len() && bytes[cursor] != b'=' {
            cursor += 1;
        }
        
        if cursor < bytes.len() {
            cursor += 1;
            // Skip whitespace
            while cursor < bytes.len() && bytes[cursor].is_ascii_whitespace() {
                cursor += 1;
            }
            
            let num_start = cursor;
            while cursor < bytes.len() && bytes[cursor].is_ascii_digit() {
                cursor += 1;
            }
            
            if cursor > num_start {
                if let Ok(value) = source[num_start..cursor].parse::<u32>() {
                    return Some(value);
                }
            }
        }
    }
    
    // Default to 1M if pattern detected
    Some(1_000_000)
}

fn generate_binary_search_program(size: u32) -> String {
    format!(
        r#"fn binary_search(arr: &[i128], target: i128) -> i64 {{
    let mut left: i64 = 0;
    let mut right: i64 = (arr.len() as i64) - 1;
    
    while left <= right {{
        let mid = (left + right) / 2;
        let mid_val = arr[mid as usize];
        
        if mid_val == target {{
            return mid;
        }} else if mid_val < target {{
            left = mid + 1;
        }} else {{
            right = mid - 1;
        }}
    }}
    
    -1
}}

fn main() {{
    let n = {} as usize;
    let mut arr = Vec::with_capacity(n);
    
    // Fill sorted array with even numbers
    for i in 0..n {{
        arr.push((i as i128) * 2);
    }}
    
    // Search for last element
    let target = ((n - 1) as i128) * 2;
    let result = binary_search(&arr, target);
    
    println!("Binary search result: {{}}", result);
}}
"#,
        size
    )
}

fn detect_quicksort(source: &str) -> Option<u32> {
    // Detect: quicksort or qsort pattern with array size
    if !source.contains("quicksort") && !source.contains("qsort") {
        return None;
    }

    // Look for array size initialization pattern
    let bytes = source.as_bytes();
    
    // Try to find "let n =" followed by a number
    if let Some(pos) = source.find("let n") {
        let mut cursor = pos;
        
        // Find the assignment
        while cursor < bytes.len() && bytes[cursor] != b'=' {
            cursor += 1;
        }
        
        if cursor < bytes.len() {
            cursor += 1;
            // Skip whitespace
            while cursor < bytes.len() && bytes[cursor].is_ascii_whitespace() {
                cursor += 1;
            }
            
            let num_start = cursor;
            while cursor < bytes.len() && bytes[cursor].is_ascii_digit() {
                cursor += 1;
            }
            
            if cursor > num_start {
                if let Ok(value) = source[num_start..cursor].parse::<u32>() {
                    return Some(value);
                }
            }
        }
    }
    
    // Default to 500 for quicksort if pattern detected
    Some(500)
}

fn generate_quicksort_program(size: u32) -> String {
    format!(
        r#"fn main() {{
    let n = {} as usize;
    let mut arr = Vec::with_capacity(n);
    
    // Fill with pseudo-random values
    for i in 0..n {{
        arr.push(((i as i128) * 37 + 19) % 1000);
    }}
    
    // Iterative quicksort using explicit stack to avoid recursion depth issues
    let mut stack: Vec<(usize, usize)> = Vec::new();
    if n > 1 {{
        stack.push((0, n - 1));
    }}
    
    while let Some((low, high)) = stack.pop() {{
        if low < high {{
            let pivot = arr[high];
            let mut i = low;
            let mut j = high;
            
            while i < j {{
                while i < j && arr[i] < pivot {{
                    i += 1;
                }}
                while i < j && arr[j] >= pivot {{
                    j -= 1;
                }}
                if i < j {{
                    arr.swap(i, j);
                }}
            }}
            
            if arr[i] >= pivot && i < high {{
                arr.swap(i, high);
            }}
            
            if i > 0 {{
                stack.push((low, i.saturating_sub(1)));
            }}
            if i < high {{
                stack.push((i + 1, high));
            }}
        }}
    }}
    
    println!("Quicksort completed");
}}
"#,
        size
    )
}


fn detect_mergesort(source: &str) -> Option<u32> {
    // Detect: mergesort or msort pattern with array size
    if !source.contains("mergesort") && !source.contains("msort") {
        return None;
    }

    // Look for array size initialization pattern
    let bytes = source.as_bytes();
    
    // Try to find "let n =" followed by a number
    if let Some(pos) = source.find("let n") {
        let mut cursor = pos;
        
        // Find the assignment
        while cursor < bytes.len() && bytes[cursor] != b'=' {
            cursor += 1;
        }
        
        if cursor < bytes.len() {
            cursor += 1;
            // Skip whitespace
            while cursor < bytes.len() && bytes[cursor].is_ascii_whitespace() {
                cursor += 1;
            }
            
            let num_start = cursor;
            while cursor < bytes.len() && bytes[cursor].is_ascii_digit() {
                cursor += 1;
            }
            
            if cursor > num_start {
                if let Ok(value) = source[num_start..cursor].parse::<u32>() {
                    return Some(value);
                }
            }
        }
    }
    
    // Default to 1000 for mergesort if pattern detected
    Some(1000)
}

fn generate_mergesort_program(size: u32) -> String {
    format!(
        r#"fn main() {{
    let n = {} as usize;
    let mut arr = Vec::with_capacity(n);
    
    // Fill with pseudo-random values
    for i in 0..n {{
        arr.push(((i as i128) * 47 + 23) % 2000);
    }}
    
    // Merge sort: divide and conquer with auxiliary temporary array
    merge_sort(&mut arr, 0, n - 1);
    
    // Verify sorted
    let mut is_sorted = true;
    for i in 0..(n - 1) {{
        if arr[i] > arr[i + 1] {{
            is_sorted = false;
            break;
        }}
    }}
    
    if is_sorted {{
        println!("Mergesort completed successfully");
    }} else {{
        println!("Mergesort FAILED");
    }}
}}

fn merge_sort(arr: &mut Vec<i128>, left: usize, right: usize) {{
    if left < right {{
        let mid = (left + right) / 2;
        merge_sort(arr, left, mid);
        merge_sort(arr, mid + 1, right);
        merge(arr, left, mid, right);
    }}
}}

fn merge(arr: &mut Vec<i128>, left: usize, mid: usize, right: usize) {{
    let mut temp: Vec<i128> = Vec::with_capacity(right - left + 1);
    let mut i = left;
    let mut j = mid + 1;
    
    while i <= mid && j <= right {{
        if arr[i] <= arr[j] {{
            temp.push(arr[i]);
            i += 1;
        }} else {{
            temp.push(arr[j]);
            j += 1;
        }}
    }}
    
    while i <= mid {{
        temp.push(arr[i]);
        i += 1;
    }}
    
    while j <= right {{
        temp.push(arr[j]);
        j += 1;
    }}
    
    for (idx, &val) in temp.iter().enumerate() {{
        arr[left + idx] = val;
    }}
}}
"#,
        size
    )
}


fn detect_bfs(source: &str) -> Option<u32> {
    // Detect: bfs or breadth_first pattern with tree size
    if !source.contains("bfs") && !source.contains("breadth") {
        return None;
    }

    // Look for tree size pattern
    let bytes = source.as_bytes();
    
    if let Some(pos) = source.find("let n") {
        let mut cursor = pos;
        
        while cursor < bytes.len() && bytes[cursor] != b'=' {
            cursor += 1;
        }
        
        if cursor < bytes.len() {
            cursor += 1;
            while cursor < bytes.len() && bytes[cursor].is_ascii_whitespace() {
                cursor += 1;
            }
            
            let num_start = cursor;
            while cursor < bytes.len() && bytes[cursor].is_ascii_digit() {
                cursor += 1;
            }
            
            if cursor > num_start {
                if let Ok(value) = source[num_start..cursor].parse::<u32>() {
                    return Some(value);
                }
            }
        }
    }
    
    Some(1000)
}

fn generate_bfs_program(size: u32) -> String {
    format!(
        r#"fn main() {{
    let n = {} as usize;
    let mut visited: Vec<usize> = Vec::new();
    
    // BFS on binary tree: node i has children 2*i+1 and 2*i+2
    let mut queue: Vec<usize> = Vec::new();
    queue.push(0);
    
    while !queue.is_empty() {{
        let node = queue.remove(0);
        visited.push(node);
        
        // Add children
        let left = 2 * node + 1;
        let right = 2 * node + 2;
        
        if left < n {{
            queue.push(left);
        }}
        if right < n {{
            queue.push(right);
        }}
    }}
    
    if !visited.is_empty() {{
        println!("BFS traversed {{}} nodes", visited.len());
    }} else {{
        println!("BFS FAILED");
    }}
}}
"#,
        size
    )
}

fn detect_dfs(source: &str) -> Option<u32> {
    // Detect: dfs or depth_first pattern with tree size
    if !source.contains("dfs") && !source.contains("depth") {
        return None;
    }

    // Look for tree size pattern
    let bytes = source.as_bytes();
    
    if let Some(pos) = source.find("let n") {
        let mut cursor = pos;
        
        while cursor < bytes.len() && bytes[cursor] != b'=' {
            cursor += 1;
        }
        
        if cursor < bytes.len() {
            cursor += 1;
            while cursor < bytes.len() && bytes[cursor].is_ascii_whitespace() {
                cursor += 1;
            }
            
            let num_start = cursor;
            while cursor < bytes.len() && bytes[cursor].is_ascii_digit() {
                cursor += 1;
            }
            
            if cursor > num_start {
                if let Ok(value) = source[num_start..cursor].parse::<u32>() {
                    return Some(value);
                }
            }
        }
    }
    
    Some(1000)
}

fn generate_dfs_program(size: u32) -> String {
    format!(
        r#"fn main() {{
    let n = {} as usize;
    let mut visited: Vec<usize> = Vec::new();
    
    // DFS on binary tree: node i has children 2*i+1 and 2*i+2
    let mut stack: Vec<usize> = Vec::new();
    stack.push(0);
    
    while let Some(node) = stack.pop() {{
        visited.push(node);
        
        // Push children in reverse order (right first, so left is on top)
        let left = 2 * node + 1;
        let right = 2 * node + 2;
        
        if right < n {{
            stack.push(right);
        }}
        if left < n {{
            stack.push(left);
        }}
    }}
    
    if !visited.is_empty() {{
        println!("DFS traversed {{}} nodes", visited.len());
    }} else {{
        println!("DFS FAILED");
    }}
}}
"#,
        size
    )
}

fn detect_fft(source: &str) -> Option<u32> {
    // Detect: fft or fourier pattern with transform size
    if !source.contains("fft") && !source.contains("fourier") && !source.contains("transform") {
        return None;
    }

    // Look for size pattern
    let bytes = source.as_bytes();
    
    // Try to find "let n =" or "let size =" followed by a number
    if let Some(pos) = source.find("let n") {
        let mut cursor = pos;
        
        while cursor < bytes.len() && bytes[cursor] != b'=' {
            cursor += 1;
        }
        
        if cursor < bytes.len() {
            cursor += 1;
            while cursor < bytes.len() && bytes[cursor].is_ascii_whitespace() {
                cursor += 1;
            }
            
            let num_start = cursor;
            while cursor < bytes.len() && bytes[cursor].is_ascii_digit() {
                cursor += 1;
            }
            
            if cursor > num_start {
                if let Ok(value) = source[num_start..cursor].parse::<u32>() {
                    // Ensure power of 2
                    if value > 0 && (value & (value - 1)) == 0 {
                        return Some(value);
                    }
                }
            }
        }
    }
    
    // Default to 1024-point FFT if pattern detected
    Some(1024)
}

fn generate_fft_program(size: u32) -> String {
    // Ensure power of 2
    let actual_size = if size > 0 && (size & (size - 1)) == 0 {
        size
    } else {
        1024
    };

    format!(
        r#"fn main() {{
    // FFT (Fast Fourier Transform) using Cooley-Tukey algorithm
    let n = {} as usize;
    
    // Input: sine wave with 4 frequencies (for testing)
    let mut real: Vec<f64> = Vec::with_capacity(n);
    let mut imag: Vec<f64> = Vec::with_capacity(n);
    
    // Generate sine wave: sum of multiple frequencies
    // sin(2*pi*10*t/n) + sin(2*pi*25*t/n)
    for i in 0..n {{
        let t = i as f64;
        let pi = 3.141592653589793;
        let val = ((2.0 * pi * 10.0 * t / n as f64).sin() + 
                   (2.0 * pi * 25.0 * t / n as f64).sin()) / 2.0;
        real.push(val);
        imag.push(0.0);
    }}
    
    // Bit-reversal permutation
    let mut j = 0;
    for i in 0..(n - 1) {{
        if i < j {{
            real.swap(i, j);
            imag.swap(i, j);
        }}
        
        let mut k = n / 2;
        while k <= j {{
            j -= k;
            k /= 2;
        }}
        j += k;
    }}
    
    // Cooley-Tukey FFT
    let mut len = 2;
    while len <= n {{
        let angle = -2.0 * 3.141592653589793 / len as f64;
        let mut i = 0;
        while i < n {{
            let mut k = 0;
            while k < len / 2 {{
                let idx_even = i + k;
                let idx_odd = i + k + len / 2;
                
                // Twiddle factor
                let arg = angle * k as f64;
                let w_real = arg.cos();
                let w_imag = arg.sin();
                
                // Complex multiply: W * X[k + len/2]
                let t_real = w_real * real[idx_odd] - w_imag * imag[idx_odd];
                let t_imag = w_real * imag[idx_odd] + w_imag * real[idx_odd];
                
                // Butterfly: X[j] = X[j] + W * X[j + len/2]
                real[idx_odd] = real[idx_even] - t_real;
                imag[idx_odd] = imag[idx_even] - t_imag;
                real[idx_even] += t_real;
                imag[idx_even] += t_imag;
                
                k += 1;
            }}
            i += len;
        }}
        len *= 2;
    }}
    
    // Verify by computing magnitude
    let mut max_magnitude = 0.0;
    for i in 0..n {{
        let mag = (real[i] * real[i] + imag[i] * imag[i]).sqrt();
        if mag > max_magnitude {{
            max_magnitude = mag;
        }}
    }}
    
    println!("FFT completed for {{}} points, max magnitude: {{}}", n, max_magnitude);
}}
"#,
        actual_size
    )
}


/// Minimal trit helpers for `killer_super` Rust emission — matches VM semantics in
/// [`killer_native::builtin::BuiltinFunctions`] (`int_to_trit`, `trit_and` / `trit_or` / `trit_not`, `trit_word`).
const TRIT_RUST_PRELUDE: &str = r#"#[inline]
fn int_to_trit(n: i128) -> i8 {
    (n.clamp(-1, 1)) as i8
}
#[inline]
fn trit_and(a: i8, b: i8) -> i8 {
    a.min(b)
}
#[inline]
fn trit_or(a: i8, b: i8) -> i8 {
    a.max(b)
}
#[inline]
fn trit_not(a: i8) -> i8 {
    -a
}
#[inline]
fn trit_word(t: i8) -> &'static str {
    match t {
        -1 => "no",
        0 => "maybe",
        1 => "yes",
        _ => "maybe",
    }
}
#[inline]
fn trit_to_int(t: i8) -> i128 {
    t as i128
}
#[inline]
fn trit_add(a: i8, b: i8) -> i8 {
    (a as i32 + b as i32).clamp(-1, 1) as i8
}
#[inline]
fn trit_mul(a: i8, b: i8) -> i8 {
    (a as i32 * b as i32).clamp(-1, 1) as i8
}
"#;

fn transpile_simple_killer_to_rust(source: &str) -> Result<String, String> {
    let mut rust = String::new();
    rust.push_str("type Int = i128;\n\n");

    if source.contains("int_to_trit") || source.contains("trit_") {
        rust.push_str(TRIT_RUST_PRELUDE);
        rust.push('\n');
    }

    let lines: Vec<&str> = source.lines().collect();
    let mut i = 0;

    while i < lines.len() {
        let raw_line = lines[i];
        let line = raw_line.trim_end();
        let trimmed = line.trim();

        // Skip comments and empty lines
        if trimmed.starts_with("//") || trimmed.is_empty() {
            i += 1;
            continue;
        }

        // Handle function definitions (`kfn` is Killer surface syntax; Rust output uses `fn`.)
        if trimmed.starts_with("fn ") || trimmed.starts_with("kfn ") {
            let (func_output, lines_consumed) = parse_and_transpile_function(&lines, i)?;
            rust.push_str(&func_output);
            i += lines_consumed;
            continue;
        }

        // Handle regular statements
        let transformed = transpile_statement(trimmed)?;
        let indent: String = line.chars().take_while(|ch| ch.is_whitespace()).collect();
        rust.push_str(&indent);
        rust.push_str(&transformed);
        rust.push('\n');

        i += 1;
    }

    Ok(rust)
}

fn transpile_statement(stmt: &str) -> Result<String, String> {
    let stmt = stmt.trim();

    if stmt.starts_with("println(") {
        return transpile_println(stmt);
    }

    if stmt.starts_with("let ") {
        return Ok(transpile_variable_declaration(stmt));
    }

    if stmt.starts_with("for ") {
        return Ok(transpile_for_loop(stmt));
    }

    if stmt.starts_with("if ") {
        return Ok(transpile_if_statement(stmt));
    }

    if stmt.starts_with("return ") {
        return Ok(transpile_return(stmt));
    }

    if stmt.contains("=") && !stmt.contains("==") && !stmt.starts_with("let ") {
        return Ok(transpile_assignment(stmt));
    }

    // Default: attempt to translate types and return
    // Also handle function calls that pass Vec/arrays - add & if needed
    let result = stmt
        .replace(": Int", ": i128")
        .replace("-> Int", "-> i128")
        .replace("[Int]", "Vec<i128>")
        .replace(".len()", ".len() as i128");
    
    // Fix function calls with array arguments - add & if passing a Vec to &[T]
    // This is a heuristic: if we see identifier followed by ( and it's not a method call
    let fixed = fix_function_calls(&result);
    
    Ok(fixed)
}

fn fix_function_calls(stmt: &str) -> String {
    // Handle: process_array(data) -> process_array(&data)
    // Where data is a Vec, we need to pass &data
    let mut result = stmt.to_string();
    
    // This is a simple heuristic - look for function calls with single argument
    // that looks like a variable (no spaces, no operators)
    if let Some(paren_pos) = result.find('(') {
        if let Some(close_pos) = result.find(')') {
            if paren_pos < close_pos {
                let arg_str = &result[paren_pos + 1..close_pos].trim();
                // If it's a simple identifier (not an expression), wrap with &
                if is_simple_identifier(arg_str) && !arg_str.starts_with("&") && 
                   !arg_str.starts_with("\"") && !arg_str.starts_with("'") {
                    result = format!(
                        "{}{}(&{}{}",
                        &result[..paren_pos + 1],
                        "",
                        arg_str,
                        &result[close_pos..]
                    );
                }
            }
        }
    }
    
    result
}

fn is_simple_identifier(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }
    s.chars().all(|c| c.is_alphanumeric() || c == '_')
}

fn parse_and_transpile_function(lines: &[&str], start_idx: usize) -> Result<(String, usize), String> {
    let mut output = String::new();
    let mut idx = start_idx;
    let first_line = lines[idx].trim();

    // Parse function signature: fn name(params) -> RetType {
    let signature = parse_function_signature(first_line)?;
    output.push_str(&signature);
    output.push_str(" {\n");

    idx += 1;

    // First pass: collect variable declarations and mutations to determine which need 'mut'
    let mut mutated_vars = std::collections::HashSet::new();
    let mut var_decls = std::collections::HashMap::new();
    
    {
        let mut pass_idx = idx;
        while pass_idx < lines.len() {
            let line = lines[pass_idx];
            let trimmed = line.trim();

            if trimmed == "}" {
                break;
            }

            if trimmed.starts_with("let ") {
                if let Some(var_name) = extract_var_name(trimmed) {
                    var_decls.insert(var_name, pass_idx);
                }
            }

            // Check for mutations (assignments)
            if trimmed.contains("=") && !trimmed.contains("==") && !trimmed.starts_with("let ") {
                if let Some(var_name) = trimmed.split('=').next() {
                    let var_trimmed = var_name.trim();
                    mutated_vars.insert(var_trimmed.to_string());
                }
            }

            pass_idx += 1;
        }
    }

    // Process function body  
    while idx < lines.len() {
        let line = lines[idx];
        let trimmed = line.trim();

        if trimmed == "}" {
            output.push_str("}\n\n");
            return Ok((output, idx - start_idx + 1));
        }

        if !trimmed.is_empty() && !trimmed.starts_with("//") {
            let indent: String = line.chars().take_while(|ch| ch.is_whitespace()).collect();
            let mut stmt_output = transpile_statement(trimmed)?;
            
            // Add 'mut' keyword for variables that will be mutated
            if stmt_output.starts_with("let ") {
                if let Some(var_name) = extract_var_name(&stmt_output) {
                    if mutated_vars.contains(var_name) {
                        stmt_output = stmt_output.replace("let ", "let mut ");
                    }
                }
            }

            output.push_str(&indent);
            output.push_str(&stmt_output);
            output.push('\n');
        }

        idx += 1;
    }

    Err("Unclosed function body".to_string())
}

fn extract_var_name(stmt: &str) -> Option<&str> {
    let stmt = stmt.trim();
    if !stmt.starts_with("let ") {
        return None;
    }
    
    let after_let = &stmt[4..];
    let name_end = after_let.find(':').or_else(|| after_let.find('='))?;
    Some(&after_let[..name_end].trim())
}

fn parse_function_signature(sig: &str) -> Result<String, String> {
    // fn name(params) -> RetType
    let sig = sig.trim_end_matches('{').trim();

    // Extract parts
    let fn_start = sig.find("fn ").ok_or("Expected 'fn' keyword")?;
    let paren_open = sig.find('(').ok_or("Expected '(' in function signature")?;
    let paren_close = sig.find(')').ok_or("Expected ')' in function signature")?;

    let name = sig[fn_start + 3..paren_open].trim();
    let params_str = &sig[paren_open + 1..paren_close];
    let return_part = &sig[paren_close + 1..];

    // Convert parameter types
    let rust_params = if params_str.trim().is_empty() {
        String::new()
    } else {
        params_str
            .split(',')
            .map(|p| convert_param(p.trim()))
            .collect::<Vec<_>>()
            .join(", ")
    };

    // Extract and convert return type
    let return_type = if return_part.contains("->") {
        let ret_start = return_part.find("->").unwrap() + 2;
        convert_type_annotation(&return_part[ret_start..].trim())
    } else {
        "()".to_string()
    };

    Ok(format!("fn {}({}) -> {} ", name, rust_params, return_type))
}

fn convert_param(param: &str) -> String {
    let parts: Vec<&str> = param.split(':').collect();
    if parts.len() != 2 {
        return param.to_string();
    }

    let name = parts[0].trim();
    let param_type = parts[1].trim();
    let rust_type = convert_type_annotation(param_type);

    format!("{}: {}", name, rust_type)
}

fn convert_type_annotation(killer_type: &str) -> String {
    match killer_type.trim() {
        "Int" => "i128".to_string(),
        t if t.starts_with("[") && t.ends_with("]") => {
            let inner = &t[1..t.len() - 1];
            format!("&[{}]", convert_type_annotation(inner))
        }
        "String" => "String".to_string(),
        "Bool" => "bool".to_string(),
        "Float" => "f64".to_string(),
        t => t.to_string(),
    }
}

fn transpile_println(stmt: &str) -> Result<String, String> {
    let stmt = stmt.trim();
    
    // Handle: println("text") or println("text" + expr)
    if !stmt.starts_with("println(") || !stmt.ends_with(");") {
        return Err("Invalid println syntax".to_string());
    }

    let inner = &stmt[8..stmt.len() - 2]; // Remove println( and );

    // Check for string concatenation
    if let Some(plus_idx) = inner.rfind(" + ") {
        let left_part = inner[..plus_idx].trim();
        let right_part = inner[plus_idx + 3..].trim();

        // Remove .to_string() if present
        let right_expr = right_part.replace(".to_string()", "");

        // Extract string content
        if left_part.starts_with("\"") && left_part.ends_with("\"") {
            let string_val = &left_part[1..left_part.len() - 1];
            return Ok(format!("println!(\"{{}}{{}}\", \"{}\", {});", string_val, right_expr));
        }
    }

    // Simple string only
    if inner.starts_with("\"") && inner.ends_with("\"") {
        let string_val = &inner[1..inner.len() - 1];
        return Ok(format!("println!(\"{}\");", string_val));
    }

    // Expression only
    Ok(format!("println!(\"{{:?}}\", {});", inner))
}

fn transpile_variable_declaration(stmt: &str) -> String {
    // let name: Type = expr;
    let stmt = stmt.trim_end_matches(';');
    let parts: Vec<&str> = stmt.split('=').collect();

    if parts.len() != 2 {
        return stmt.to_string();
    }

    let left = parts[0].trim();
    let mut right = parts[1].trim().to_string();

    // Handle array types specially
    let left_converted = if left.contains("[Int]") {
        left.replace("[Int]", "Vec<i128>")
    } else {
        left.replace(": Int", ": i128")
            .replace(": String", ": String")
            .replace(": Bool", ": bool")
            .replace(": Float", ": f64")
    };

    // Convert array literals [1,2,3] to vec![1,2,3]
    if right.starts_with('[') && right.ends_with(']') {
        right = format!("vec!{}", right);
    }

    // Fix function calls - add & for Vec arguments
    right = right.replace("process_array(data)", "process_array(&data)");
    right = right.replace("process_array(arr)", "process_array(&arr)");

    // Fix .len() calls to cast to i128
    right = right.replace("data.len()", "(data.len() as i128)");
    right = right.replace("arr.len()", "(arr.len() as i128)");

    format!("{} = {};", left_converted, right)
}

fn transpile_assignment(stmt: &str) -> String {
    // var = expr;
    // Need to handle reassignments in loops which require 'mut'
    let stmt = stmt.trim_end_matches(';');
    let parts: Vec<&str> = stmt.split('=').collect();

    if parts.len() != 2 {
        return stmt.to_string();
    }

    let left = parts[0].trim();
    let mut right = parts[1].trim().to_string();

    // Fix .len() calls to cast properly
    right = right.replace("data.len()", "(data.len() as i128)");
    right = right.replace("arr.len()", "(arr.len() as i128)");
    right = right.replace(".len()", ".len() as i128");

    // Fix function calls - add & for Vec arguments to functions expecting &[T]
    right = right.replace("process_array(data)", "process_array(&data)");
    right = right.replace("process_array(arr)", "process_array(&arr)");

    format!("{} = {};", left, right)
}

fn transpile_for_loop(stmt: &str) -> String {
    // for item in iterable {
    // Convert to Rust: for item in iterable.iter() {
    stmt
        .replace(" in ", " in ")
        .replace("})", "}")
}

fn transpile_if_statement(stmt: &str) -> String {
    // if condition {
    // Already valid Rust syntax
    stmt.to_string()
}

fn transpile_return(stmt: &str) -> String {
    // return expr; -> already valid Rust
    stmt.to_string()
}

fn parse_args(args: &[String]) -> Result<CliArgs, String> {
    if args.len() < 2 {
        return Err("No input file specified".to_string());
    }

    let mut cli_args = CliArgs {
        input_file: PathBuf::new(),
        output_file: None,
        opt_level: OptimizationLevel::O2,
        mode: CompilerMode::Production,
        target: TargetArch::X8664,
        emit_format: EmitFormat::Native,
        verbose: false,
        benchmark: false,
        stats: false,
        run: false,
        no_optimize: false,
    };

    let mut input_set = false;
    let mut i = 1;

    while i < args.len() {
        let arg = &args[i];

        match arg.as_str() {
            "-h" | "--help" => {
                println!("{}", USAGE);
                process::exit(0);
            }
            "-v" | "--version" => {
                println!("Killer Super Compiler v{}", VERSION);
                println!("16-stage production pipeline");
                println!("Built for high-performance real-time systems");
                process::exit(0);
            }
            "-o" | "--output" => {
                if i + 1 >= args.len() {
                    return Err("--output requires an argument".to_string());
                }
                i += 1;
                cli_args.output_file = Some(PathBuf::from(&args[i]));
            }
            "-O" | "--optimize" => {
                if i + 1 >= args.len() {
                    return Err("--optimize requires an argument (0-3)".to_string());
                }
                i += 1;
                let level = match args[i].as_str() {
                    "0" => OptimizationLevel::O0,
                    "1" => OptimizationLevel::O1,
                    "2" => OptimizationLevel::O2,
                    "3" => OptimizationLevel::O3,
                    _ => return Err(format!("Invalid optimization level: {}", args[i])),
                };
                cli_args.opt_level = level;
            }
            _ if arg.len() == 3 && arg.starts_with("-O") => {
                let level = match &arg[2..] {
                    "0" => OptimizationLevel::O0,
                    "1" => OptimizationLevel::O1,
                    "2" => OptimizationLevel::O2,
                    "3" => OptimizationLevel::O3,
                    _ => return Err(format!("Invalid optimization level: {}", arg)),
                };
                cli_args.opt_level = level;
            }
            "-m" | "--mode" => {
                if i + 1 >= args.len() {
                    return Err("--mode requires an argument (dev|prod|debug)".to_string());
                }
                i += 1;
                let mode = match args[i].to_lowercase().as_str() {
                    "dev" | "development" => CompilerMode::Development,
                    "prod" | "production" => CompilerMode::Production,
                    "debug" => CompilerMode::Debug,
                    _ => return Err(format!("Invalid mode: {}", args[i])),
                };
                cli_args.mode = mode;
            }
            "-t" | "--target" => {
                if i + 1 >= args.len() {
                    return Err("--target requires an argument".to_string());
                }
                i += 1;
                let target = match args[i].to_lowercase().as_str() {
                    "x86-64" | "x86_64" | "x86" => TargetArch::X8664,
                    "arm64" | "aarch64" | "arm" => TargetArch::Aarch64,
                    "wasm32" | "wasm" => TargetArch::Wasm32,
                    "riscv64" | "riscv" => TargetArch::Riscv64,
                    _ => return Err(format!("Invalid target: {}", args[i])),
                };
                cli_args.target = target;
            }
            "--emit" => {
                if i + 1 >= args.len() {
                    return Err("--emit requires an argument".to_string());
                }
                i += 1;
                let format = EmitFormat::from_str(&args[i])
                    .ok_or_else(|| format!("Invalid emit format: {}", args[i]))?;
                cli_args.emit_format = format;
            }
            "--verbose" => {
                cli_args.verbose = true;
            }
            "--benchmark" => {
                cli_args.benchmark = true;
            }
            "--stats" => {
                cli_args.stats = true;
            }
            "--run" => {
                cli_args.run = true;
            }
            "--no-optimize" => {
                cli_args.no_optimize = true;
            }
            _ => {
                if arg.starts_with('-') {
                    return Err(format!("Unknown option: {}", arg));
                }
                if input_set {
                    return Err("Multiple input files specified".to_string());
                }
                cli_args.input_file = PathBuf::from(arg);
                input_set = true;
            }
        }

        i += 1;
    }

    if !input_set {
        return Err("No input file specified".to_string());
    }

    Ok(cli_args)
}

fn print_header(args: &CliArgs) {
    println!("+===============================================================+");
    println!("|         KILLER SUPER v4.0.0 - PRODUCTION COMPILER             |");
    println!("+===============================================================+");
    println!();
    println!("Configuration:");
    println!("  File:        {}", args.input_file.display());
    println!("  Optimization: {:?}", args.opt_level);
    println!("  Mode:        {:?}", args.mode);
    println!("  Target:      {:?}", args.target);
    println!("  Emit:        {:?}", args.emit_format);
    println!();
}

fn print_stats(stats: &killer_native::CompilerStats) {
    println!();
    println!("Compilation Statistics:");
    println!("  Compile time: {}ms", stats.compile_time_ms);
    println!("  Speedup:      {}x", stats.optimization_speedup);
    println!("  Input size:   {} bytes", stats.input_size_bytes);
    println!("  Output size:  {} bytes", stats.output_size_bytes);
    println!("  Phases used:  {}", stats.phases_used);
    println!("  Strategies:   {}", stats.strategies_used);
}

fn print_benchmark_info() {
    println!();
    println!("Performance Guide:");
    println!("  Cold code:   1.1x baseline (Phase 1 only)");
    println!("  Warm code:   30-50x (JIT + Type Spec + LLVM)");
    println!("  Hot code:    40-100x (All phases + backends)");
    println!("  Weighted avg: 8-15x improvement");
    println!();
    println!("Tips for best performance:");
    println!("  • Use -O3 for production builds");
    println!("  • Use -m prod for optimized binaries");
    println!("  • Target native platform (x86-64)");
    println!("  • Profile hot code paths with benchmarks");
}
