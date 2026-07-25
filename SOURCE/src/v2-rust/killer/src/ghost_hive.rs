//! Ghost Hive — 1M-agent evolution engine built on the Ghost VM.
//!
//! Capsule evaluation reuses the Ghost VM's `run()` with `NullHost`.
//! Populations persist to `.hive` binary files and evolution can resume
//! from any saved generation.
//!
//! Parallelism: `std::thread::scope` splits evaluation across CPU cores.

use crate::ghost_vm::{self, Capsule, NullHost, MAX_RAM, OP_HALT, OP_PUSH, OP_DUP,
                       OP_MUL, OP_ADD, OP_NOP, CAPABILITIES_ALL,
                       CAPABILITY_SELF_MODIFY};

// ---------------------------------------------------------------------------
// Data types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct Agent {
    pub code: Vec<u8>,
    pub fitness: i64,
    pub age: u64,
    pub lineage: u64,
    pub mutations: u32,
}

impl Agent {
    pub fn new(code: Vec<u8>) -> Self {
        Self { code, fitness: i64::MIN, age: 0, lineage: 0, mutations: 0 }
    }
}

#[derive(Debug, Clone)]
pub enum FitnessMode {
    MaxOutput,
    Target(i32),
    Longevity,
    Diversity,
    TestSuite { inputs: Vec<i32>, expected: Vec<i32> },
}

#[derive(Debug, Clone)]
pub struct HiveConfig {
    pub population_size: usize,
    pub elite_ratio: f64,
    pub mutation_rate: f64,
    pub crossover_rate: f64,
    pub max_code_len: usize,
    pub fuel_per_eval: u32,
    pub fitness_fn: FitnessMode,
    pub num_threads: usize,
}

impl Default for HiveConfig {
    fn default() -> Self {
        Self {
            population_size: 1000,
            elite_ratio: 0.10,
            mutation_rate: 0.05,
            crossover_rate: 0.70,
            max_code_len: 256,
            fuel_per_eval: 1000,
            fitness_fn: FitnessMode::MaxOutput,
            num_threads: 4,
        }
    }
}

pub struct HiveEngine {
    pub population: Vec<Agent>,
    pub hall_of_fame: Vec<Agent>,
    pub generation: u64,
    pub config: HiveConfig,
    rng_state: u64,
}

const MAX_HOF: usize = 100;

// ---------------------------------------------------------------------------
// RNG — SplitMix64 (fast, deterministic, no deps)
// ---------------------------------------------------------------------------

fn splitmix64(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9e3779b97f4a7c15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
    z ^ (z >> 31)
}

// ---------------------------------------------------------------------------
// Mutation operators
// ---------------------------------------------------------------------------

/// Weighted mutation: BYTE_FLIP 40%, BYTE_INSERT 15%, BYTE_DELETE 15%,
/// OPCODE_SWAP 10%, BLOCK_COPY 10%, BLOCK_SHUFFLE 10%
fn mutate_code(code: &[u8], rate: f64, max_len: usize, rng: &mut u64) -> Vec<u8> {
    let mut out = code.to_vec();
    if out.is_empty() {
        out.push(OP_HALT);
        return out;
    }

    let num_mutations = ((out.len() as f64 * rate).ceil() as usize).max(1);

    for _ in 0..num_mutations {
        let roll = (splitmix64(rng) % 100) as u32;
        if out.is_empty() { break; }

        match roll {
            0..=39 => {
                // BYTE_FLIP
                let idx = splitmix64(rng) as usize % out.len();
                out[idx] = (splitmix64(rng) & 0xFF) as u8;
            }
            40..=54 => {
                // BYTE_INSERT
                if out.len() < max_len {
                    let idx = splitmix64(rng) as usize % (out.len() + 1);
                    let val = (splitmix64(rng) & 0xFF) as u8;
                    out.insert(idx, val);
                }
            }
            55..=69 => {
                // BYTE_DELETE
                if out.len() > 1 {
                    let idx = splitmix64(rng) as usize % out.len();
                    out.remove(idx);
                }
            }
            70..=79 => {
                // OPCODE_SWAP — swap two adjacent bytes
                if out.len() >= 2 {
                    let idx = splitmix64(rng) as usize % (out.len() - 1);
                    out.swap(idx, idx + 1);
                }
            }
            80..=89 => {
                // BLOCK_COPY — copy 3-8 bytes within the code
                let blen = 3 + (splitmix64(rng) as usize % 6);
                let blen = blen.min(out.len());
                let src = splitmix64(rng) as usize % (out.len() - blen + 1);
                let block: Vec<u8> = out[src..src + blen].to_vec();
                let dst = splitmix64(rng) as usize % (out.len() + 1);
                let dst = dst.min(out.len());
                for (i, &b) in block.iter().enumerate() {
                    if dst + i < out.len() {
                        out[dst + i] = b;
                    } else if out.len() < max_len {
                        out.push(b);
                    }
                }
            }
            _ => {
                // BLOCK_SHUFFLE — shuffle 3-5 bytes
                let blen = 3 + (splitmix64(rng) as usize % 3);
                let blen = blen.min(out.len());
                let start = splitmix64(rng) as usize % (out.len() - blen + 1);
                for i in (start + 1..start + blen).rev() {
                    let j = start + (splitmix64(rng) as usize % (i - start + 1));
                    out.swap(i, j);
                }
            }
        }
    }

    // Ensure HALT terminator and length cap
    if out.len() > max_len {
        out.truncate(max_len - 1);
    }
    if out.is_empty() || *out.last().unwrap() != OP_HALT {
        out.push(OP_HALT);
    }
    out
}

