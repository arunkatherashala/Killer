/// Comprehensive AI Module Tests
/// Tests all 4 AI layers integrated together

#[cfg(test)]
mod ai_integration_tests {
    use killer_native::ai_optimizer::*;
    use killer_native::llm_client::*;
    use killer_native::agent_framework::*;
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

    #[test]
    fn test_llm_message_creation() {
        println!("\n💬 TEST: LLM Message Creation");
        
        let msg = LLMMessage {
            role: MessageRole::User,
            content: "What is Killer?".to_string(),
            tool_use: None,
        };

        assert_eq!(msg.role, MessageRole::User);
        assert!(msg.content.contains("Killer"));
        println!("  ✓ User message created: {}", msg.content);

        let system_msg = LLMMessage {
            role: MessageRole::System,
            content: "You are a helpful assistant".to_string(),
            tool_use: None,
        };

        assert_eq!(system_msg.role, MessageRole::System);
        println!("  ✓ System message created");
    }

    #[test]
    fn test_llm_request_building() {
        println!("\n📨 TEST: LLM Request Building");
        
        let request = LLMRequest {
            messages: vec![
                LLMMessage {
                    role: MessageRole::System,
                    content: "You are helpful".to_string(),
                    tool_use: None,
                },
                LLMMessage {
                    role: MessageRole::User,
                    content: "Explain Killer".to_string(),
                    tool_use: None,
                },
            ],
            model: "gpt-4".to_string(),
            temperature: 0.7,
            max_tokens: 1024,
            tools: vec![],
            stream: false,
        };

        assert_eq!(request.messages.len(), 2);
        assert_eq!(request.model, "gpt-4");
        println!("  ✓ Request with {} messages", request.messages.len());
        println!("  ✓ Model: {}", request.model);
        println!("  ✓ Temperature: {}", request.temperature);
    }

    #[test]
    fn test_llm_response_handling() {
        println!("\n📤 TEST: LLM Response Handling");
        
        let response = LLMResponse {
            content: "Killer is a high-performance language".to_string(),
            model: "gpt-4".to_string(),
            tokens_used: 150,
            finish_reason: "stop".to_string(),
            tool_calls: vec![],
        };

        assert_eq!(response.tokens_used, 150);
        assert_eq!(response.finish_reason, "stop");
        println!("  ✓ Response received: {} tokens", response.tokens_used);
        println!("  ✓ Content: {}", response.content);
    }

    #[test]
    fn test_llm_cost_tracking() {
        println!("\n💰 TEST: LLM Cost Tracking");
        
        let config = LLMConfig {
            provider: LLMProvider::OpenAI,
            api_key: "test".to_string(),
            base_url: None,
            timeout_seconds: 30,
        };

        let client = LLMClient::new(config);
        let response = LLMResponse {
            content: "Test".to_string(),
            model: "gpt-4".to_string(),
            tokens_used: 1000,
            finish_reason: "stop".to_string(),
            tool_calls: vec![],
        };

        let mut tracker = CostTracker {
            openai_cost: 0.0,
            claude_cost: 0.0,
            total_requests: 0,
            cached_hits: 0,
        };

        client.update_cost_tracking(&response, &mut tracker);
        println!("  ✓ OpenAI cost tracked: ${:.6}", tracker.openai_cost);
        assert!(tracker.openai_cost > 0.0);
    }

    // ============================================================================
    // AGENT FRAMEWORK TESTS
    // ============================================================================

    #[test]
    fn test_agent_creation_and_init() {
        println!("\n🤖 TEST: Agent Creation and Initialization");
        
        let config = AgentConfig::new("TestBot", "analyst");
        let agent = Agent::new(config.clone());

        agent.initialize().unwrap();
        let status = agent.status().unwrap();

        println!("  ✓ Agent created: {}", status.name);
        println!("  ✓ Role: {}", config.role);
        println!("  ✓ State: {:?}", status.state);
        assert_eq!(status.state, AgentState::Idle);
    }

    #[test]
    fn test_agent_memory() {
        println!("\n💾 TEST: Agent Memory System");
        
        let config = AgentConfig::new("MemoryBot", "learner");
        let agent = Agent::new(config);
        agent.initialize().unwrap();

        println!("  Recording important facts...");
        agent.remember("Killer is 6.89x faster", 0.95).unwrap();
        agent.remember("Cluster scales linearly", 0.90).unwrap();
        agent.remember("AI optimizes automatically", 0.85).unwrap();

        let status = agent.status().unwrap();
        println!("  ✓ Stored {} memories", status.memories);
        assert_eq!(status.memories, 3);
    }

    #[test]
    fn test_agent_reasoning() {
        println!("\n🧠 TEST: Agent Reasoning Chain");
        
        let config = AgentConfig::new("ReasoningBot", "strategist");
        let agent = Agent::new(config);
        agent.initialize().unwrap();

        println!("  Generating thoughts...");
        let thought1 = agent.reason("What is our goal?").unwrap();
        let thought2 = agent.reason("What resources do we have?").unwrap();
        let thought3 = agent.reason("What's the best approach?").unwrap();

        println!("  ✓ {}", thought1);
        println!("  ✓ {}", thought2);
        println!("  ✓ {}", thought3);
    }

