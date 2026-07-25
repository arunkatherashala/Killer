/// Week 6 Phase 4: Loop Classification and Parameter Discovery
/// 
/// Classifies loops into types (CPU-bound, memory-bound, mixed) and uses genetic algorithm
/// to discover optimal optimization parameters for each type

use crate::optimization::{OptimizationGene, GeneticOptimizer};
use std::collections::HashMap;

/// Loop type classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LoopType {
    /// CPU-intensive (arithmetic, special functions)
    CpuBound,
    /// Memory-intensive (cache misses, irregular access)
    MemoryBound,
    /// Mixed computation and memory access
    Mixed,
}

impl std::fmt::Display for LoopType {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            LoopType::CpuBound => write!(f, "CPU-Bound"),
            LoopType::MemoryBound => write!(f, "Memory-Bound"),
            LoopType::Mixed => write!(f, "Mixed"),
        }
    }
}

/// Features extracted from loop for classification
#[derive(Debug, Clone)]
pub struct LoopFeatures {
    /// Memory access patterns (0.0-1.0, 0=sequential/predictable, 1=random/irregular)
    pub memory_irregularity: f64,
    
    /// Arithmetic intensity (ops per byte loaded)
    pub arithmetic_intensity: f64,
    
    /// Branch density (branches per instruction)
    pub branch_density: f64,
    
    /// Loop trip count (estimated iterations)
    pub trip_count: u64,
    
    /// whether loop is vectorizable
    pub vectorizable: bool,
}

impl LoopFeatures {
    /// Classify loop into type based on features
    pub fn classify(&self) -> LoopType {
        // Heuristic classification
        // High arithmetic intensity + low memory irregularity = CPU-bound
        // Low arithmetic intensity + high memory irregularity = Memory-bound
        // Otherwise = Mixed
        
        let cpu_score = self.arithmetic_intensity * (1.0 - self.memory_irregularity);
        let mem_score = self.memory_irregularity;
        
        if cpu_score > 2.0 && mem_score < 0.4 {
            LoopType::CpuBound
        } else if mem_score > 0.6 && cpu_score < 1.0 {
            LoopType::MemoryBound
        } else {
            LoopType::Mixed
        }
    }
}

/// Optimal parameters for a loop type (discovered via GA)
#[derive(Debug, Clone)]
pub struct OptimalParameters {
    pub loop_type: LoopType,
    pub gene: OptimizationGene,
    pub fitness: f64,
    pub confidence: f64,  // 0.0-1.0, higher = more runs confirm this parameter
}

impl std::fmt::Display for OptimalParameters {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "{}: {} (fitness={:.2}, confidence={:.1}%)",
            self.loop_type,
            self.gene,
            self.fitness,
            self.confidence * 100.0
        )
    }
}

/// Parameter discovery engine using genetic algorithm
pub struct ParameterDiscovery {
    /// Discovered parameters by loop type
    parameters: HashMap<LoopType, OptimalParameters>,
    
    /// Number of GA runs per loop type
    runs_per_type: usize,
    
    /// GA population size
    population_size: usize,
    
    /// GA generations per run
    generations: u32,
}

impl ParameterDiscovery {
    /// Create new discovery engine
    pub fn new(population_size: usize, generations: u32) -> Self {
        ParameterDiscovery {
            parameters: HashMap::new(),
            runs_per_type: 3,
            population_size,
            generations,
        }
    }

    /// Discover optimal parameters for a loop type
    pub fn discover_parameters(
        &mut self,
        loop_type: LoopType,
        fitness_fn: impl Fn(&OptimizationGene) -> f64 + Copy,
    ) {
        let mut best_gene: Option<OptimizationGene> = None;
        let mut best_fitness: f64 = 0.0;
        let mut success_count = 0;

        // Run GA multiple times and average results
        for _run in 0..self.runs_per_type {
            let mut optimizer = GeneticOptimizer::new(self.population_size);

            // Evolve for specified generations
            for _ in 0..self.generations {
                optimizer.evaluate_population(fitness_fn);
                optimizer.next_generation(0.4); // 40% mutation rate
            }

            // Final evaluation
            optimizer.evaluate_population(fitness_fn);
            let stats = optimizer.generation_stats();

            if stats.best_fitness > 0.0 {
                success_count += 1;
                
                if let Some(ind) = optimizer.best_individual.as_ref() {
                    let fitness = ind.get_fitness();
                    if fitness > best_fitness {
                        best_gene = Some(ind.gene);
                        best_fitness = fitness;
                    }
                }
            }
        }

        // Store discovered parameters
        if let Some(gene) = best_gene {
            let confidence = success_count as f64 / self.runs_per_type as f64;
            self.parameters.insert(
                loop_type,
                OptimalParameters {
                    loop_type,
                    gene,
                    fitness: best_fitness,
                    confidence,
                },
            );
        }
    }

