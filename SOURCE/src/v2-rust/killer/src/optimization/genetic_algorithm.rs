/// Week 6 Phase 3: Genetic Algorithm Foundation
/// 
/// Implements genetic algorithm framework for automated optimization parameter discovery
/// Searches parameter space to maximize: speedup × (1 - code_size_ratio) × (1 - compile_time_ratio)

use std::fmt;
use std::time::{SystemTime, UNIX_EPOCH};

/// Simple pseudo-random number generator (no external dependencies)
struct SimpleRng {
    seed: u64,
    counter: u64,
}

impl SimpleRng {
    fn new() -> Self {
        let seed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64;
        SimpleRng { seed, counter: 0 }
    }

    fn next(&mut self) -> u64 {
        // Linear congruential generator with counter to avoid patterns
        self.counter = self.counter.wrapping_add(1);
        let mixed = self.seed.wrapping_add(self.counter);
        self.seed = mixed.wrapping_mul(1664525).wrapping_add(1013904223);
        self.seed
    }

    fn gen_range(&mut self, min: u64, max: u64) -> u64 {
        if max <= min {
            return min;
        }
        min + (self.next() % (max - min))
    }

    fn gen_bool(&mut self, probability: f64) -> bool {
        if probability <= 0.0 {
            return false;
        }
        if probability >= 1.0 {
            return true;
        }
        (self.next() as f64 / u64::MAX as f64) < probability
    }
}

/// Optimization parameters that genetic algorithm will tune
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OptimizationGene {
    /// Unroll factor: 1, 2, 4, 8, 16, 32
    pub unroll_factor: u8,
    
    /// Enable SIMD vectorization hints
    pub vectorization: bool,
    
    /// Add inline hints to functions
    pub inline_hints: bool,
    
    /// Enable memory prefetching
    pub prefetch: bool,
    
    /// LLVM optimization level: 0=O0, 1=O1, 2=O2, 3=O3, 4=Oz
    pub opt_level: u8,
}

impl OptimizationGene {
    /// Create random gene (used for initial population)
    fn random(rng: &mut SimpleRng) -> Self {
        let unroll_opts = [1u8, 2, 4, 8, 16, 32];
        let idx = rng.gen_range(0, 6) as usize;
        OptimizationGene {
            unroll_factor: unroll_opts[idx],
            vectorization: rng.gen_bool(0.5),
            inline_hints: rng.gen_bool(0.5),
            prefetch: rng.gen_bool(0.5),
            opt_level: (rng.gen_range(0, 5)) as u8,
        }
    }

    /// Mutate a single parameter
    fn mutate(&mut self, rng: &mut SimpleRng) {
        let mutation_type = rng.gen_range(0, 5);

        match mutation_type {
            0 => {
                // Mutate unroll factor
                let unroll_opts = [1u8, 2, 4, 8, 16, 32];
                let idx = rng.gen_range(0, 6) as usize;
                self.unroll_factor = unroll_opts[idx];
            }
            1 => {
                // Flip vectorization
                self.vectorization = !self.vectorization;
            }
            2 => {
                // Flip inline hints
                self.inline_hints = !self.inline_hints;
            }
            3 => {
                // Flip prefetch
                self.prefetch = !self.prefetch;
            }
            4 => {
                // Change opt level
                self.opt_level = (rng.gen_range(0, 5)) as u8;
            }
            _ => {}
        }
    }
}

impl fmt::Display for OptimizationGene {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Gene(unroll:{}, vec:{}, inline:{}, prefetch:{}, opt:{})",
            self.unroll_factor,
            if self.vectorization { "Y" } else { "N" },
            if self.inline_hints { "Y" } else { "N" },
            if self.prefetch { "Y" } else { "N" },
            match self.opt_level {
                0 => "O0",
                1 => "O1",
                2 => "O2",
                3 => "O3",
                4 => "Oz",
                _ => "?",
            }
        )
    }
}

/// Individual in genetic algorithm population
/// Represents one parameter combination and its fitness
#[derive(Debug, Clone)]
pub struct Individual {
    pub gene: OptimizationGene,
    pub fitness: Option<f64>, // None = not yet evaluated
}

impl Individual {
    /// Create new individual with random gene
    pub fn random() -> Self {
        let mut rng = SimpleRng::new();
        Individual {
            gene: OptimizationGene::random(&mut rng),
            fitness: None,
        }
    }

    /// Create individual with specific gene
    pub fn with_gene(gene: OptimizationGene) -> Self {
        Individual { gene, fitness: None }
    }

    /// Set fitness score
    pub fn set_fitness(&mut self, fitness: f64) {
        self.fitness = Some(fitness);
    }

    /// Get fitness (panics if not evaluated)
    pub fn get_fitness(&self) -> f64 {
        self.fitness.expect("Fitness not evaluated")
    }

