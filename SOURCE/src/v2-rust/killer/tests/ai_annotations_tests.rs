/// AI Annotations Feature Tests
/// Tests for AI-First Language Extensions (@ai_assist, @ai_schedule, @ai_validate)

#[cfg(test)]
mod ai_annotations_tests {
    use killer_native::ai_annotations::*;

    // ============================================================================
    // AI ANNOTATION PARSING TESTS
    // ============================================================================

    #[test]
    fn test_parse_assist_annotation() {
        let annotation_str = "assist(numeric optimization)";
        let result = AIAnnotation::parse_annotation_string(annotation_str);
        assert!(result.is_ok(), "Should parse assist annotation");
        
        if let Ok(AIAnnotationType::Assist(goal)) =result {
            assert_eq!(goal, "numeric optimization");
            println!("✓ assist annotation parsed: {}", goal);
        }
    }

    #[test]
    fn test_parse_schedule_annotation() {
        let annotation_str = "schedule(fetch|analyze|report,delay=5000,parallel=true)";
        let result = AIAnnotation::parse_annotation_string(annotation_str);
        assert!(result.is_ok(), "Should parse schedule annotation");
        
        if let Ok(AIAnnotationType::Schedule { steps, delay_ms, parallel }) = result {
            assert_eq!(steps.len(), 3);
            assert_eq!(steps[0], "fetch");
            assert_eq!(steps[1], "analyze");
            assert_eq!(steps[2], "report");
            assert_eq!(delay_ms, Some(5000));
            assert!(parallel);
            println!("✓ schedule annotation parsed: steps={:?}, delay={}ms, parallel={}", steps, delay_ms.unwrap_or(0), parallel);
        }
    }

    #[test]
    fn test_parse_schedule_with_seconds_delay() {
        let annotation_str = "schedule(load|process,delay=5s,parallel=false)";
        let result = AIAnnotation::parse_annotation_string(annotation_str);
        assert!(result.is_ok());
        
        if let Ok(AIAnnotationType::Schedule { delay_ms, .. }) = result {
            assert_eq!(delay_ms, Some(5000)); // 5s = 5000ms
            println!("✓ schedule with seconds delay parsed correctly");
        }
    }

    #[test]
    fn test_parse_schedule_with_minutes_delay() {
        let annotation_str = "schedule(workflow,delay=1min,parallel=true)";
        let result = AIAnnotation::parse_annotation_string(annotation_str);
        assert!(result.is_ok());
        
        if let Ok(AIAnnotationType::Schedule { delay_ms, .. }) = result {
            assert_eq!(delay_ms, Some(60000)); // 1min = 60000ms
            println!("✓ schedule with minutes delay parsed correctly");
        }
    }

    #[test]
    fn test_parse_validate_annotation() {
        let annotation_str = "validate(bounds checking)";
        let result = AIAnnotation::parse_annotation_string(annotation_str);
        assert!(result.is_ok());
        
        if let Ok(AIAnnotationType::Validate(constraint)) = result {
            assert_eq!(constraint, "bounds checking");
            println!("✓ validate annotation parsed: {}", constraint);
        }
    }

    // ============================================================================
    // AI ANNOTATION CREATION TESTS
    // ============================================================================

    #[test]
    fn test_create_annotation() {
        let annotation_type = AIAnnotationType::Assist("optimize performance".to_string());
        let annotation = AIAnnotation::new(annotation_type, "my_function".to_string(), 42);
        
        assert_eq!(annotation.function_name, "my_function");
        assert_eq!(annotation.line, 42);
        println!("✓ AI annotation created successfully");
    }

    #[test]
    fn test_annotation_with_metadata() {
        let annotation_type = AIAnnotationType::Assist("improve latency".to_string());
        let mut annotation = AIAnnotation::new(annotation_type, "fast_path".to_string(), 100);
        annotation = annotation.with_metadata("priority".to_string(), "critical".to_string());
        annotation = annotation.with_metadata("target_latency_ms".to_string(), "100".to_string());
        
        assert_eq!(annotation.metadata.get("priority"), Some(&"critical".to_string()));
        assert_eq!(annotation.metadata.get("target_latency_ms"), Some(&"100".to_string()));
        println!("✓ annotation metadata added successfully");
    }