    /// Get optimal parameters for loop type (or discover if not found)
    pub fn get_parameters(
        &self,
        loop_type: LoopType,
    ) -> Option<&OptimalParameters> {
        self.parameters.get(&loop_type)
    }

    /// Get all discovered parameters
    pub fn all_parameters(&self) -> impl Iterator<Item = &OptimalParameters> {
        self.parameters.values()
    }

    /// Recommend parameters for loop
    pub fn recommend(&self, features: &LoopFeatures) -> Option<OptimizationGene> {
        let loop_type = features.classify();
        self.parameters
            .get(&loop_type)
            .map(|params| params.gene)
    }
}

/// Parameter recommendation system
pub struct ParameterRecommender {
    /// Discovered optimal parameters
    discovery: ParameterDiscovery,
}

impl ParameterRecommender {
    /// Create recommender with discovery engine
    pub fn new(population_size: usize, generations: u32) -> Self {
        ParameterRecommender {
            discovery: ParameterDiscovery::new(population_size, generations),
        }
    }

    /// Run full discovery for all loop types
    pub fn discover_all(&mut self) {
        // CPU-bound fitness function
        self.discovery.discover_parameters(LoopType::CpuBound, |gene| {
            // Reward: high opt_level, unrolling, vectorization, inline hints
            let opt = gene.opt_level as f64 * 10.0;
            let unroll = (gene.unroll_factor as f64).min(32.0) / 4.0;
            let vec = if gene.vectorization { 2.5 } else { 1.0 };
            let inline = if gene.inline_hints { 1.5 } else { 1.0 };

            opt * unroll * vec * inline
        });

        // Memory-bound fitness function
        self.discovery.discover_parameters(LoopType::MemoryBound, |gene| {
            // Reward: moderate opt_level, small code, prefetch
            let opt = gene.opt_level as f64 * 5.0;
            let size_penalty = 1.0 / (gene.unroll_factor as f64 / 2.0);
            let prefetch = if gene.prefetch { 2.0 } else { 1.0 };
            let small_code = if !gene.inline_hints { 1.3 } else { 1.0 };

            opt * size_penalty * prefetch * small_code
        });

        // Mixed fitness function
        self.discovery.discover_parameters(LoopType::Mixed, |gene| {
            // Balance: moderate rewards across all factors
            let opt = gene.opt_level as f64 * 6.0;
            let unroll = ((gene.unroll_factor as f64).log2() + 1.0) * 1.5;
            let vec = if gene.vectorization { 1.8 } else { 1.0 };
            let prefetch = if gene.prefetch { 1.2 } else { 1.0 };

            opt * unroll * vec * prefetch
        });
    }

    /// Get parameters for loop
    pub fn get_parameters(&self, loop_type: LoopType) -> Option<&OptimalParameters> {
        self.discovery.get_parameters(loop_type)
    }

    /// Recommend parameters for loop features
    pub fn recommend(&self, features: &LoopFeatures) -> Option<OptimizationGene> {
        self.discovery.recommend(features)
    }

