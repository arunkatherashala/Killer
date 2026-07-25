# PHASE 27 - DISTRIBUTED SYSTEMS & MESSAGING
## Master Plan & Detailed Specifications

**Date:** March 18, 2026  
**Target Delivery:** Single session  
**Scope:** 5 modules, 250+ functions, 3,000+ lines, 50+ tests  
**Integration:** All modules registered in lib.rs

---

## Phase 27 Overview

Add infrastructure capabilities for deploying Killer services across distributed clusters. Covers service discovery, load balancing, circuit breaker patterns, message queues, and distributed tracing - essential for production microservices.

**Why Phase 27?**
- Phases 20-26 built complete single-service stack
- Phase 27 enables multi-service deployments
- Circuit breakers & resilience patterns prevent cascading failures
- Message queues enable async communication
- Distributed tracing provides observability across services

---

## Module Specifications

### Phase 27.1: Service Discovery (450-500 lines, 50 functions)

**Purpose:** Dynamic service registration and lookup (Consul, DNS, Kubernetes patterns)

**Categories:**

#### Service Registry (12 functions)
- `create_service_registry()` - Consul/etcd-like registry
- `register_service()` - Add service instance
- `deregister_service()` - Remove service instance
- `find_service_by_name()` - Lookup all instances
- `get_service_instance()` - Get specific instance
- `list_all_services()` - All registered services
- `get_service_count()` - How many instances
- `update_service_status()` - Healthy/unhealthy
- `mark_service_healthy()` - Set status
- `mark_service_unhealthy()` - Unregister if failing
- `check_service_health()` - Query health
- `get_service_endpoints()` - All instances' addresses

#### Service Registration (10 functions)
- `register_http_service()` - HTTP service
- `register_grpc_service()` - gRPC service
- `register_with_ttl()` - TTL-based registration
- `renew_service_registration()` - Heartbeat/keep-alive
- `get_registration_id()` - UUID for instance
- `update_service_metadata()` - Add tags/labels
- `get_service_metadata()` - Query tags
- `register_with_health_check()` - Automated checking
- `deregister_all_instances()` - Bulk unregister
- `get_service_registration_time()` - When registered

#### DNS Service Discovery (10 functions)
- `configure_dns_discovery()` - DNS SRV records
- `query_service_dns()` - Lookup via DNS
- `create_dns_record()` - service.consul format
- `parse_service_dns()` - Extract host:port
- `get_dns_ttl()` - Cache duration
- `create_srv_record()` - SRV record format
- `parse_srv_record()` - Extract priority/weight
- `weighted_round_robin_dns()` - Load balance via DNS
- `get_all_dns_instances()` - All A records
- `check_dns_resolution()` - Verify DNS works

#### Health Checking (10 functions)
- `create_health_check()` - HTTP/TCP probe
- `execute_health_check()` - Run health check
- `get_health_check_result()` - Pass/fail status
- `get_health_check_latency()` - Response time
- `set_health_check_interval()` - How often to check
- `set_health_check_timeout()` - Max wait time
- `get_health_check_history()` - Status over time
- `mark_unhealthy_after_failures()` - Failure threshold
- `auto_recover_service()` - Retry failed services
- `get_healthy_instances()` - Filter healthy only

#### Service Watch (8 functions)
- `watch_service_changes()` - Subscribe to changes
- `on_service_registered()` - Callback on register
- `on_service_deregistered()` - Callback on deregister
- `on_service_status_changed()` - Callback on status change
- `unwatch_service()` - Stop watching
- `get_watch_events()` - Event history
- `broadcast_service_update()` - Pub/sub notify
- `get_service_change_log()` - Audit trail

---

### Phase 27.2: Load Balancing (450-500 lines, 50 functions)

**Purpose:** Distribute traffic across service instances

**Categories:**

#### Load Balancing Algorithms (15 functions)
- `round_robin_load_balancer()` - Simple round-robin
- `least_connections_balancer()` - Route to least-busy
- `weighted_round_robin()` - Weight by capacity
- `random_balancer()` - Random selection
- `ip_hash_balancer()` - Client IP affinity
- `least_response_time()` - Route by latency
- `consistent_hash_balancer()` - Minimal redistribution
- `resource_based_balancer()` - CPU/memory aware
- `sticky_sessions()` - Session affinity
- `maglev_hash()` - Google's consistent hash
- `rendezvous_hash()` - HRW algorithm
- `power_of_two_choices()` - Sample 2 random
- `locality_aware_balancer()` - Geographic preference
- `priority_balancer()` - Primary > secondary
- `custom_balancer()` - User-defined function