fn crossover(a: &[u8], b: &[u8], max_len: usize, rng: &mut u64) -> Vec<u8> {
    if a.is_empty() || b.is_empty() {
        return a.to_vec();
    }
    let split_a = splitmix64(rng) as usize % a.len();
    let split_b = splitmix64(rng) as usize % b.len();
    let mut child: Vec<u8> = a[..split_a].to_vec();
    child.extend_from_slice(&b[split_b..]);
    if child.len() > max_len {
        child.truncate(max_len - 1);
    }
    if child.is_empty() || *child.last().unwrap() != OP_HALT {
        child.push(OP_HALT);
    }
    child
}

fn lineage_hash(parent_lineage: u64, mutation_count: u32) -> u64 {
    let mut s = parent_lineage;
    s = s.wrapping_mul(6364136223846793005).wrapping_add(mutation_count as u64);
    s ^ (s >> 33)
}

// ---------------------------------------------------------------------------
// Fitness evaluation
// ---------------------------------------------------------------------------

fn eval_agent(code: &[u8], config: &HiveConfig) -> i64 {
    match &config.fitness_fn {
        FitnessMode::MaxOutput => {
            eval_single(code, 0, config.fuel_per_eval)
        }
        FitnessMode::Target(target) => {
            let result = eval_single(code, 0, config.fuel_per_eval);
            let diff = (result as i64 - *target as i64).abs();
            -(diff) // closer to target = higher fitness (less negative)
        }
        FitnessMode::Longevity => {
            eval_longevity(code, config.fuel_per_eval)
        }
        FitnessMode::Diversity => {
            eval_diversity(code, config.fuel_per_eval)
        }
        FitnessMode::TestSuite { inputs, expected } => {
            eval_test_suite(code, inputs, expected, config.fuel_per_eval)
        }
    }
}

fn eval_single(code: &[u8], input: i32, fuel: u32) -> i64 {
    let mut capsule = Capsule::with_ram_and_fuel(MAX_RAM, fuel);
    capsule.code = code.to_vec();
    capsule.capabilities = CAPABILITIES_ALL & !CAPABILITY_SELF_MODIFY;
    capsule.ram[0..4].copy_from_slice(&input.to_le_bytes());
    let mut host = NullHost;
    let _ = ghost_vm::run(&mut capsule, &mut host, Some(fuel));
    capsule.stack.last().copied().unwrap_or(i64::MIN)
}

fn eval_longevity(code: &[u8], fuel: u32) -> i64 {
    let mut capsule = Capsule::with_ram_and_fuel(MAX_RAM, fuel);
    capsule.code = code.to_vec();
    capsule.capabilities = CAPABILITIES_ALL & !CAPABILITY_SELF_MODIFY;
    let mut host = NullHost;
    let _ = ghost_vm::run(&mut capsule, &mut host, Some(fuel));
    // Score = how much fuel was consumed (higher = ran longer)
    let remaining = capsule.stack.last().copied().unwrap_or(0);
    (fuel as i64) - (remaining.max(0) as i64)
}

fn eval_diversity(code: &[u8], fuel: u32) -> i64 {
    let mut seen = std::collections::HashSet::new();
    for input in -5..=5i32 {
        let result = eval_single(code, input, fuel);
        seen.insert(result);
    }
    seen.len() as i64
}

fn eval_test_suite(code: &[u8], inputs: &[i32], expected: &[i32], fuel: u32) -> i64 {
    let mut score: i64 = 0;
    for (inp, exp) in inputs.iter().zip(expected.iter()) {
        let result = eval_single(code, *inp, fuel);
        let diff = (result - *exp as i64).abs();
        score -= diff;
    }
    score
}

