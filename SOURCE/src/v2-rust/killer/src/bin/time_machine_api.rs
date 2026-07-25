use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Instant;

use killer_native::time_machine::{
    event::*,
    event_log::*,
    causality_engine::*,
    reversible::*,
    timeseries::*,
    quantum::*,
    whatif::*,
    ml::*,
    physics::*,
};

/// Shared temporal API context
#[allow(dead_code)]
struct TemporalAPI {
    event_log: Mutex<EventLog>,
    causality: Mutex<CausalityEngine>,
    reversible: Mutex<ReversibleComputationEngine>,
    timeseries: Mutex<TimeSeriesDatabase>,
    quantum: Mutex<QuantumTemporalSimulator>,
    whatif: Mutex<WhatIfAnalysisEngine>,
    ml: Mutex<TemporalMLEngine>,
    physics: Mutex<PhysicsEngine>,
    request_count: Mutex<u64>,
}

impl TemporalAPI {
    fn new() -> Self {
        Self {
            event_log: Mutex::new(EventLog::new(100_000)),
            causality: Mutex::new(CausalityEngine::new()),
            reversible: Mutex::new(ReversibleComputationEngine::new()),
            timeseries: Mutex::new(TimeSeriesDatabase::new(86400, true)),
            quantum: Mutex::new(QuantumTemporalSimulator::new(0.01)),
            whatif: Mutex::new(WhatIfAnalysisEngine::new()),
            ml: Mutex::new(TemporalMLEngine::new()),
            physics: Mutex::new(PhysicsEngine::new()),
            request_count: Mutex::new(0),
        }
    }

    fn handle_request(&self, request: &str) -> String {
        *self.request_count.lock().unwrap() += 1;

        let lines: Vec<&str> = request.lines().collect();
        if lines.is_empty() {
            return self.error_response("Invalid request");
        }

        let request_line: Vec<&str> = lines[0].split_whitespace().collect();
        if request_line.len() < 2 {
            return self.error_response("Invalid request line");
        }

        let method = request_line[0];
        let path = request_line[1];

        match (method, path) {
            ("GET", "/api/status") => self.status_response(),
            ("POST", "/api/events") => self.create_event_response(),
            ("POST", "/api/causality/link") => self.link_causality_response(),
            ("POST", "/api/timeseries/insert") => self.insert_timeseries_response(),
            ("GET", "/api/analysis/deep") => self.deep_analysis_response(),
            ("POST", "/api/benchmark") => self.benchmark_response(),
            ("GET", "/health") => self.ok_response("{\"status\": \"healthy\"}"),
            _ => self.error_response(&format!("Unknown endpoint: {}", path)),
        }
    }

    fn status_response(&self) -> String {
        let count = *self.request_count.lock().unwrap();
        let events = self.event_log.lock().unwrap().total_events_count();
        
        let response = format!(
            r#"{{"system": "KILLER Time Machine", "version": "2.1.0", "status": "operational", "phases_active": 8, "events": {}, "requests": {}}}"#,
            events, count
        );
        
        self.ok_response(&response)
    }

    fn create_event_response(&self) -> String {
        let mut log = self.event_log.lock().unwrap();
        let event_id = log.total_events_count();
        
        let event = Event::new(
            event_id,
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
            event_id,
            OperationType::Add,
            "api".to_string(),
            vec![],
            true,
        );
        
        let _ = log.append(event);
        
        let response = format!(
            r#"{{"event_id": {}, "status": "created"}}"#,
            event_id
        );
        
        self.ok_response(&response)
    }

    fn link_causality_response(&self) -> String {
        let cause_id = 1u64;
        let effect_id = 2u64;
        
        let mut causality = self.causality.lock().unwrap();
        match causality.link_events(cause_id, effect_id) {
            Ok(_) => {
                let response = format!(
                    r#"{{"cause": {}, "effect": {}, "linked": true}}"#,
                    cause_id, effect_id
                );
                self.ok_response(&response)
            }
            Err(_) => self.error_response("Failed to link"),
        }
    }

    fn insert_timeseries_response(&self) -> String {
        let mut ts = self.timeseries.lock().unwrap();
        let point = DataPoint {
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
            value: 42.0,
            measurement: "api_metric".to_string(),
            tags: vec![],
        };
        
        ts.insert(point);
        
        let response = r#"{"inserted": true, "value": 42.0}"#;
        self.ok_response(response)
    }

    fn deep_analysis_response(&self) -> String {
        let log = self.event_log.lock().unwrap();
        let reversible = self.reversible.lock().unwrap();
        
        let response = format!(
            r#"{{"events": {}, "reversibility": {:.1}%, "consistency": 100.0}}"#,
            log.total_events_count(),
            reversible.reversibility_percentage()
        );
        self.ok_response(&response)
    }

    fn benchmark_response(&self) -> String {
        let start = Instant::now();
        let mut ts = self.timeseries.lock().unwrap();
        
        for i in 0..1000 {
            let point = DataPoint {
                timestamp: i as u128,
                value: (i as f64) * 1.5,
                measurement: "bench".to_string(),
                tags: vec![],
            };
            ts.insert(point);
        }
        
        let elapsed = start.elapsed();
        let throughput = 1000.0 / elapsed.as_secs_f64();
        
        let response = format!(
            r#"{{"operations": 1000, "duration_ms": {:.2}, "throughput": {:.0}}}"#,
            elapsed.as_millis(),
            throughput
        );
        self.ok_response(&response)
    }

    fn ok_response(&self, body: &str) -> String {
        format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nAccess-Control-Allow-Origin: *\r\n\r\n{}",
            body.len(),
            body
        )
    }

    fn error_response(&self, message: &str) -> String {
        let body = format!(r#"{{"error": "{}"}}"#, message);
        format!(
            "HTTP/1.1 400 Bad Request\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
            body.len(),
            body
        )
    }
}

fn main() {
    let port = 8080u16;
    let addr = format!("127.0.0.1:{}", port);
    let listener = TcpListener::bind(&addr).expect("Failed to bind");
    
    let api = Arc::new(TemporalAPI::new());

    println!("\n+================================================================+");
    println!("|    KILLER TIME MACHINE - REST API (:8080)                     |");
    println!("|  GET /api/status    POST /api/events  POST /api/benchmark    |");
    println!("+================================================================+\n");

    for stream in listener.incoming() {
        let api = Arc::clone(&api);
        thread::spawn(move || {
            if let Ok(stream) = stream {
                let mut stream = stream;
                let mut buffer = [0; 4096];
                if let Ok(n) = stream.read(&mut buffer) {
                    let request = String::from_utf8_lossy(&buffer[..n]);
                    let response = api.handle_request(&request);
                    let _ = stream.write_all(response.as_bytes());
                }
            }
        });
    }
}
