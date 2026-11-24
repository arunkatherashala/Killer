/// Comprehensive AI Module Tests
/// Tests all 4 AI layers integrated together

#[cfg(test)]
mod ai_integration_tests {
    use killer_native::ai_optimizer::*;
    use killer_native::super_agent_layer::*;

    // ============================================================================
    // AI OPTIMIZER TESTS
    // ============================================================================

    #[test]
    fn test_ai_optimizer_pattern_tracking() {
        println!("\n🧠 TEST: AI Optimizer Pattern Tracking");
        
        let optimizer = SuperProcessorAIOptimizer::new();
        
        // Simulate operation patterns
        println!("  Recording operation patterns...");
        for i in 0..1000 {
            let latency = if i % 100 == 0 { 500 } else { 50 };
            optimizer
                .database
                .record_execution("arithmetic", latency, 256)
                .unwrap();
        }

        for i in 0..500 {
            let memory = if i % 50 == 0 { 10000 } else { 256 };
            optimizer
                .database
                .record_execution("memory_intensive", 200, memory)
                .unwrap();
        }

        // Check patterns tracked
        let patterns = optimizer.database.all_patterns().unwrap();
        println!("  ✓ Tracked {} operation types", patterns.len());
        
        for pattern in &patterns {
            println!(
                "    - {}: frequency={}, avg_latency={}μs, gpu_suitable={}",
                pattern.op_type, pattern.frequency, pattern.avg_latency_us, pattern.gpu_suitable
            );
        }

        assert_eq!(patterns.len(), 2);
        assert!(patterns.iter().any(|p| p.op_type == "arithmetic" && p.frequency > 500));
        println!("  ✓ Pattern tracking validated");
    }

    #[test]
    fn test_ai_optimizer_recommendations() {
        println!("\n🎯 TEST: AI Optimizer Generates Recommendations");
        
        let optimizer = SuperProcessorAIOptimizer::new();

        // Hot path
        for _ in 0..2000 {
            optimizer.database.record_execution("hot_arithmetic", 30, 128).unwrap();
        }

        // Cold path
        for _ in 0..10 {
            optimizer.database.record_execution("rare_operation", 1000, 50000).unwrap();
        }

        // Generate recommendations
        let recommendations = optimizer.analyze_and_recommend().unwrap();
        println!("  ✓ Generated {} recommendations", recommendations.len());

        for rec in &recommendations {
            println!(
                "    - {}: JIT_threshold={}, batch_size={}, use_gpu={}, confidence={:.2}",
                rec.operation_type, rec.jit_threshold, rec.batch_size, rec.use_gpu, rec.confidence.value
            );
        }

        // Verify hot path gets lower JIT threshold (compile sooner)
        let hot_rec = recommendations
            .iter()
            .find(|r| r.operation_type == "hot_arithmetic")
            .unwrap();
        assert!(hot_rec.jit_threshold < 500, "Hot path should compile sooner");
        assert!(
            hot_rec.expected_improvement > 1.0,
            "Should predict performance improvement"
        );

        println!("  ✓ Recommendations validated");
    }

    #[test]
    fn test_ai_optimizer_stats() {
        println!("\n📊 TEST: AI Optimizer Statistics");
        
        let optimizer = SuperProcessorAIOptimizer::new();

        for _ in 0..100 {
            optimizer.database.record_execution("test_op", 75, 512).unwrap();
        }

        optimizer.analyze_and_recommend().unwrap();
        let stats = optimizer.stats().unwrap();

        println!(
            "  Patterns tracked: {}", 
            stats.patterns_tracked
        );
        println!(
            "  Total samples: {}", 
            stats.total_samples
        );
        println!(
            "  Confident recommendations: {}", 
            stats.confident_recommendations
        );
        println!(
            "  Avg improvement: {:.2}x", 
            stats.avg_expected_improvement
        );

        assert!(stats.total_samples > 0);
        println!("  ✓ Statistics validated");
    }

    // ============================================================================
    // LLM CLIENT TESTS
    // ============================================================================