#### Load Balancer State (12 functions)
- `create_load_balancer()` - Initialize LB
- `add_backend_instance()` - Add target
- `remove_backend_instance()` - Remove target
- `get_backend_instances()` - All targets
- `get_current_index()` - RR state
- `update_backend_weight()` - Change capacity
- `get_backend_weight()` - Query capacity
- `get_backend_connection_count()` - Current load
- `reset_load_balancer_state()` - Clear state
- `export_load_balancer_metrics()` - Stats
- `get_load_balancer_health()` - All backends healthy?
- `set_backend_priority()` - Primary/secondary

#### Request Routing (12 functions)
- `select_backend()` - Choose next instance
- `route_request()` - Send to selected backend
- `route_with_timeout()` - Timeout on slow backend
- `route_with_retry()` - Retry on failure
- `route_with_circuit_breaker()` - CB integration
- `get_routing_decision()` - Which backend chosen
- `log_routing_decision()` - Audit trail
- `get_request_destination()` - Final host:port
- `validate_routing_decision()` - Check valid instance
- `handle_routing_failure()` - Fallback strategy
- `get_route_latency()` - Response time
- `aggregate_routing_stats()` - Performance metrics

#### Health-Aware Routing (11 functions)
- `exclude_unhealthy_backends()` - Filter out bad instances
- `get_healthy_backend_count()` - How many working
- `get_unhealthy_backend_count()` - How many broken
- `route_only_to_healthy()` - Safe routing
- `handle_all_backends_down()` - Fail-open strategy
- `gradual_health_restoration()` - Slow recovery
- `get_backend_availability()` - % healthy
- `alert_on_backend_failure()` - Notification
- `auto_degrade_service()` - Graceful degradation
- `get_time_since_backend_failure()` - Time metric
- `predict_backend_recovery()` - ML-based estimate

---

### Phase 27.3: Circuit Breaker (450-500 lines, 50 functions)

**Purpose:** Prevent cascading failures across services

**Categories:**

#### Circuit State Management (12 functions)
- `create_circuit_breaker()` - Initialize CB
- `get_circuit_state()` - Open/Closed/Half-Open
- `set_circuit_state()` - Change state
- `circuit_is_open()` - Is currently open?
- `circuit_is_closed()` - Is currently closed?
- `circuit_is_half_open()` - Is currently testing?
- `transition_to_open()` - Closed → Open
- `transition_to_half_open()` - Open → Half-Open
- `transition_to_closed()` - Half-Open → Closed
- `get_state_change_time()` - When did state change
- `get_time_in_state()` - How long in current state
- `get_state_transition_history()` - State over time

#### Failure Detection (12 functions)
- `record_success()` - Success count increment
- `record_failure()` - Failure count increment
- `get_failure_count()` - Total failures
- `get_success_count()` - Total successes
- `get_failure_rate()` - % failures
- `get_failure_threshold()` - When to open CB
- `set_failure_threshold()` - Configure trigger
- `get_consecutive_failures()` - Streak count
- `reset_failure_counter()` - Clear on success
- `get_time_since_last_failure()` - Last failure time
- `is_failure_rate_exceeded()` - Should we open?
- `calculate_error_rate()` - Real-time %

#### Recovery & Testing (13 functions)
- `get_timeout_duration()` - Open duration
- `set_timeout_duration()` - Configure window
- `attempt_half_open_request()` - Test if recovered
- `record_half_open_attempt()` - Count test
- `get_half_open_attempt_count()` - How many tests
- `get_half_open_success_threshold()` - When to close
- `set_half_open_success_threshold()` - Configure
- `is_recovery_attempt_successful()` - Did test work?
- `exponential_backoff_timeout()` - Increase timeout
- `get_next_retry_time()` - When to retry
- `calculate_retry_delay()` - Backoff algorithm
- `schedule_recovery_check()` - Queue test
- `perform_scheduled_recovery_check()` - Run test