    /// Print summary of discovered parameters
    pub fn print_summary(&self) {
        println!("\n=== Discovered Optimal Parameters ===\n");
        for params in self.discovery.all_parameters() {
            println!("{}", params);
        }
        println!();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_loop_type_classification() {
        // CPU-bound loop
        let cpu_features = LoopFeatures {
            memory_irregularity: 0.2,
            arithmetic_intensity: 3.0,
            branch_density: 0.05,
            trip_count: 1000,
            vectorizable: true,
        };
        assert_eq!(cpu_features.classify(), LoopType::CpuBound);

        // Memory-bound loop
        let mem_features = LoopFeatures {
            memory_irregularity: 0.8,
            arithmetic_intensity: 0.5,
            branch_density: 0.1,
            trip_count: 10000,
            vectorizable: false,
        };
        assert_eq!(mem_features.classify(), LoopType::MemoryBound);

        // Mixed loop
        let mixed_features = LoopFeatures {
            memory_irregularity: 0.5,
            arithmetic_intensity: 1.0,
            branch_density: 0.2,
            trip_count: 5000,
            vectorizable: true,
        };
        assert_eq!(mixed_features.classify(), LoopType::Mixed);
    }

    #[test]
    fn test_parameter_discovery_cpu_bound() {
        let mut discovery = ParameterDiscovery::new(15, 5);

        discovery.discover_parameters(LoopType::CpuBound, |gene| {
            let opt = gene.opt_level as f64 * 10.0;
            let unroll = (gene.unroll_factor as f64).min(32.0) / 4.0;
            let vec = if gene.vectorization { 2.5 } else { 1.0 };
            let inline = if gene.inline_hints { 1.5 } else { 1.0 };

            opt * unroll * vec * inline
        });

        let params = discovery.get_parameters(LoopType::CpuBound);
        assert!(params.is_some());
        let p = params.unwrap();
        assert!(p.fitness > 50.0); // Should have good fitness
        assert!(p.gene.vectorization); // Should prefer vectorization
        println!("✅ CPU-bound discovery: {}", p);
    }

    #[test]
    fn test_parameter_discovery_memory_bound() {
        let mut discovery = ParameterDiscovery::new(15, 5);

        discovery.discover_parameters(LoopType::MemoryBound, |gene| {
            let opt = gene.opt_level as f64 * 5.0;
            let size_penalty = 1.0 / (gene.unroll_factor as f64 / 2.0);
            let prefetch = if gene.prefetch { 2.0 } else { 1.0 };
            let small_code = if !gene.inline_hints { 1.3 } else { 1.0 };

            opt * size_penalty * prefetch * small_code
        });

        let params = discovery.get_parameters(LoopType::MemoryBound);
        assert!(params.is_some());
        let p = params.unwrap();
        assert!(p.fitness > 20.0);
        assert!(p.gene.prefetch); // Should prefer prefetch
        println!("✅ Memory-bound discovery: {}", p);
    }

    #[test]
    fn test_parameter_recommender() {
        let mut recommender = ParameterRecommender::new(15, 5);
        recommender.discover_all();

        // Test CPU-bound recommendation
        let cpu_features = LoopFeatures {
            memory_irregularity: 0.2,
            arithmetic_intensity: 3.0,
            branch_density: 0.05,
            trip_count: 1000,
            vectorizable: true,
        };

        let recommendation = recommender.recommend(&cpu_features);
        assert!(recommendation.is_some());
        println!("✅ Parameter recommender works");
    }

    #[test]
    fn test_confidence_tracking() {
        let mut discovery = ParameterDiscovery::new(10, 3);

        discovery.discover_parameters(LoopType::CpuBound, |gene| {
            gene.opt_level as f64 * 5.0
        });

        if let Some(params) = discovery.get_parameters(LoopType::CpuBound) {
            assert!(params.confidence > 0.0);
            assert!(params.confidence <= 1.0);
            println!(
                "✅ Confidence: {:.1}%",
                params.confidence * 100.0
            );
        }
    }

    #[test]
    fn test_full_discovery_pipeline() {
        let mut recommender = ParameterRecommender::new(20, 8);
        recommender.discover_all();

        // Should have discovered parameters for all types
        assert!(recommender.get_parameters(LoopType::CpuBound).is_some());
        assert!(recommender.get_parameters(LoopType::MemoryBound).is_some());
        assert!(recommender.get_parameters(LoopType::Mixed).is_some());

        println!("✅ Full discovery pipeline complete");
        recommender.print_summary();
    }
}