    /// Create offspring from this individual (mutation)
    pub fn mutate(&self) -> Individual {
        let mut rng = SimpleRng::new();
        let mut child_gene = self.gene;
        child_gene.mutate(&mut rng);
        Individual {
            gene: child_gene,
            fitness: None,
        }
    }

    /// Create offspring from two parents (crossover)
    pub fn crossover(&self, other: &Individual) -> Individual {
        let mut rng = SimpleRng::new();
        
        // Randomly choose each parameter from either parent
        let gene = OptimizationGene {
            unroll_factor: if rng.gen_bool(0.5) {
                self.gene.unroll_factor
            } else {
                other.gene.unroll_factor
            },
            vectorization: if rng.gen_bool(0.5) {
                self.gene.vectorization
            } else {
                other.gene.vectorization
            },
            inline_hints: if rng.gen_bool(0.5) {
                self.gene.inline_hints
            } else {
                other.gene.inline_hints
            },
            prefetch: if rng.gen_bool(0.5) {
                self.gene.prefetch
            } else {
                other.gene.prefetch
            },
            opt_level: if rng.gen_bool(0.5) {
                self.gene.opt_level
            } else {
                other.gene.opt_level
            },
        };

        Individual { gene, fitness: None }
    }
}

/// Performance metrics for fitness evaluation
#[derive(Debug, Clone)]
pub struct PerformanceMetrics {
    pub speedup: f64,
    pub baseline_binary_size_kb: f64,
    pub optimized_binary_size_kb: f64,
    pub baseline_compile_time_ms: f64,
    pub optimized_compile_time_ms: f64,
}

impl PerformanceMetrics {
    /// Calculate fitness score: speedup × (1 - size_ratio) × (1 - compile_time_ratio)
    pub fn fitness(&self) -> f64 {
        let size_ratio = self.optimized_binary_size_kb / self.baseline_binary_size_kb;
        let compile_ratio = self.optimized_compile_time_ms / self.baseline_compile_time_ms;

        // Penalize large code size increases and long compile times
        // Reward speedup exponentially
        let size_penalty = (1.0 - (size_ratio - 1.0).max(0.0).min(1.0)) * 0.5 + 0.5;
        let compile_penalty = (1.0 - (compile_ratio - 1.0).max(0.0).min(1.0)) * 0.3 + 0.7;

        self.speedup * size_penalty * compile_penalty
    }
}

/// Genetic algorithm population and evolution
pub struct GeneticOptimizer {
    pub population: Vec<Individual>,
    pub generation: u32,
    pub best_individual: Option<Individual>,
    rng: SimpleRng,
}

impl GeneticOptimizer {
    /// Create optimizer with initial population
    pub fn new(population_size: usize) -> Self {
        let mut rng = SimpleRng::new();
        let population = (0..population_size)
            .map(|_| {
                Individual::with_gene(OptimizationGene::random(&mut rng))
            })
            .collect();

        GeneticOptimizer {
            population,
            generation: 0,
            best_individual: None,
            rng,
        }
    }

    /// Evaluate fitness for all unevaluated individuals
    pub fn evaluate_population(&mut self, evaluator: impl Fn(&OptimizationGene) -> f64) {
        for individual in &mut self.population {
            if individual.fitness.is_none() {
                let fitness = evaluator(&individual.gene);
                individual.set_fitness(fitness);
            }
        }

        // Track best individual
        let best = self
            .population
            .iter()
            .max_by(|a, b| a.get_fitness().partial_cmp(&b.get_fitness()).unwrap());

        if let Some(best_ind) = best {
            self.best_individual = Some(best_ind.clone());
        }
    }

    /// Evolve population to next generation
    pub fn next_generation(&mut self, mutation_rate: f64) {
        let mut new_population = vec![];

        // Elitism: keep top 2 individuals
        let mut pop_sorted = self.population.clone();
        pop_sorted.sort_by(|a, b| b.get_fitness().partial_cmp(&a.get_fitness()).unwrap());

        new_population.push(pop_sorted[0].clone());
        if pop_sorted.len() > 1 {
            new_population.push(pop_sorted[1].clone());
        }

        // Generate rest of population via crossover and mutation
        while new_population.len() < self.population.len() {
            let parent1_idx = (self.rng.next() % self.population.len() as u64) as usize;
            let parent2_idx = (self.rng.next() % self.population.len() as u64) as usize;

            let parent1 = self.population[parent1_idx].clone();
            let parent2 = self.population[parent2_idx].clone();

            let mut child = parent1.crossover(&parent2);

            // Apply mutation
            if self.rng.gen_bool(mutation_rate) {
                child = child.mutate();
            }

            new_population.push(child);
        }

        self.population = new_population;
        self.generation += 1;
    }

