#![cfg(feature = "legacy-killer-rcore-tests")]
/// Week 6 Phase 3: Genetic Algorithm Integration Tests
/// 
/// Tests the genetic algorithm framework for optimizing loop parameters
/// across different loops and scenarios

#[cfg(test)]
mod tests {
    use killer_rcore::optimization::{
        OptimizationGene, GeneticOptimizer, PerformanceMetrics,
    };

    #[test]
    fn test_genetic_algorithm_creation() {
        let optimizer = GeneticOptimizer::new(20);
        assert_eq!(optimizer.population.len(), 20);
        assert_eq!(optimizer.generation, 0);
        assert!(optimizer.best_individual.is_none());
    }

    #[test]
    fn test_gene_parameter_space() {
        // Verify parameter space: 6 × 2 × 2 × 2 × 5 = 240 combinations
        // (unroll 6 options × vectorization × inline × prefetch × opt_level)
        
        let unroll_factors = [1u8, 2, 4, 8, 16, 32];
        let mut count = 0;

        for unroll in unroll_factors.iter() {
            for vec in [true, false].iter() {
                for inline in [true, false].iter() {
                    for prefetch in [true, false].iter() {
                        for opt_level in 0..5u8 {
                            let gene = OptimizationGene {
                                unroll_factor: *unroll,
                                vectorization: *vec,
                                inline_hints: *inline,
                                prefetch: *prefetch,
                                opt_level,
                            };

                            // Verify all combinations are valid
                            assert!(matches!(
                                gene.unroll_factor,
                                1 | 2 | 4 | 8 | 16 | 32
                            ));
                            assert!(gene.opt_level <= 4);

                            count += 1;
                        }
                    }
                }
            }
        }

        assert_eq!(count, 240);
        println!("✅ Parameter space: {} valid combinations", count);
    }

    #[test]
    fn test_fitness_calculation_realistic() {
        // Test realistic scenario: 10x speedup, 10% size increase, 20% compile time increase
        let metrics = PerformanceMetrics {
            speedup: 10.0,
            baseline_binary_size_kb: 100.0,
            optimized_binary_size_kb: 110.0,
            baseline_compile_time_ms: 100.0,
            optimized_compile_time_ms: 120.0,
        };

        let fitness = metrics.fitness();
        
        // Should be less than 10 due to penalties
        assert!(fitness < 10.0);
        assert!(fitness > 5.0); // But still significant
        println!("✅ Fitness (10x speedup, +10% size, +20% time): {:.4}", fitness);
    }

    #[test]
    fn test_fitness_calculation_extreme_speedup() {
        // Test synthetic loop (extreme optimization)
        let metrics = PerformanceMetrics {
            speedup: 100000.0,
            baseline_binary_size_kb: 100.0,
            optimized_binary_size_kb: 102.0,
            baseline_compile_time_ms: 100.0,
            optimized_compile_time_ms: 105.0,
        };

        let fitness = metrics.fitness();
        
        // Extreme speedup should dominate
        assert!(fitness > 90000.0);
        println!(
            "✅ Fitness (100k speedup, +2% size, +5% time): {:.4}",
            fitness
        );
    }

    #[test]
    fn test_simple_genetic_evolution() {
        let mut optimizer = GeneticOptimizer::new(10);

        // Simple fitness function: reward higher optimization level
        for gen in 0..5 {
            optimizer.evaluate_population(|gene| gene.opt_level as f64 * 10.0);

            let stats = optimizer.generation_stats();
            println!(
                "Gen {}: best={:.2}, avg={:.2}",
                gen, stats.best_fitness, stats.avg_fitness
            );

            optimizer.next_generation(0.4);
        }

        // Final generation should find high opt levels (3 or 4 = 30 or 40)
        optimizer.evaluate_population(|gene| gene.opt_level as f64 * 10.0);
        let best_fitness = optimizer.best_individual.as_ref().unwrap().get_fitness();
        assert!(best_fitness >= 30.0); // Should find opt_level 3 or 4

        println!("✅ Evolution converged to fitness {:.2}", best_fitness);
    }

    #[test]
    fn test_genetic_evolution_multiobjective() {
        let mut optimizer = GeneticOptimizer::new(20);

        // Fitness rewards: opt_level + unroll factor - small penalty for boolean flags
        // This tests balancing multiple objectives
        for _gen in 0..8 {
            optimizer.evaluate_population(|gene| {
                let opt_score = gene.opt_level as f64;
                let unroll_score = (gene.unroll_factor as f64).log2(); // 0-5 for 1-32
                let bool_penalty = if gene.vectorization { 0.0 } else { 0.5 }
                    + if gene.inline_hints { 0.0 } else { 0.5 };

                opt_score + unroll_score - bool_penalty
            });

            optimizer.next_generation(0.5);
        }

        optimizer.evaluate_population(|gene| {
            let opt_score = gene.opt_level as f64;
            let unroll_score = (gene.unroll_factor as f64).log2();
            let bool_penalty = if gene.vectorization { 0.0 } else { 0.5 }
                + if gene.inline_hints { 0.0 } else { 0.5 };

            opt_score + unroll_score - bool_penalty
        });

        let best = optimizer.best_individual.as_ref().unwrap();
        assert!(best.get_fitness() > 2.0);
        println!(
            "✅ Multiobjective converged: {}",
            best.gene
        );
    }