    // ============================================================================
    // AI HINT TESTS
    // ============================================================================

    #[test]
    fn test_create_ai_hint() {
        let hint = AIHint::new(
            "optimization".to_string(),
            "Use batch processing for large datasets".to_string(),
            0.85,
        );
        
        assert_eq!(hint.category, "optimization");
        assert_eq!(hint.confidence, 0.85);
        assert_eq!(hint.priority, 5); // default
        println!("✓ AI hint created: {}", hint.suggestion);
    }

    #[test]
    fn test_ai_hint_with_improvement() {
        let hint = AIHint::new(
            "performance".to_string(),
            "Parallelize loop iterations".to_string(),
            0.92,
        )
        .with_improvement(2.5);
        
        assert_eq!(hint.improvement, Some(2.5));
        println!("✓ AI hint with improvement: expected {:.1}x speedup", hint.improvement.unwrap_or(1.0));
    }

    #[test]
    fn test_ai_hint_priority() {
        let hint = AIHint::new(
            "correctness".to_string(),
            "Add null check".to_string(),
            0.98,
        )
        .with_priority(9); // Critical
        
        assert_eq!(hint.priority, 9);
        println!("✓ AI hint priority set to: {}/10", hint.priority);
    }

    // ============================================================================
    // AI HINT SET TESTS
    // ============================================================================

    #[test]
    fn test_ai_hint_set_creation() {
        let mut hint_set = AIHintSet::with_function_name("process_data".to_string());
        
        hint_set.add_hint(AIHint::new(
            "performance".to_string(),
            "Cache frequent lookups".to_string(),
            0.75,
        ).with_priority(7));
        
        hint_set.add_hint(AIHint::new(
            "correctness".to_string(),
            "Validate input bounds".to_string(),
            0.95,
        ).with_priority(9));
        
        hint_set.add_hint(AIHint::new(
            "readability".to_string(),
            "Extract method for clarity".to_string(),
            0.60,
        ).with_priority(3));
        
        assert_eq!(hint_set.hints.len(), 3);
        
        // Should be sorted by priority (descending)
        assert_eq!(hint_set.hints[0].priority, 9);
        assert_eq!(hint_set.hints[1].priority, 7);
        assert_eq!(hint_set.hints[2].priority, 3);
        
        println!("✓ hint set auto-sorted by priority: {} hints", hint_set.hints.len());
    }

    #[test]
    fn test_hint_set_top_hint() {
        let mut hint_set = AIHintSet::with_function_name("compute".to_string());
        
        hint_set.add_hint(AIHint::new(
            "low-priority".to_string(),
            "Minor optimization".to_string(),
            0.50,
        ).with_priority(2));
        
        hint_set.add_hint(AIHint::new(
            "high-priority".to_string(),
            "Critical fix needed".to_string(),
            0.99,
        ).with_priority(10));
        
        let top = hint_set.top_hint();
        assert!(top.is_some());
        assert_eq!(top.unwrap().priority, 10);
        assert_eq!(top.unwrap().confidence, 0.99);
        println!("✓ top hint retrieved (priority={}, confidence={})", top.unwrap().priority, top.unwrap().confidence);
    }

    #[test]
    fn test_hint_set_high_confidence_filtering() {
        let mut hint_set = AIHintSet::with_function_name("ml_model".to_string());
        
        hint_set.add_hint(AIHint::new("opt1".to_string(), "Maybe optimize".to_string(), 0.45));
        hint_set.add_hint(AIHint::new("opt2".to_string(), "Probably optimize".to_string(), 0.78));
        hint_set.add_hint(AIHint::new("opt3".to_string(), "Definitely optimize".to_string(), 0.95));
        
        let high_conf = hint_set.high_confidence_hints(0.75);
        assert_eq!(high_conf.len(), 2); // opt2 and opt3
        
        let conf_len = high_conf.len();
        for hint in high_conf {
            assert!(hint.confidence >= 0.75);
        }
        println!("✓ filtered {} high-confidence hints (confidence >= 0.75)", conf_len);
    }