    #[test]
    fn test_agent_action_and_observation() {
        println!("\n⚙️ TEST: Agent Actions & Observations");
        
        let config = AgentConfig::new("ActionBot", "executor");
        let agent = Agent::new(config);
        agent.initialize().unwrap();

        let mut params = std::collections::HashMap::new();
        params.insert("target".to_string(), "SuperProcessor".to_string());

        let action = Action {
            tool_name: "optimize".to_string(),
            parameters: params,
            reasoning: "Improve performance".to_string(),
        };

        println!("  Taking action: {}", action.tool_name);
        agent.act(action).unwrap();

        let observation = Observation {
            action_id: "act_1".to_string(),
            result: "Performance improved by 15%".to_string(),
            success: true,
        };

        println!("  Observing result: {}", observation.result);
        agent.observe(observation).unwrap();

        let status = agent.status().unwrap();
        println!("  ✓ Actions taken: {}", status.actions_taken);
        assert_eq!(status.actions_taken, 1);
    }

    #[test]
    fn test_agent_pool() {
        println!("\n👥 TEST: Agent Pool Management");
        
        let pool = AgentPool::new();

        for i in 0..3 {
            let config = AgentConfig::new(&format!("Agent{}", i), "worker");
            let agent = Agent::new(config);
            agent.initialize().unwrap();
            pool.add_agent(agent).unwrap();
        }

        let count = pool.count().unwrap();
        println!("  ✓ Pool contains {} agents", count);
        assert_eq!(count, 3);
    }

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

    #[test]
    fn test_full_ai_stack_integration() {
        println!("\n🚀 TEST: Full AI Stack Integration");
        
        println!("\n  ✓ Phase 1: AI Optimizer");
        let optimizer = SuperProcessorAIOptimizer::new();
        for _ in 0..500 {
            optimizer
                .database
                .record_execution("compute", 75, 512)
                .unwrap();
        }
        let recs = optimizer.analyze_and_recommend().unwrap();
        println!("    - Generated {} recommendations", recs.len());

        println!("\n  ✓ Phase 2: LLM Client");
        let request = LLMRequest {
            messages: vec![LLMMessage {
                role: MessageRole::User,
                content: "Optimize process".to_string(),
                tool_use: None,
            }],
            model: "gpt-4".to_string(),
            temperature: 0.7,
            max_tokens: 512,
            tools: vec![],
            stream: false,
        };
        println!("    - LLM request ready ({})", request.messages.len());

        println!("\n  ✓ Phase 3: Agent Framework");
        let config = AgentConfig::new("OptimizationAgent", "optimizer");
        let agent = Agent::new(config);
        agent.initialize().unwrap();
        agent.remember("AI optimizer active", 0.9).unwrap();
        agent.remember("Processing workload", 0.85).unwrap();
        let status = agent.status().unwrap();
        println!("    - Agent ready with {} memories", status.memories);

        println!("\n  ✓ Phase 4: SuperAgent Layer");
        let swarm = AgentSwarm::new("optimization_swarm", 3);
        let _tools = swarm.get_tool_registry();
        println!("    - Swarm created with tool registry");

        println!("\n  ✅ FULL STACK INTEGRATION SUCCESSFUL!");
        println!("    - AI Optimizer: Ready");
        println!("    - LLM Client: Ready");
        println!("    - Agent Framework: Ready");
        println!("    - SuperAgent Layer: Ready");
    }

    #[test]
    fn test_ai_flow_scenario() {
        println!("\n🎯 TEST: Real-World AI Scenario");
        println!("\n  Scenario: Optimize SuperProcessor for financial workload\n");

        // Step 1: Optimizer analyzes patterns
        println!("  Step 1️⃣: AI Optimizer analyzes patterns");
        let optimizer = SuperProcessorAIOptimizer::new();
        for i in 0..500 {
            let latency = if i % 50 == 0 { 200 } else { 45 };
            let memory = if i % 100 == 0 { 12000 } else { 256 };
            optimizer
                .database
                .record_execution("price_calc", latency, memory)
                .unwrap();
        }
        let recs = optimizer.analyze_and_recommend().unwrap();
        println!("    Result: {} recommendations generated", recs.len());

        // Step 2: Agent reasons about optimization
        println!("\n  Step 2️⃣: Agent reasons about strategy");
        let agent = Agent::new(AgentConfig::new("FinanceOptimizer", "optimizer"));
        agent.initialize().unwrap();
        agent
            .reason("How to optimize financial calculations?")
            .unwrap();
        agent.remember("Identified hot paths in calculations", 0.9).unwrap();

        // Step 3: Execute action
        println!("\n  Step 3️⃣: Execute optimization");
        let mut params = std::collections::HashMap::new();
        params.insert("threshold".to_string(), "250".to_string());
        let action = Action {
            tool_name: "apply_jit_optimization".to_string(),
            parameters: params,
            reasoning: "Lower JIT threshold for hot paths".to_string(),
        };
        agent.act(action).unwrap();

        // Step 4: Observe results
        println!("\n  Step 4️⃣: Observe results");
        let observation = Observation {
            action_id: "opt_1".to_string(),
            result: "Performance improved from 1.9M to 2.2M ops/sec".to_string(),
            success: true,
        };
        agent.observe(observation).unwrap();

        // Step 5: Knowledge graph tracks learnings
        println!("\n  Step 5️⃣: Update knowledge graph");
        let kg = KnowledgeGraph::new();
        kg.add_entity("optimization_event", "Applied JIT tuning", "action")
            .unwrap();
        kg.add_entity("performance_gain", "300K ops/sec", "metric")
            .unwrap();
        kg.add_relation("optimization_event", "performance_gain", "resulted_in", 0.95)
            .unwrap();

        let status = agent.status().unwrap();
        println!("\n  ✅ Scenario Complete!");
        println!("    - Agent iterations: {}", status.iterations);
        println!("    - Actions taken: {}", status.actions_taken);
        println!("    - Memories stored: {}", status.memories);
    }
}