    /// Get summary statistics
    pub fn generation_stats(&self) -> GenerationStats {
        let fitnesses: Vec<f64> = self.population.iter().map(|i| i.get_fitness()).collect();
        
        let best = *fitnesses.iter().max_by(|a, b| a.partial_cmp(b).unwrap()).unwrap();
        let worst = *fitnesses.iter().min_by(|a, b| a.partial_cmp(b).unwrap()).unwrap();
        let avg = fitnesses.iter().sum::<f64>() / fitnesses.len() as f64;

        GenerationStats {
            generation: self.generation,
            best_fitness: best,
            worst_fitness: worst,
            avg_fitness: avg,
            population_size: self.population.len(),
        }
    }
}

/// Statistics for a generation
#[derive(Debug, Clone)]
pub struct GenerationStats {
    pub generation: u32,
    pub best_fitness: f64,
    pub worst_fitness: f64,
    pub avg_fitness: f64,
    pub population_size: usize,
}

impl fmt::Display for GenerationStats {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Gen {}: best={:.4}, worst={:.4}, avg={:.4}",
            self.generation, self.best_fitness, self.worst_fitness, self.avg_fitness
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_optimization_gene_creation() {
        let mut rng = SimpleRng::new();
        let gene = OptimizationGene::random(&mut rng);
        assert!(matches!(gene.unroll_factor, 1 | 2 | 4 | 8 | 16 | 32));
        assert!(gene.opt_level <= 4);
    }

    #[test]
    fn test_individual_creation() {
        let ind = Individual::random();
        assert!(ind.fitness.is_none());
    }

    #[test]
    fn test_individual_fitness() {
        let mut ind = Individual::random();
        ind.set_fitness(42.5);
        assert_eq!(ind.get_fitness(), 42.5);
    }

    #[test]
    fn test_individual_mutation() {
        let parent = Individual::random();
        let child = parent.mutate();
        
        // Child should be different (with high probability)
        assert_ne!(parent.gene, child.gene);
    }

    #[test]
    fn test_individual_crossover() {
        let parent1 = Individual::random();
        let parent2 = Individual::random();
        let child = parent1.crossover(&parent2);
        
        // Child gets parameters from parents
        assert_eq!(child.fitness, None);
    }

    #[test]
    fn test_performance_metrics_fitness() {
        let metrics = PerformanceMetrics {
            speedup: 2.0,
            baseline_binary_size_kb: 100.0,
            optimized_binary_size_kb: 110.0,
            baseline_compile_time_ms: 100.0,
            optimized_compile_time_ms: 120.0,
        };

        let fitness = metrics.fitness();
        
        // Fitness should reward speedup and penalize size/time increases
        assert!(fitness > 0.0);
        assert!(fitness < 2.0); // Penalized, not full 2x
    }

    #[test]
    fn test_optimizer_creation() {
        let optimizer = GeneticOptimizer::new(10);
        assert_eq!(optimizer.population.len(), 10);
        assert_eq!(optimizer.generation, 0);
        assert!(optimizer.best_individual.is_none());
    }

    #[test]
    fn test_optimizer_evaluation() {
        let mut optimizer = GeneticOptimizer::new(5);
        
        // Simple fitness function: reward higher opt_level
        optimizer.evaluate_population(|gene| gene.opt_level as f64);

        // All should be evaluated
        for ind in &optimizer.population {
            assert!(ind.fitness.is_some());
        }

        // Best individual should be set
        assert!(optimizer.best_individual.is_some());
    }

    #[test]
    fn test_optimizer_evolution() {
        let mut optimizer = GeneticOptimizer::new(10);
        optimizer.evaluate_population(|gene| gene.opt_level as f64);

        let initial_best = optimizer.best_individual.as_ref().unwrap().get_fitness();

        optimizer.next_generation(0.5);
        optimizer.evaluate_population(|gene| gene.opt_level as f64);

        // Generation should increment
        assert_eq!(optimizer.generation, 1);

        // Best fitness should be maintained or improve
        assert!(optimizer.best_individual.as_ref().unwrap().get_fitness() >= initial_best);
    }

    #[test]
    fn test_generation_stats() {
        let mut optimizer = GeneticOptimizer::new(5);
        optimizer.evaluate_population(|gene| gene.opt_level as f64);

        let stats = optimizer.generation_stats();
        
        assert_eq!(stats.generation, 0);
        assert_eq!(stats.population_size, 5);
        assert!(stats.best_fitness >= stats.worst_fitness);
        assert!(stats.avg_fitness > 0.0);
    }

    #[test]
    fn test_optimizer_basic_evolution() {
        let mut optimizer = GeneticOptimizer::new(10);
        optimizer.evaluate_population(|gene| gene.opt_level as f64);

        let initial_best = optimizer.best_individual.as_ref().unwrap().get_fitness();

        optimizer.next_generation(0.5);
        optimizer.evaluate_population(|gene| gene.opt_level as f64);

        // Generation should increment
        assert_eq!(optimizer.generation, 1);

        // Best fitness should be maintained or improve
        assert!(optimizer.best_individual.as_ref().unwrap().get_fitness() >= initial_best);
    }
}