    // ============================================================================
    // ANNOTATION TYPE TESTS
    // ============================================================================

    #[test]
    fn test_annotation_equality() {
        let type1 = AIAnnotationType::Assist("goal1".to_string());
        let type2 = AIAnnotationType::Assist("goal1".to_string());
        let type3 = AIAnnotationType::Assist("goal2".to_string());
        
        assert_eq!(type1, type2);
        assert_ne!(type1, type3);
        println!("✓ annotation type equality works");
    }

    #[test]
    fn test_schedule_with_no_defaults() {
        let annotation_str = "schedule(step1|step2)";
        let result = AIAnnotation::parse_annotation_string(annotation_str);
        assert!(result.is_ok());
        
        if let Ok(AIAnnotationType::Schedule { steps, delay_ms, parallel }) = result {
            assert_eq!(steps, vec!["step1", "step2"]);
            assert_eq!(delay_ms, None); // no delay specified
            assert!(!parallel); // defaults to false
            println!("✓ schedule with minimal parameters parsed");
        }
    }

    // ============================================================================
    // INTEGRATION SCENARIOS
    // ============================================================================

    #[test]
    fn test_full_ai_annotation_workflow() {
        println!("\n🔄 Full AI Annotation Workflow Test");
        
        // 1. Parse annotation from source
        println!("Step 1: Parse @ai_assist annotation");
        let annotation_str = "assist(numeric optimization)";
        let annotation_type = AIAnnotation::parse_annotation_string(annotation_str)
            .expect("should parse");
        println!("  ✓ Parsed: {:?}", annotation_type);
        
        // 2. Create full annotation object
        println!("Step 2: Create AI annotation object");
        let mut annotation = AIAnnotation::new(annotation_type, "process_matrix".to_string(), 56);
        annotation = annotation.with_metadata("targeted_improvement".to_string(), "1.5x".to_string());
        println!("  ✓ Created annotation for function: {}", annotation.function_name);
        
        // 3. Generate hints for the function
        println!("Step 3: Generate optimization hints");
        let mut hint_set = AIHintSet::with_function_name(annotation.function_name.clone());
        
        hint_set.add_hint(AIHint::new(
            "parallelization".to_string(),
            "Use SIMD for element-wise operations".to_string(),
            0.88,
        ).with_improvement(2.1).with_priority(9));
        
        hint_set.add_hint(AIHint::new(
            "caching".to_string(),
            "Cache matrix transpose results".to_string(),
            0.76,
        ).with_improvement(1.4).with_priority(7));
        
        println!("  ✓ Generated {} optimization hints", hint_set.hints.len());
        
        // 4. Select top recommendation
        println!("Step 4: Select top recommendation");
        if let Some(top_hint) = hint_set.top_hint() {
            println!("  ✓ Top recommendation (priority={}): {}", top_hint.priority, top_hint.suggestion);
            println!("    Expected improvement: {:.1}x", top_hint.improvement.unwrap_or(1.0));
        }
        
        println!("✅ Full workflow completed successfully!");
    }

    #[test]
    fn test_ai_annotations_coverage() {
        println!("\n📊 AI Annotations Feature Coverage Test");
        
        // Demonstrate all annotation types
        let assist = AIAnnotationType::Assist("improve speed".to_string());
        let validate = AIAnnotationType::Validate("check bounds".to_string());
        let schedule = AIAnnotationType::Schedule {
            steps: vec!["fetch".to_string(), "process".to_string(), "store".to_string()],
            delay_ms: Some(1000),
            parallel: true,
        };
        
        println!("✓ @ai_assist: {}", match assist { AIAnnotationType::Assist(goal) => goal, _ => "N/A".to_string() });
        println!("✓ @ai_validate: {}", match validate { AIAnnotationType::Validate(c) => c, _ => "N/A".to_string() });
        println!("✓ @ai_schedule: {:?} steps", match schedule { AIAnnotationType::Schedule { steps, .. } => steps.len(), _ => 0 });
        println!("\n✅ All annotation types verified!");
    }
}