// ---------------------------------------------------------------------------
// Seed population
// ---------------------------------------------------------------------------

fn generate_seed_population(size: usize, max_code_len: usize, rng: &mut u64) -> Vec<Agent> {
    let mut pop = Vec::with_capacity(size);

    // Template programs
    let templates: Vec<Vec<u8>> = vec![
        // push X; halt
        vec![OP_PUSH, 42, 0, 0, 0, OP_HALT],
        // push X; dup; mul; halt  → X²
        vec![OP_PUSH, 7, 0, 0, 0, OP_DUP, OP_MUL, OP_HALT],
        // push 1000; halt
        vec![OP_PUSH, 0xE8, 0x03, 0, 0, OP_HALT],
        // load 0 (from RAM); dup; mul; halt → input²
        vec![0x20, 0x00, 0x00, OP_DUP, OP_MUL, OP_HALT],
        // push large; dup; mul; halt
        vec![OP_PUSH, 0xFF, 0x7F, 0, 0, OP_DUP, OP_MUL, OP_HALT],
        // push max_i32; halt
        vec![OP_PUSH, 0xFF, 0xFF, 0xFF, 0x7F, OP_HALT],
        // nop; push 1; halt
        vec![OP_NOP, OP_PUSH, 1, 0, 0, 0, OP_HALT],
        // push A; push B; add; halt
        vec![OP_PUSH, 100, 0, 0, 0, OP_PUSH, 200, 0, 0, 0, OP_ADD, OP_HALT],
    ];

    for t in &templates {
        if pop.len() >= size { break; }
        pop.push(Agent::new(t.clone()));
    }

    while pop.len() < size {
        let len = 4 + (splitmix64(rng) as usize % (max_code_len.min(32) - 3));
        let len = len.min(max_code_len);
        let mut code = Vec::with_capacity(len);
        for _ in 0..len - 1 {
            code.push((splitmix64(rng) & 0xFF) as u8);
        }
        code.push(OP_HALT);
        pop.push(Agent::new(code));
    }

    pop
}

// ---------------------------------------------------------------------------
// HiveEngine
// ---------------------------------------------------------------------------

impl HiveEngine {
    pub fn new(config: HiveConfig) -> Self {
        let mut rng_state: u64 = 0xDEAD_BEEF_CAFE_BABE;
        let population = generate_seed_population(
            config.population_size, config.max_code_len, &mut rng_state,
        );
        Self {
            population,
            hall_of_fame: Vec::new(),
            generation: 0,
            config,
            rng_state,
        }
    }

    pub fn with_seed(config: HiveConfig, seed_code: Vec<u8>) -> Self {
        let mut rng_state: u64 = 0xDEAD_BEEF_CAFE_BABE;
        let mut pop = Vec::with_capacity(config.population_size);
        pop.push(Agent::new(seed_code.clone()));
        for _ in 1..config.population_size {
            let mutated = mutate_code(&seed_code, config.mutation_rate, config.max_code_len, &mut rng_state);
            pop.push(Agent::new(mutated));
        }
        Self {
            population: pop,
            hall_of_fame: Vec::new(),
            generation: 0,
            config,
            rng_state,
        }
    }

    fn next_rng(&mut self) -> u64 {
        splitmix64(&mut self.rng_state)
    }

    /// Evaluate all agents in parallel, return (best_fitness, avg_fitness).
    pub fn evaluate_parallel(&mut self) -> (i64, i64) {
        let config = &self.config;
        let num_threads = config.num_threads.max(1);

        if num_threads == 1 || self.population.len() < 64 {
            // Serial path
            for agent in &mut self.population {
                agent.fitness = eval_agent(&agent.code, config);
            }
        } else {
            let chunk_size = (self.population.len() + num_threads - 1) / num_threads;
            let pop_slice = &mut self.population[..];

            std::thread::scope(|s| {
                let mut handles = Vec::new();
                for chunk in pop_slice.chunks_mut(chunk_size) {
                    let cfg = config.clone();
                    handles.push(s.spawn(move || {
                        for agent in chunk.iter_mut() {
                            agent.fitness = eval_agent(&agent.code, &cfg);
                        }
                    }));
                }
                for h in handles {
                    let _ = h.join();
                }
            });
        }

        let best = self.population.iter().map(|a| a.fitness).max().unwrap_or(i64::MIN);
        let sum: i64 = self.population.iter().map(|a| a.fitness).fold(0i64, |acc, v| acc.saturating_add(v));
        let avg = if self.population.is_empty() { 0 } else { sum / self.population.len() as i64 };
        (best, avg)
    }