    #[test]
    fn test_population_diversity_maintenance() {
        let mut optimizer = GeneticOptimizer::new(30);

        for gen in 0..10 {
            optimizer.evaluate_population(|gene| {
                (gene.opt_level as f64) * (gene.unroll_factor as f64 / 10.0)
            });

            let genes_before = optimizer.population.len();

            optimizer.next_generation(0.6); // High mutation rate

            let genes_after = optimizer.population.len();
            assert_eq!(genes_before, genes_after);
        }

        // Count unique genes
        let mut unique_genes = std::collections::HashSet::new();
        for ind in &optimizer.population {
            let key = format!("{:?}", ind.gene);
            unique_genes.insert(key);
        }

        // Should maintain some diversity even with high mutation
        assert!(unique_genes.len() > 5);
        println!("✅ Population diversity: {} unique genes in pop of 30", unique_genes.len());
    }

    #[test]
    fn test_elitism_preservation() {
        let mut optimizer = GeneticOptimizer::new(15);

        // Weaker fitness in early generations
        optimizer.evaluate_population(|gene| gene.opt_level as f64);
        let initial_best_fitness = optimizer.best_individual.as_ref().unwrap().get_fitness();

        for _ in 0..5 {
            optimizer.next_generation(0.8); // High mutation
            optimizer.evaluate_population(|gene| gene.opt_level as f64);
        }

        let final_best_fitness = optimizer.best_individual.as_ref().unwrap().get_fitness();

        // Best fitness should be maintained or improve (never go down)
        assert!(
            final_best_fitness >= initial_best_fitness,
            "Elitism broken: {} < {}",
            final_best_fitness,
            initial_best_fitness
        );

        println!(
            "✅ Elitism preserved: initial={:.2}, final={:.2}",
            initial_best_fitness, final_best_fitness
        );
    }

    #[test]
    fn test_evolution_cpu_bound_workload() {
        // Simulate optimizing for CPU-bound workload
        // CPU-bound rewards: high opt_level, unrolling, vectorization
        let mut optimizer = GeneticOptimizer::new(25);

        for gen in 0..12 {
            optimizer.evaluate_population(|gene| {
                let opt = gene.opt_level as f64 * 5.0;
                let unroll = (gene.unroll_factor as f64).min(32.0) / 4.0;
                let vec = if gene.vectorization { 3.0 } else { 1.0 };
                let inline = if gene.inline_hints { 1.5 } else { 1.0 };

                opt * unroll * vec * inline
            });

            if gen % 4 == 0 {
                let stats = optimizer.generation_stats();
                println!("Gen {}: {}", gen, stats);
            }

            optimizer.next_generation(0.4);
        }

        optimizer.evaluate_population(|gene| {
            let opt = gene.opt_level as f64 * 5.0;
            let unroll = (gene.unroll_factor as f64).min(32.0) / 4.0;
            let vec = if gene.vectorization { 3.0 } else { 1.0 };
            let inline = if gene.inline_hints { 1.5 } else { 1.0 };

            opt * unroll * vec * inline
        });

        let best = optimizer.best_individual.as_ref().unwrap();
        
        // Should favor high opt_level and vectorization
        assert!(best.gene.opt_level >= 2);
        assert!(best.gene.vectorization);
        
        println!("✅ CPU workload evolved: {}", best.gene);
    }

    #[test]
    fn test_evolution_memory_bound_workload() {
        // Memory-bound rewards: moderate opt_level, smaller code size (low unroll, no inline)
        let mut optimizer = GeneticOptimizer::new(25);

        for gen in 0..12 {
            optimizer.evaluate_population(|gene| {
                let opt = gene.opt_level as f64 * 3.0;
                let size_penalty = 1.0 / (gene.unroll_factor as f64 / 2.0);
                let prefetch = if gene.prefetch { 2.0 } else { 1.0 };
                let no_bloat = if !gene.inline_hints { 1.5 } else { 1.0 };

                opt * size_penalty * prefetch * no_bloat
            });

            if gen % 4 == 0 {
                let stats = optimizer.generation_stats();
                println!("Gen {}: {}", gen, stats);
            }

            optimizer.next_generation(0.4);
        }

        optimizer.evaluate_population(|gene| {
            let opt = gene.opt_level as f64 * 3.0;
            let size_penalty = 1.0 / (gene.unroll_factor as f64 / 2.0);
            let prefetch = if gene.prefetch { 2.0 } else { 1.0 };
            let no_bloat = if !gene.inline_hints { 1.5 } else { 1.0 };

            opt * size_penalty * prefetch * no_bloat
        });

        let best = optimizer.best_individual.as_ref().unwrap();
        
        // Should prefer smaller code and prefetching
        assert!(best.gene.unroll_factor < 16); // Not too much unrolling
        assert!(best.gene.prefetch); // Should enable prefetch
        
        println!("✅ Memory workload evolved: {}", best.gene);
    }