#### Metrics & Monitoring (13 functions)
- `get_circuit_breaker_metrics()` - All stats
- `get_open_time_total()` - Total time open
- `get_request_count()` - Total requests
- `get_rejected_request_count()` - Requests rejected
- `get_slow_request_count()` - Requests > timeout
- `export_circuit_metrics()` - Prometheus format
- `get_circuit_health_score()` - 0-100 health
- `alert_on_circuit_open()` - Notification on open
- `alert_on_excessive_errors()` - Threshold alert
- `log_circuit_state_change()` - Audit trail
- `get_circuit_breaker_name()` - Identifier
- `set_circuit_breaker_name()` - Label
- `get_circuit_breaker_description()` - Metadata

---

### Phase 27.4: Message Queues (450-500 lines, 50 functions)

**Purpose:** Asynchronous communication (RabbitMQ, Kafka patterns)

**Categories:**

#### Queue Management (12 functions)
- `create_message_queue()` - New queue
- `create_topic()` - Pub/sub topic
- `delete_queue()` - Remove queue
- `delete_topic()` - Remove topic
- `get_queue_length()` - Messages pending
- `get_queue_depth()` - Size in bytes
- `purge_queue()` - Clear all messages
- `list_all_queues()` - All queues
- `list_all_topics()` - All topics
- `get_queue_attributes()` - Config
- `set_queue_ttl()` - Message expiry
- `set_queue_max_length()` - Size limit

#### Publishing (12 functions)
- `publish_message()` - Send to queue
- `publish_to_topic()` - Send to subscribers
- `batch_publish()` - Multiple messages
- `publish_with_priority()` - Urgent/normal
- `publish_with_ttl()` - Time-to-live
- `publish_with_correlation_id()` - Request/reply
- `publish_with_routing_key()` - Kinesis-style
- `publish_transactional()` - All-or-nothing
- `get_publish_result()` - Success/failure
- `get_message_id()` - Unique ID
- `get_publish_timestamp()` - When sent
- `retry_failed_publish()` - Automatic retry

#### Consuming (13 functions)
- `consume_message()` - Get from queue (blocking)
- `try_consume_message()` - Non-blocking
- `consume_with_timeout()` - Wait max N seconds
- `subscribe_to_topic()` - Listen to topic
- `create_subscription()` - Named subscription
- `get_subscriber_count()` - How many listeners
- `acknowledge_message()` - Mark processed
- `nack_message()` - Unprocessed, requeue
- `get_consumer_group()` - Which group
- `create_consumer_group()` - Kafka-style group
- `get_message_offset()` - Position in stream
- `seek_to_offset()` - Jump to position
- `consume_from_beginning()` - Replay messages

#### Dead Letter Queue (8 functions)
- `create_dead_letter_queue()` - DLQ setup
- `send_to_dlq()` - Move failed message
- `get_dlq_messages()` - Failed messages
- `replay_dlq_message()` - Retry from DLQ
- `get_dlq_length()` - How many failed
- `purge_dlq()` - Clear failed
- `get_dlq_stats()` - Metrics
- `alert_on_dlq_depth()` - Monitor failures

#### Message Format (5 functions)
- `create_message()` - Message wrapper
- `get_message_body()` - Content
- `get_message_headers()` - Metadata
- `get_message_properties()` - Attributes
- `parse_message()` - Deserialize

---

### Phase 27.5: Distributed Tracing (450-500 lines, 50 functions)

**Purpose:** Observability across services (OpenTelemetry patterns)

**Categories:**

#### Trace Management (12 functions)
- `create_trace()` - Start trace
- `create_span()` - Create span
- `add_span_to_trace()` - Add to trace
- `get_trace_duration()` - Total time
- `get_span_count_in_trace()` - # of spans
- `get_critical_path()` - Longest dependency chain
- `export_trace()` - Send to collector
- `get_trace_id()` - Unique ID
- `get_trace_status()` - Success/failure
- `get_trace_error_message()` - Error details
- `get_trace_tags()` - Metadata
- `set_trace_tag()` - Add metadata

#### Span Details (13 functions)
- `get_span_id()` - Unique ID
- `get_parent_span_id()` - Parent's ID
- `get_span_start_time()` - When started
- `get_span_end_time()` - When finished
- `get_span_duration()` - Total time
- `set_span_attribute()` - Add metadata
- `get_span_attribute()` - Query metadata
- `add_span_event()` - Log event in span
- `get_span_events()` - All events
- `set_span_status()` - OK/error
- `get_span_status()` - Current status
- `add_span_link()` - Cross-trace reference
- `get_span_links()` - All references