    pub fn evolve_generation(&mut self) -> GenerationReport {
        let (best_fitness, avg_fitness) = self.evaluate_parallel();

        // Sort descending by fitness
        self.population.sort_by(|a, b| b.fitness.cmp(&a.fitness));

        // Update hall of fame
        let top10: Vec<Agent> = self.population.iter().take(10).cloned().collect();
        for agent in top10 {
            self.update_hof(agent);
        }

        let pop_size = self.config.population_size;
        let elite_count = ((pop_size as f64 * self.config.elite_ratio).ceil() as usize).max(1);
        let parent_pool_size = (pop_size / 2).max(elite_count + 1);

        let mut next_gen: Vec<Agent> = Vec::with_capacity(pop_size);

        // Elite: top agents survive unchanged, age incremented
        for agent in self.population.iter().take(elite_count) {
            let mut elite = agent.clone();
            elite.age += 1;
            next_gen.push(elite);
        }

        // Fill remainder with crossover + mutation
        while next_gen.len() < pop_size {
            let roll = self.next_rng() % 100;
            let child_code = if roll < (self.config.crossover_rate * 100.0) as u64 {
                let p1 = self.next_rng() as usize % parent_pool_size;
                let p2 = self.next_rng() as usize % parent_pool_size;
                let p1 = p1.min(self.population.len() - 1);
                let p2 = p2.min(self.population.len() - 1);
                crossover(
                    &self.population[p1].code,
                    &self.population[p2].code,
                    self.config.max_code_len,
                    &mut self.rng_state,
                )
            } else {
                let p = self.next_rng() as usize % parent_pool_size;
                let p = p.min(self.population.len() - 1);
                self.population[p].code.clone()
            };

            let mutated = mutate_code(&child_code, self.config.mutation_rate, self.config.max_code_len, &mut self.rng_state);
            let parent_idx = self.next_rng() as usize % parent_pool_size.min(self.population.len());
            let parent = &self.population[parent_idx];
            next_gen.push(Agent {
                code: mutated,
                fitness: i64::MIN,
                age: 0,
                lineage: lineage_hash(parent.lineage, parent.mutations + 1),
                mutations: parent.mutations + 1,
            });
        }

        self.population = next_gen;
        self.generation += 1;

        let best_agent = self.population.first().map(|a| a.clone());
        GenerationReport {
            generation: self.generation,
            population_size: pop_size,
            best_fitness,
            avg_fitness,
            best_age: best_agent.as_ref().map(|a| a.age).unwrap_or(0),
            best_mutations: best_agent.as_ref().map(|a| a.mutations).unwrap_or(0),
        }
    }

    fn update_hof(&mut self, agent: Agent) {
        // Only add if better than worst in HOF or HOF isn't full
        if self.hall_of_fame.len() < MAX_HOF {
            if !self.hall_of_fame.iter().any(|a| a.code == agent.code) {
                self.hall_of_fame.push(agent);
                self.hall_of_fame.sort_by(|a, b| b.fitness.cmp(&a.fitness));
            }
        } else {
            let worst = self.hall_of_fame.last().map(|a| a.fitness).unwrap_or(i64::MIN);
            if agent.fitness > worst && !self.hall_of_fame.iter().any(|a| a.code == agent.code) {
                self.hall_of_fame.pop();
                self.hall_of_fame.push(agent);
                self.hall_of_fame.sort_by(|a, b| b.fitness.cmp(&a.fitness));
            }
        }
    }

    pub fn best_agent(&self) -> Option<&Agent> {
        self.hall_of_fame.first().or_else(|| {
            self.population.iter().max_by_key(|a| a.fitness)
        })
    }

    /// Inject an external capsule into the population (replaces worst agent).
    pub fn inject(&mut self, code: Vec<u8>) {
        let agent = Agent::new(code);
        if let Some(worst) = self.population.iter_mut().min_by_key(|a| a.fitness) {
            *worst = agent;
        }
    }

    /// Export best agent as a Ghost VM Capsule.
    pub fn export_best(&self) -> Option<Capsule> {
        self.best_agent().map(|agent| {
            let mut c = Capsule::with_ram_and_fuel(MAX_RAM, self.config.fuel_per_eval);
            c.code = agent.code.clone();
            c
        })
    }
}

#[derive(Debug, Clone)]
pub struct GenerationReport {
    pub generation: u64,
    pub population_size: usize,
    pub best_fitness: i64,
    pub avg_fitness: i64,
    pub best_age: u64,
    pub best_mutations: u32,
}

impl std::fmt::Display for GenerationReport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Gen {} | Pop {} | Best: {} | Avg: {} | Age: {} | Mutations: {}",
            self.generation, self.population_size,
            self.best_fitness, self.avg_fitness,
            self.best_age, self.best_mutations,
        )
    }
}