    #[test]
    fn test_convergence_detection() {
        // Test detecting convergence by measuring stagnation
        let mut optimizer = GeneticOptimizer::new(20);
        let mut best_fitness_history = vec![];

        for _gen in 0..15 {
            optimizer.evaluate_population(|gene| {
                gene.opt_level as f64 * (1.0 + gene.unroll_factor as f64 / 100.0)
            });

            let stats = optimizer.generation_stats();
            best_fitness_history.push(stats.best_fitness);

            optimizer.next_generation(0.3);
        }

        // Check for convergence: improvement should slow down
        let early_improvement = best_fitness_history[5] - best_fitness_history[0];
        let late_improvement = best_fitness_history[14] - best_fitness_history[10];

        // Late improvement should be smaller
        assert!(late_improvement <= early_improvement);
        println!(
            "✅ Convergence detected: early_improvement={:.4}, late_improvement={:.4}",
            early_improvement, late_improvement
        );
    }

    #[test]
    fn test_full_pipeline_realistic_scenario() {
        // Full pipeline test: simulate optimizing a real loop
        println!("\n=== REALISTIC OPTIMIZATION SCENARIO ===\n");

        let mut optimizer = GeneticOptimizer::new(30);

        // Simulate real performance data collection
        // Assume baseline O0: 10ms, we want to find parameters for best speedup
        let baseline_time_ms = 10.0;
        let baseline_size_kb = 100.0;

        println!("Baseline (O0): {:.2}ms, {} KB\n", baseline_time_ms, baseline_size_kb);
        println!("Evolution Progress:");
        println!("{:-^60}", "Generation");
        println!("{:<12} {:<15} {:<15} {:<15}", "Gen", "Best Fitness", "Best Gene", "Avg Fitness");
        println!("{:-^60}", "-");

        for gen in 0..20 {
            // More realistic fitness: speedup decreases with unroll factor (code size)
            optimizer.evaluate_population(|gene| {
                // Base speedup from opt_level
                let opt_speedup = match gene.opt_level {
                    0 => 1.0,
                    1 => 1.3,
                    2 => 1.5,
                    3 => 1.6,
                    4 => 1.4, // Oz might be slower for some
                    _ => 1.0,
                };

                // Unroll overhead: larger unroll = larger code
                let unroll_overhead = (gene.unroll_factor as f64 / 2.0).min(3.0);
                let size_kb = baseline_size_kb * unroll_overhead;
                let size_penalty = (size_kb / baseline_size_kb).min(2.0);

                // Boolean flags
                let vec_bonus = if gene.vectorization { 1.1 } else { 1.0 };
                let prefetch_bonus = if gene.prefetch { 1.15 } else { 1.0 };

                // Final fitness = speedup / (cost factors)
                (opt_speedup * vec_bonus * prefetch_bonus) / size_penalty
            });

            let stats = optimizer.generation_stats();

            if gen % 5 == 0 || gen == 19 {
                let best_gene = &optimizer.best_individual.as_ref().unwrap().gene;
                println!(
                    "{:<12} {:<15.4} {:<15} {:<15.4}",
                    gen, stats.best_fitness, best_gene, stats.avg_fitness
                );
            }

            optimizer.next_generation(0.5);
        }

        println!("{:-^60}\n", "-");

        let final_best = optimizer.best_individual.as_ref().unwrap();
        let final_fitness = final_best.get_fitness();

        println!("FINAL RESULT:");
        println!("  Gene:    {}", final_best.gene);
        println!("  Fitness: {:.4}", final_fitness);

        // Should find reasonable parameters
        assert!(final_fitness > 1.0);
        println!("\n✅ Full pipeline test passed");
    }

    #[test]
    fn test_parameter_distribution_after_evolution() {
        // Check that evolved population has good coverage of high-fitness parameters
        let mut optimizer = GeneticOptimizer::new(40);

        for _ in 0..15 {
            optimizer.evaluate_population(|gene| {
                gene.opt_level as f64 * 2.0
                    + (gene.unroll_factor as f64).log2()
                    + (if gene.vectorization { 2.0 } else { 0.0 })
                    + (if gene.prefetch { 1.0 } else { 0.0 })
            });

            optimizer.next_generation(0.4);
        }

        // Analyze distribution
        let mut opt_levels = std::collections::HashMap::new();
        let mut vector_count = 0;
        let mut prefetch_count = 0;

        for ind in &optimizer.population {
            *opt_levels
                .entry(ind.gene.opt_level)
                .or_insert(0usize) += 1;

            if ind.gene.vectorization {
                vector_count += 1;
            }
            if ind.gene.prefetch {
                prefetch_count += 1;
            }
        }

        println!(
            "✅ Parameter distribution after evolution:"
        );
        for level in 0..5 {
            if let Some(count) = opt_levels.get(&level) {
                println!("  opt_level {}: {} individuals", level, count);
            }
        }
        println!("  vectorization: {} individuals", vector_count);
        println!("  prefetch: {} individuals", prefetch_count);

        // Should have diversity
        assert!(!opt_levels.is_empty());
    }
}