#### Instrumentation (12 functions)
- `instrument_http_request()` - Auto-trace HTTP
- `instrument_database_query()` - Auto-trace SQL
- `instrument_function_call()` - Decorator pattern
- `instrument_message_queue()` - Auto-trace queue
- `record_request_size()` - Bytes sent
- `record_response_size()` - Bytes received
- `record_error_type()` - Exception class
- `record_error_stack_trace()` - Full traceback
- `record_database_statement()` - SQL query
- `record_http_method_and_path()` - HTTP details
- `record_http_status_code()` - Response code
- `record_message_queue_operation()` - Op type

#### Context Propagation (8 functions)
- `extract_trace_context()` - From HTTP headers
- `inject_trace_context()` - Into HTTP headers
- `create_trace_header()` - W3C format
- `parse_trace_header()` - Extract parent ID
- `get_baggage()` - Cross-service metadata
- `set_baggage()` - Add cross-service data
- `propagate_context()` - Pass down
- `clear_context()` - Cleanup

#### Sampling & Filtering (5 functions)
- `create_sampler()` - Sampling config
- `should_sample_trace()` - Yes/no decision
- `set_sample_rate()` - % of traces
- `get_sampled_traces_count()` - How many
- `export_sampling_stats()` - Metrics

---

## Integration Points

### Service Discovery ↔ Load Balancing
- LB queries SD to find available instances
- LB notified when instances become healthy/unhealthy

### Load Balancing ↔ Circuit Breaker
- CB prevents routing to failing services
- CB reopens to allow requests to recover

### Circuit Breaker ↔ Message Queues
- CB integrates with queue consumers
- Failed processing triggers async retry via queue

### Message Queues ↔ Tracing
- Each message carries trace ID for correlation
- Queue operations automatically traced

### All ↔ Distributed Tracing
- All 4 modules emit spans for observability
- Complete request flow visible across services

---

## Production Deployment Scenario

```
User Request
     ↓
[Phase 24 HTTP Server]
     ↓
[Phase 26 OAuth Validation]
     ↓
[Phase 27.1 Service Discovery] → Find auth service
     ↓
[Phase 27.2 Load Balancer] → Choose instance
     ↓
[Phase 27.3 Circuit Breaker] → Check health
     ↓
[Phase 27.5 Distributed Trace] → Start span
     ↓
[Send HTTP Request + Trace Context]
     ↓
[Auth Service Processes] → Records span
     ↓
[Publishes Event to Queue] ← Phase 27.4
     ↓
[Consumer Group Processes] ← Phase 27.4
     ↓
[Records Processing Metrics] ← Phase 27.5
     ↓
[Response with Complete Trace]
```

---

## Implementation Patterns

### Type-Safe Design
```rust
pub enum CircuitState { Closed, Open, HalfOpen }
pub enum MessagePriority { High, Normal, Low }
pub enum TraceStatus { OK, Error }
pub struct ServiceInstance { id, host, port, healthy, weight }
pub struct Span { id, trace_id, parent_id, start, end, attributes }
```

### Error Handling
```rust
pub enum DistributedError {
    ServiceNotFound,
    AllInstancesDown,
    QueueFull,
    CircuitOpen,
    TracingFailed,
}
```

---

## Success Criteria

✅ 250+ functions across 5 modules  
✅ 3,000+ lines of production code  
✅ 50+ comprehensive unit tests  
✅ Production-ready resilience patterns  
✅ Zero unsafe code  
✅ Type-safe error handling  
✅ Full lib.rs integration  

---

## Delivery Timeline

**Estimated:** Single session (~60 minutes)

1. **Phase 27.1 Service Discovery** (~15 min) - 50 fn
2. **Phase 27.2 Load Balancing** (~15 min) - 50 fn
3. **Phase 27.3 Circuit Breaker** (~15 min) - 50 fn
4. **Phase 27.4 Message Queues** (~12 min) - 50 fn
5. **Phase 27.5 Distributed Tracing** (~8 min) - 50 fn
6. **Integration & Testing** (~5 min)

**Total:** ~60 minutes max, single-session delivery

---

**Phase 27 Ready to Launch** 🚀