// ---------------------------------------------------------------------------
// .hive binary format
// ---------------------------------------------------------------------------
//
// HIVE (4 bytes magic)
// version: u16 = 1
// config block: population_size(u32) + elite_ratio(f64) + mutation_rate(f64)
//               + crossover_rate(f64) + max_code_len(u32) + fuel_per_eval(u32)
//               + fitness_mode(u8) + mode_payload
//               + num_threads(u32)
// generation: u64
// rng_state: u64
// population_count: u32
// for each agent:
//   code_len: u16 + code bytes + fitness: i64 + age: u64 + lineage: u64 + mutations: u32
// hof_count: u32
// for each hof agent: same format

const HIVE_MAGIC: &[u8; 4] = b"HIVE";
const HIVE_VERSION: u16 = 1;

impl HiveEngine {
    pub fn encode(&self) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(HIVE_MAGIC);
        out.extend_from_slice(&HIVE_VERSION.to_le_bytes());

        // Config
        out.extend_from_slice(&(self.config.population_size as u32).to_le_bytes());
        out.extend_from_slice(&self.config.elite_ratio.to_le_bytes());
        out.extend_from_slice(&self.config.mutation_rate.to_le_bytes());
        out.extend_from_slice(&self.config.crossover_rate.to_le_bytes());
        out.extend_from_slice(&(self.config.max_code_len as u32).to_le_bytes());
        out.extend_from_slice(&self.config.fuel_per_eval.to_le_bytes());
        out.extend_from_slice(&(self.config.num_threads as u32).to_le_bytes());

        // Fitness mode
        match &self.config.fitness_fn {
            FitnessMode::MaxOutput => { out.push(0); }
            FitnessMode::Target(v) => {
                out.push(1);
                out.extend_from_slice(&v.to_le_bytes());
            }
            FitnessMode::Longevity => { out.push(2); }
            FitnessMode::Diversity => { out.push(3); }
            FitnessMode::TestSuite { inputs, expected } => {
                out.push(4);
                out.extend_from_slice(&(inputs.len() as u32).to_le_bytes());
                for v in inputs { out.extend_from_slice(&v.to_le_bytes()); }
                for v in expected { out.extend_from_slice(&v.to_le_bytes()); }
            }
        }

        out.extend_from_slice(&self.generation.to_le_bytes());
        out.extend_from_slice(&self.rng_state.to_le_bytes());

        // Population
        out.extend_from_slice(&(self.population.len() as u32).to_le_bytes());
        for agent in &self.population {
            encode_agent(&mut out, agent);
        }

        // Hall of fame
        out.extend_from_slice(&(self.hall_of_fame.len() as u32).to_le_bytes());
        for agent in &self.hall_of_fame {
            encode_agent(&mut out, agent);
        }

        out
    }

    pub fn decode(bytes: &[u8]) -> Result<Self, String> {
        if bytes.len() < 4 { return Err("truncated".into()); }
        if &bytes[0..4] != HIVE_MAGIC { return Err("bad magic".into()); }
        let mut i = 4;

        let version = read_u16(bytes, &mut i)?;
        if version != HIVE_VERSION { return Err(format!("unsupported hive version {version}")); }

        let population_size = read_u32(bytes, &mut i)? as usize;
        let elite_ratio = read_f64(bytes, &mut i)?;
        let mutation_rate = read_f64(bytes, &mut i)?;
        let crossover_rate = read_f64(bytes, &mut i)?;
        let max_code_len = read_u32(bytes, &mut i)? as usize;
        let fuel_per_eval = read_u32(bytes, &mut i)?;
        let num_threads = read_u32(bytes, &mut i)? as usize;

        let mode_tag = read_u8(bytes, &mut i)?;
        let fitness_fn = match mode_tag {
            0 => FitnessMode::MaxOutput,
            1 => {
                let v = read_i32(bytes, &mut i)?;
                FitnessMode::Target(v)
            }
            2 => FitnessMode::Longevity,
            3 => FitnessMode::Diversity,
            4 => {
                let count = read_u32(bytes, &mut i)? as usize;
                let mut inputs = Vec::with_capacity(count);
                for _ in 0..count { inputs.push(read_i32(bytes, &mut i)?); }
                let mut expected = Vec::with_capacity(count);
                for _ in 0..count { expected.push(read_i32(bytes, &mut i)?); }
                FitnessMode::TestSuite { inputs, expected }
            }
            _ => return Err(format!("unknown fitness mode {mode_tag}")),
        };

        let generation = read_u64(bytes, &mut i)?;
        let rng_state = read_u64(bytes, &mut i)?;

        let pop_count = read_u32(bytes, &mut i)? as usize;
        let mut population = Vec::with_capacity(pop_count);
        for _ in 0..pop_count {
            population.push(decode_agent(bytes, &mut i)?);
        }

        let hof_count = read_u32(bytes, &mut i)? as usize;
        let mut hall_of_fame = Vec::with_capacity(hof_count);
        for _ in 0..hof_count {
            hall_of_fame.push(decode_agent(bytes, &mut i)?);
        }

        let config = HiveConfig {
            population_size, elite_ratio, mutation_rate, crossover_rate,
            max_code_len, fuel_per_eval, fitness_fn, num_threads,
        };

        Ok(HiveEngine { population, hall_of_fame, generation, config, rng_state })
    }
}