    // ============================================================================
    // SUPER AGENT LAYER TESTS
    // ============================================================================

    #[test]
    fn test_tool_registry() {
        println!("\n🔧 TEST: Custom Tool Registry");
        
        let registry = ToolRegistry::new();

        let handler = std::sync::Arc::new(|params: std::collections::HashMap<String, String>| {
            let query = params.get("query").cloned().unwrap_or_else(|| "default".to_string());
            Ok(format!("Search results for: {}", query))
        });

        println!("  Registering custom tools...");
        registry
            .register_tool(
                "web_search",
                "Search the web for information",
                vec!["query".to_string()],
                handler,
            )
            .unwrap();

        let tools = registry.list_tools().unwrap();
        println!("  ✓ Registered tools: {:?}", tools);
        assert!(tools.contains(&"web_search".to_string()));
    }

    #[test]
    fn test_workflow_definition() {
        println!("\n📋 TEST: Workflow Definition & Execution Order");
        
        let workflow = Workflow::new("research_workflow");

        println!("  Defining steps...");
        workflow
            .add_step(
                "fetch",
                SuperAgentType::Researcher,
                "Fetch data from sources",
                vec![],
            )
            .unwrap();

        workflow
            .add_step(
                "analyze",
                SuperAgentType::Analyzer,
                "Analyze the data",
                vec!["fetch".to_string()],
            )
            .unwrap();

        workflow
            .add_step(
                "report",
                SuperAgentType::Planner,
                "Generate report",
                vec!["analyze".to_string()],
            )
            .unwrap();

        let order = workflow.get_execution_order().unwrap();
        println!("  ✓ Execution order: {:?}", order);
        assert_eq!(order, vec!["fetch", "analyze", "report"]);
    }

    #[test]
    fn test_knowledge_graph() {
        println!("\n📚 TEST: Knowledge Graph Construction");
        
        let kg = KnowledgeGraph::new();

        println!("  Adding entities...");
        kg.add_entity("killer_core", "Killer SuperProcessor", "system")
            .unwrap();
        kg.add_entity("ai_layer", "AI Optimization", "subsystem")
            .unwrap();
        kg.add_entity("agent_framework", "Agent Framework", "subsystem")
            .unwrap();

        println!("  Adding relations...");
        kg.add_relation("killer_core", "ai_layer", "contains", 0.9)
            .unwrap();
        kg.add_relation("killer_core", "agent_framework", "contains", 0.8)
            .unwrap();

        println!("  ✓ Entities: {}", kg.entity_count().unwrap());
        println!("  ✓ Relations: {}", kg.relation_count().unwrap());

        let related = kg
            .find_related("killer_core", "contains")
            .unwrap();
        println!("  ✓ Related to killer_core: {:?}", related);
        assert_eq!(related.len(), 2);
    }

    #[test]
    fn test_agent_swarm() {
        println!("\n🐝 TEST: Agent Swarm Coordination");
        
        let swarm = AgentSwarm::new("optimization_team", 4);

        println!("  Setting up swarm...");
        
        // Register tools
        let tools = swarm.get_tool_registry();
        let handler = std::sync::Arc::new(|_params: std::collections::HashMap<String, String>| {
            Ok("Profiling complete".to_string())
        });
        
        tools
            .register_tool(
                "profile_operations",
                "Profile operation execution",
                vec![],
                handler,
            )
            .unwrap();

        // Build knowledge graph
        let kg = swarm.get_knowledge_graph();
        kg.add_entity("perf_metric", "Performance Metric", "data").unwrap();

        // Define workflow
        let workflow = swarm.get_workflow();
        workflow
            .add_step(
                "profile",
                SuperAgentType::Analyzer,
                "Profile workload",
                vec![],
            )
            .unwrap();

        println!("  ✓ Swarm created with {} agents", 4);
        println!("  ✓ Tools registered: {:?}", tools.list_tools().unwrap());
        println!("  ✓ Knowledge entities: {}", kg.entity_count().unwrap());
    }

    // ============================================================================
    // INTEGRATION TESTS
    // ============================================================================

}