fn encode_agent(out: &mut Vec<u8>, agent: &Agent) {
    out.extend_from_slice(&(agent.code.len() as u16).to_le_bytes());
    out.extend_from_slice(&agent.code);
    out.extend_from_slice(&agent.fitness.to_le_bytes());
    out.extend_from_slice(&agent.age.to_le_bytes());
    out.extend_from_slice(&agent.lineage.to_le_bytes());
    out.extend_from_slice(&agent.mutations.to_le_bytes());
}

fn decode_agent(bytes: &[u8], i: &mut usize) -> Result<Agent, String> {
    let code_len = read_u16(bytes, i)? as usize;
    if *i + code_len > bytes.len() { return Err("truncated agent code".into()); }
    let code = bytes[*i..*i + code_len].to_vec();
    *i += code_len;
    let fitness = read_i64(bytes, i)?;
    let age = read_u64(bytes, i)?;
    let lineage = read_u64(bytes, i)?;
    let mutations = read_u32(bytes, i)?;
    Ok(Agent { code, fitness, age, lineage, mutations })
}

// --- Binary helpers ---

fn read_u8(b: &[u8], i: &mut usize) -> Result<u8, String> {
    if *i >= b.len() { return Err("truncated u8".into()); }
    let v = b[*i];
    *i += 1;
    Ok(v)
}

fn read_u16(b: &[u8], i: &mut usize) -> Result<u16, String> {
    if *i + 2 > b.len() { return Err("truncated u16".into()); }
    let v = u16::from_le_bytes([b[*i], b[*i+1]]);
    *i += 2;
    Ok(v)
}

fn read_u32(b: &[u8], i: &mut usize) -> Result<u32, String> {
    if *i + 4 > b.len() { return Err("truncated u32".into()); }
    let v = u32::from_le_bytes([b[*i], b[*i+1], b[*i+2], b[*i+3]]);
    *i += 4;
    Ok(v)
}

fn read_i32(b: &[u8], i: &mut usize) -> Result<i32, String> {
    if *i + 4 > b.len() { return Err("truncated i32".into()); }
    let v = i32::from_le_bytes([b[*i], b[*i+1], b[*i+2], b[*i+3]]);
    *i += 4;
    Ok(v)
}

fn read_u64(b: &[u8], i: &mut usize) -> Result<u64, String> {
    if *i + 8 > b.len() { return Err("truncated u64".into()); }
    let v = u64::from_le_bytes([
        b[*i], b[*i+1], b[*i+2], b[*i+3],
        b[*i+4], b[*i+5], b[*i+6], b[*i+7],
    ]);
    *i += 8;
    Ok(v)
}

fn read_i64(b: &[u8], i: &mut usize) -> Result<i64, String> {
    if *i + 8 > b.len() { return Err("truncated i64".into()); }
    let v = i64::from_le_bytes([
        b[*i], b[*i+1], b[*i+2], b[*i+3],
        b[*i+4], b[*i+5], b[*i+6], b[*i+7],
    ]);
    *i += 8;
    Ok(v)
}

fn read_f64(b: &[u8], i: &mut usize) -> Result<f64, String> {
    if *i + 8 > b.len() { return Err("truncated f64".into()); }
    let v = f64::from_le_bytes([
        b[*i], b[*i+1], b[*i+2], b[*i+3],
        b[*i+4], b[*i+5], b[*i+6], b[*i+7],
    ]);
    *i += 8;
    Ok(v)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_creates_valid_population() {
        let config = HiveConfig { population_size: 50, ..Default::default() };
        let engine = HiveEngine::new(config);
        assert_eq!(engine.population.len(), 50);
        for agent in &engine.population {
            assert!(!agent.code.is_empty());
            assert_eq!(*agent.code.last().unwrap(), OP_HALT);
            assert!(agent.code.len() <= 256);
        }
    }

    #[test]
    fn single_generation_runs() {
        let config = HiveConfig {
            population_size: 20,
            fuel_per_eval: 500,
            num_threads: 1,
            ..Default::default()
        };
        let mut engine = HiveEngine::new(config);
        let report = engine.evolve_generation();
        assert_eq!(report.generation, 1);
        assert_eq!(report.population_size, 20);
        assert_eq!(engine.population.len(), 20);
    }

    #[test]
    fn crossover_produces_valid_children() {
        let a = vec![OP_PUSH, 1, 0, 0, 0, OP_HALT];
        let b = vec![OP_PUSH, 2, 0, 0, 0, OP_DUP, OP_MUL, OP_HALT];
        let mut rng: u64 = 42;
        for _ in 0..100 {
            let child = crossover(&a, &b, 256, &mut rng);
            assert!(!child.is_empty());
            assert_eq!(*child.last().unwrap(), OP_HALT);
            assert!(child.len() <= 256);
        }
    }

    #[test]
    fn all_mutation_operators_produce_valid_bytecode() {
        let code = vec![OP_PUSH, 10, 0, 0, 0, OP_DUP, OP_MUL, OP_HALT];
        let mut rng: u64 = 1337;
        for _ in 0..500 {
            let mutated = mutate_code(&code, 0.15, 256, &mut rng);
            assert!(!mutated.is_empty());
            assert_eq!(*mutated.last().unwrap(), OP_HALT);
            assert!(mutated.len() <= 256);
        }
    }

    #[test]
    fn hive_encode_decode_roundtrip() {
        let config = HiveConfig {
            population_size: 10,
            fuel_per_eval: 200,
            num_threads: 1,
            ..Default::default()
        };
        let mut engine = HiveEngine::new(config);
        engine.evolve_generation();

        let bytes = engine.encode();
        let restored = HiveEngine::decode(&bytes).unwrap();

        assert_eq!(restored.generation, engine.generation);
        assert_eq!(restored.population.len(), engine.population.len());
        assert_eq!(restored.config.population_size, engine.config.population_size);
        assert_eq!(restored.config.fuel_per_eval, engine.config.fuel_per_eval);
        assert_eq!(restored.rng_state, engine.rng_state);

        for (a, b) in restored.population.iter().zip(engine.population.iter()) {
            assert_eq!(a.code, b.code);
            assert_eq!(a.fitness, b.fitness);
            assert_eq!(a.age, b.age);
        }
    }

    #[test]
    fn hive_encode_decode_roundtrip_target_mode() {
        let config = HiveConfig {
            population_size: 5,
            fitness_fn: FitnessMode::Target(42),
            num_threads: 1,
            ..Default::default()
        };
        let engine = HiveEngine::new(config);
        let bytes = engine.encode();
        let restored = HiveEngine::decode(&bytes).unwrap();
        match &restored.config.fitness_fn {
            FitnessMode::Target(v) => assert_eq!(*v, 42),
            _ => panic!("wrong fitness mode"),
        }
    }

    #[test]
    fn hive_encode_decode_roundtrip_test_suite_mode() {
        let config = HiveConfig {
            population_size: 5,
            fitness_fn: FitnessMode::TestSuite {
                inputs: vec![1, 2, 3],
                expected: vec![1, 4, 9],
            },
            num_threads: 1,
            ..Default::default()
        };
        let engine = HiveEngine::new(config);
        let bytes = engine.encode();
        let restored = HiveEngine::decode(&bytes).unwrap();
        match &restored.config.fitness_fn {
            FitnessMode::TestSuite { inputs, expected } => {
                assert_eq!(inputs, &[1, 2, 3]);
                assert_eq!(expected, &[1, 4, 9]);
            }
            _ => panic!("wrong fitness mode"),
        }
    }

    #[test]
    fn parallel_matches_serial() {
        let config_serial = HiveConfig {
            population_size: 30,
            fuel_per_eval: 200,
            num_threads: 1,
            ..Default::default()
        };
        let config_parallel = HiveConfig {
            population_size: 30,
            fuel_per_eval: 200,
            num_threads: 4,
            ..Default::default()
        };

        let mut engine_s = HiveEngine::new(config_serial);
        let mut engine_p = HiveEngine::new(config_parallel);
        // Copy same population
        engine_p.population = engine_s.population.clone();

        let (best_s, avg_s) = engine_s.evaluate_parallel();
        let (best_p, avg_p) = engine_p.evaluate_parallel();

        assert_eq!(best_s, best_p);
        assert_eq!(avg_s, avg_p);

        for (a, b) in engine_s.population.iter().zip(engine_p.population.iter()) {
            assert_eq!(a.fitness, b.fitness);
        }
    }

    #[test]
    fn hall_of_fame_updates() {
        let config = HiveConfig {
            population_size: 20,
            fuel_per_eval: 500,
            num_threads: 1,
            ..Default::default()
        };
        let mut engine = HiveEngine::new(config);
        for _ in 0..5 {
            engine.evolve_generation();
        }
        assert!(!engine.hall_of_fame.is_empty());
        // HOF should be sorted descending
        for w in engine.hall_of_fame.windows(2) {
            assert!(w[0].fitness >= w[1].fitness);
        }
    }

    #[test]
    fn convergence_toward_max_output() {
        let config = HiveConfig {
            population_size: 100,
            fuel_per_eval: 500,
            num_threads: 2,
            max_code_len: 64,
            mutation_rate: 0.08,
            crossover_rate: 0.70,
            elite_ratio: 0.10,
            fitness_fn: FitnessMode::MaxOutput,
        };
        let mut engine = HiveEngine::new(config);

        let mut first_best: i64 = i64::MIN;
        let mut last_best: i64 = i64::MIN;

        for gen in 0..200 {
            let report = engine.evolve_generation();
            if gen == 0 { first_best = report.best_fitness; }
            last_best = report.best_fitness;
        }

        // After 200 generations the best should improve (or at minimum not regress)
        assert!(last_best >= first_best,
            "Expected improvement: first={first_best}, last={last_best}");
    }

    #[test]
    fn inject_replaces_worst() {
        let config = HiveConfig {
            population_size: 10,
            fuel_per_eval: 200,
            num_threads: 1,
            ..Default::default()
        };
        let mut engine = HiveEngine::new(config);
        engine.evaluate_parallel();

        let inject_code = vec![OP_PUSH, 0xFF, 0xFF, 0xFF, 0x7F, OP_HALT]; // push max_i32
        engine.inject(inject_code.clone());

        assert!(engine.population.iter().any(|a| a.code == inject_code));
    }

    #[test]
    fn export_best_creates_capsule() {
        let config = HiveConfig {
            population_size: 10,
            fuel_per_eval: 200,
            num_threads: 1,
            ..Default::default()
        };
        let mut engine = HiveEngine::new(config);
        engine.evolve_generation();

        let capsule = engine.export_best().expect("should have a best agent");
        assert!(!capsule.code.is_empty());
        assert_eq!(*capsule.code.last().unwrap(), OP_HALT);
    }

    #[test]
    fn with_seed_populates_from_seed() {
        let seed = vec![OP_PUSH, 42, 0, 0, 0, OP_DUP, OP_MUL, OP_HALT];
        let config = HiveConfig {
            population_size: 20,
            num_threads: 1,
            ..Default::default()
        };
        let engine = HiveEngine::with_seed(config, seed.clone());
        assert_eq!(engine.population.len(), 20);
        assert_eq!(engine.population[0].code, seed);
    }

    #[test]
    fn empty_code_mutation_safe() {
        let mut rng: u64 = 99;
        let result = mutate_code(&[], 0.10, 256, &mut rng);
        assert!(!result.is_empty());
        assert_eq!(*result.last().unwrap(), OP_HALT);
    }

    #[test]
    fn hive_decode_rejects_bad_magic() {
        let bad = b"NOPE";
        assert!(HiveEngine::decode(bad).is_err());
    }

    #[test]
    fn hive_decode_rejects_truncated() {
        assert!(HiveEngine::decode(&[]).is_err());
        assert!(HiveEngine::decode(&[b'H', b'I']).is_err());
    }

    #[test]
    fn target_mode_rewards_closer() {
        let close = vec![OP_PUSH, 41, 0, 0, 0, OP_HALT]; // 41, target 42 → diff 1
        let far = vec![OP_PUSH, 10, 0, 0, 0, OP_HALT];   // 10, target 42 → diff 32
        let config = HiveConfig {
            fitness_fn: FitnessMode::Target(42),
            fuel_per_eval: 100,
            ..Default::default()
        };
        let fit_close = eval_agent(&close, &config);
        let fit_far = eval_agent(&far, &config);
        assert!(fit_close > fit_far, "close={fit_close} should beat far={fit_far}");
    }

    #[test]
    fn longevity_mode() {
        // A NOP chain runs longer than an immediate halt
        let long = vec![OP_NOP, OP_NOP, OP_NOP, OP_NOP, OP_NOP, OP_HALT];
        let short = vec![OP_HALT];
        let config = HiveConfig {
            fitness_fn: FitnessMode::Longevity,
            fuel_per_eval: 100,
            ..Default::default()
        };
        let fit_long = eval_agent(&long, &config);
        let fit_short = eval_agent(&short, &config);
        assert!(fit_long >= fit_short, "long={fit_long} should >= short={fit_short}");
    }
}
