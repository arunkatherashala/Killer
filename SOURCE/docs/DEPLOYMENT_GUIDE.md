# Killer Language V2.1 - Deployment Guide

## Table of Contents
1. [Quick Start](#quick-start)
2. [Development Setup](#development-setup)
3. [Docker Deployment](#docker-deployment)
4. [Kubernetes Deployment](#kubernetes-deployment)
5. [Configuration](#configuration)
6. [Monitoring & Observability](#monitoring--observability)
7. [Security Verification](#security-verification)
8. [Performance Tuning](#performance-tuning)
9. [Troubleshooting](#troubleshooting)

---

## Quick Start

### Prerequisites
- Windows 10+, macOS 10.15+, or Linux (Ubuntu 20.04+)
- Git, Rust 1.75+, Docker & Docker Compose (for containerized deployment)
- 4GB RAM minimum, 8GB recommended
- 2 CPU cores minimum, 4 cores recommended

### Clone & Build
```bash
# Clone repository
git clone https://github.com/killer-lang/killer.git
cd killer

# Build from source
cargo build --release

# Run tests
cargo test --release

# Run example
./target/release/killer-native examples/01_hello.killer
```

### Docker Quick Start
```bash
# Build Docker image
docker build -t killer:2.1.0 .

# Run single container
docker run -it killer:2.1.0 examples/01_hello.killer

# Start full stack with monitoring
docker-compose up -d

# View runtime logs
docker-compose logs -f killer-runtime

# Access services
# - Killer:   http://localhost:8080
# - Grafana:  http://localhost:3000
# - Prometheus: http://localhost:9091
# - Jaeger:   http://localhost:16686
```

---

## Development Setup

### Local Build Requirements

#### Windows
```powershell
# Install build tools
winget install Git.Git Rustlang.Rust.MSVC

# Install optional dependencies
winget install LLVM.LLVM Docker.Docker

# Build
cargo build --release --features "phase16-ghost,phase17-adaptive,phase18-pgo,phase19-assassin,phase20-isolation,phase21-audit,vector-optimization"
```

#### macOS
```bash
# Install Homebrew if not already installed
/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"

# Install dependencies
brew install rust git llvm pkg-config

# For seccomp support (optional)
brew install libseccomp

# Build
cargo build --release --features "phase16-ghost,phase17-adaptive,phase18-pgo,phase19-assassin,phase20-isolation,phase21-audit,vector-optimization"
```

#### Linux (Ubuntu/Debian)
```bash
# Update packages
sudo apt-get update && sudo apt-get upgrade -y

# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env

# Install dependencies
sudo apt-get install -y \
    build-essential \
    pkg-config \
    libseccomp-dev \
    libseccomp2 \
    llvm \
    clang

# Build
cargo build --release --features "phase16-ghost,phase17-adaptive,phase18-pgo,phase19-assassin,phase20-isolation,phase21-audit,vector-optimization"
```

### Development Workflow
```bash
# Start development build
cargo build

# Run tests continuously
cargo test -- --nocapture --test-threads=1

# Check code quality
cargo clippy --all

# Format code
cargo fmt --all

# Generate documentation
cargo doc --no-deps --open
```

---

## Docker Deployment

### Single Container Deployment

```bash
# Build image
docker build -t killer:2.1.0 .

# Run with minimal configuration
docker run -d \
  --name killer-app \
  -p 8080:8080 \
  -p 9090:9090 \
  -v $PWD/workspace:/workspace \
  killer:2.1.0
```

### Multi-Container Stack (Development)

```bash
# Start all services
docker-compose up -d

# Verify services
docker-compose ps

# View service logs
docker-compose logs -f

# Stop all services
docker-compose down
```

### Production Docker Deployment

```bash
# Build production image
docker build -t killer:2.1.0-prod \
  --build-arg BUILD_DATE=$(date -u +'%Y-%m-%dT%H:%M:%SZ') \
  --build-arg VCS_REF=$(git rev-parse --short HEAD) \
  .

# Push to registry
docker tag killer:2.1.0-prod myregistry.azurecr.io/killer:2.1.0-prod
docker push myregistry.azurecr.io/killer:2.1.0-prod

# Run with security options
docker run -d \
  --name killer-prod \
  --security-opt seccomp=unconfined \
  --cap-add SYS_ADMIN \
  --cap-add SYS_PTRACE \
  -p 8080:8080 \
  -p 9090:9090 \
  -v killer-workspace:/workspace \
  -v killer-logs:/app/logs \
  -e KILLER_LOG_LEVEL=info \
  -e KILLER_AUDIT_ENABLED=true \
  -e KILLER_THREAT_DETECTION=enabled \
  --restart unless-stopped \
  killer:2.1.0-prod
```

---

## Kubernetes Deployment

### Prerequisites
- Kubernetes 1.24+ cluster (minikube, EKS, AKS, GKE, etc.)
- kubectl 1.24+
- Helm 3.0+ (optional, for advanced deployments)

### Simple Kubernetes Deployment

```yaml
# killer-deployment.yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: killer-runtime
  namespace: default
  labels:
    app: killer
    version: "2.1.0"
spec:
  replicas: 3
  selector:
    matchLabels:
      app: killer
  template:
    metadata:
      labels:
        app: killer
    spec:
      containers:
      - name: killer
        image: killer:2.1.0
        imagePullPolicy: Always
        ports:
        - containerPort: 8080
          name: http
        - containerPort: 9090
          name: metrics
        env:
        - name: KILLER_LOG_LEVEL
          value: "info"
        - name: KILLER_AUDIT_ENABLED
          value: "true"
        - name: KILLER_THREAT_DETECTION
          value: "enabled"
        - name: PROMETHEUS_ENABLED
          value: "true"
        resources:
          requests:
            memory: "512Mi"
            cpu: "500m"
          limits:
            memory: "1024Mi"
            cpu: "1000m"
        livenessProbe:
          httpGet:
            path: /health
            port: 8080
          initialDelaySeconds: 10
          periodSeconds: 30
        readinessProbe:
          httpGet:
            path: /ready
            port: 8080
          initialDelaySeconds: 5
          periodSeconds: 10
        securityContext:
          runAsNonRoot: true
          runAsUser: 1000
          allowPrivilegeEscalation: false
          capabilities:
            add:
            - SYS_ADMIN
            - SYS_PTRACE
        volumeMounts:
        - name: workspace
          mountPath: /workspace
        - name: audit-logs
          mountPath: /app/logs/audit
      volumes:
      - name: workspace
        emptyDir: {}
      - name: audit-logs
        emptyDir: {}

---
apiVersion: v1
kind: Service
metadata:
  name: killer-service
spec:
  type: LoadBalancer
  selector:
    app: killer
  ports:
  - name: http
    port: 8080
    targetPort: 8080
  - name: metrics
    port: 9090
    targetPort: 9090
```

Deploy to Kubernetes:
```bash
# Apply manifest
kubectl apply -f killer-deployment.yaml

# Verify deployment
kubectl get deployments
kubectl get pods

# Check service
kubectl get svc killer-service

# View logs
kubectl logs -f deployment/killer-runtime

# Scale deployment
kubectl scale deployment killer-runtime --replicas=5
```

### Advanced Kubernetes with Helm

```bash
# Create Helm values file
cat > killer-values.yaml <<EOF
replicaCount: 3
image:
  repository: killer
  tag: 2.1.0
  pullPolicy: Always

service:
  type: LoadBalancer
  port: 8080
  metricsPort: 9090

resources:
  limits:
    cpu: 1000m
    memory: 1024Mi
  requests:
    cpu: 500m
    memory: 512Mi

autoscaling:
  enabled: true
  minReplicas: 2
  maxReplicas: 10
  targetCPUUtilizationPercentage: 70
  targetMemoryUtilizationPercentage: 80

monitoring:
  prometheus:
    enabled: true
  jaeger:
    enabled: true
  grafana:
    enabled: true
EOF

# Deploy with Helm (if chart available)
helm install killer ./chart -f killer-values.yaml
```

---

## Configuration

### Environment Variables

```bash
# Logging
KILLER_LOG_LEVEL=info                    # trace, debug, info, warn, error, critical
KILLER_LOG_FORMAT=json                   # json or text

# Performance
KILLER_JIT_ENABLED=true                  # Enable JIT compilation
KILLER_MEMOIZATION_ENABLED=true          # Enable memoization cache
KILLER_VECTOR_ENABLED=true               # Enable vector optimizations

# Security
KILLER_SECCOMP_ENABLED=true              # Enable seccomp filtering
KILLER_CGROUPS_ENABLED=true              # Enable resource limits
KILLER_AUDIT_ENABLED=true                # Enable audit logging
KILLER_THREAT_DETECTION=enabled          # Enable threat intelligence

# Monitoring
PROMETHEUS_ENABLED=true                  # Enable Prometheus metrics
JAEGER_ENABLED=true                      # Enable Jaeger tracing
JAEGER_AGENT_HOST=localhost              # Jaeger agent host
JAEGER_AGENT_PORT=6831                   # Jaeger agent port

# Compliance
KILLER_COMPLIANCE_MODE=strict             # strict, standard, permissive
KILLER_AUDIT_RETENTION_DAYS=90           # Audit log retention
```

### TOML Configuration File (deployment.toml)

See `deployment.toml` in root directory for comprehensive configuration options.

Key sections:
- `[build]`: Compilation settings and feature flags
- `[performance]`: Optimization targets
- `[security]`: Security module configuration
- `[deployment]`: Container and Kubernetes settings
- `[testing]`: Test suite configuration
- `[monitoring]`: Prometheus, Jaeger, logging settings
- `[compliance]`: Audit, compliance, and SLO settings
- `[ci_cd]`: CI/CD pipeline configuration
- `[scaling]`: Auto-scaling policies

---

## Monitoring & Observability

### Prometheus Metrics

Access metrics at `http://localhost:9091`

Key metrics:
```
killer_execution_time_ms          - Execution time per operation
killer_memory_usage_bytes         - Memory usage tracking
killer_jit_compilations_total     - JIT compilation count
killer_memoization_hits_total     - Cache hits
killer_threat_detections_total    - Threat detection count
killer_audit_events_total         - Audit events logged
killer_seccomp_blocks_total       - Seccomp syscall blocks
```

### Grafana Dashboards

Access Grafana at `http://localhost:3000` (admin/admin)

Pre-configured dashboards:
1. **System Overview** - CPU, memory, I/O performance
2. **Performance** - JIT, memoization, vector ops
3. **Security** - Threats, audit events, violations
4. **Compliance** - Audit coverage, threat detection
5. **Resources** - Seccomp blocks, cgroups limits

### Jaeger Distributed Tracing

Access Jaeger UI at `http://localhost:16686`

Trace components:
- Compilation time
- Execution time
- Memory allocations
- Security checks
- Audit logging

### Audit Logs

Audit logs stored in `/app/logs/audit` or PostgreSQL (if configured)

Format: JSON with timestamp, level, component, action, target, details

```json
{
  "timestamp": "2024-01-20T15:30:45.123Z",
  "level": "INFO",
  "component": "threat_intelligence",
  "action": "threat_detected",
  "target": "user_code",
  "details": {
    "threat_type": "privilege_escalation",
    "severity": "critical",
    "pattern": "setuid"
  },
  "source": "ptrace_audit",
  "result": "blocked"
}
```

---

## Security Verification

### Pre-Deployment Security Checks

```bash
# Run security tests
cargo test --release --test test_phase19_assassin --test test_phase20_isolation --test test_phase21_audit

# Run complex security scenario
./target/release/killer-native examples/test_complex_security.killer

# Verify seccomp profile
docker run --rm killer:2.1.0 --show-seccomp-profile

# Check threat detection
docker run --rm killer:2.1.0 --show-threat-rules

# Validate compliance
docker run --rm killer:2.1.0 --compliance-report
```

### Post-Deployment Verification

```bash
# Check audit coverage
kubectl logs deployment/killer-runtime | grep "audit"

# Verify threat detection active
kubectl logs deployment/killer-runtime | grep "threat_intelligence"

# Monitor security score
kubectl exec -it pods/killer-runtime -- killer --compliance-score

# Review audit trail
kubectl logs deployment/killer-runtime | grep "audit_logger"
```

---

## Performance Tuning

### Build Optimization

```bash
# Release build with all optimizations
cargo build --release \
  --features "phase16-ghost,phase17-adaptive,phase18-pgo,phase19-assassin,phase20-isolation,phase21-audit,vector-optimization" \
  -Z build-std --target x86_64-unknown-linux-gnu

# LTO optimization
export RUSTFLAGS="-C target-cpu=native -C opt-level=3 -C lto=fat"
cargo build --release

# Profile-guided optimization
cargo rustc --release -- -C llvm-args=-pgo-warn-missing-function
```

### Runtime Tuning

```bash
# Enable JIT aggressive mode
KILLER_JIT_THRESHOLD=100 docker-compose up

# Increase memoization cache
KILLER_MEMO_CACHE_SIZE=512M docker-compose up

# Vector operation width
KILLER_VECTOR_WIDTH=512 docker-compose up

# Parallel execution
KILLER_MAX_WORKERS=8 docker-compose up
```

### Kubernetes Resource Tuning

```yaml
# requests: minimum guaranteed
# limits: maximum allowed

resources:
  requests:
    memory: "1Gi"    # Request 1GB
    cpu: "1000m"     # Request 1 CPU
  limits:
    memory: "2Gi"    # Limit to 2GB
    cpu: "2000m"     # Limit to 2 CPUs
```

---

## Troubleshooting

### Common Issues

#### Build Failures
```bash
# Clean build
cargo clean
cargo build --release

# Check Rust version
rustc --version  # Should be 1.75+

# Update Rust
rustup update
```

#### Docker Build Issues
```bash
# Check Docker
docker --version

# Clean Docker cache
docker system prune -a

# Rebuild with verbose output
docker build --no-cache -t killer:2.1.0 . --progress=plain
```

#### Performance Issues
```bash
# Check system resources
docker stats killer-runtime

# Monitor CPU usage
docker top killer-runtime

# Check logs for warnings
docker logs killer-runtime | grep -i "warn\|error"
```

#### Security Test Failures
```bash
# Run individual security test
cargo test test_phase19_seccomp -- --nocapture

# Check seccomp configuration
docker run --rm killer:2.1.0 --show-seccomp-profile

# Verify ptrace auditing
docker logs killer-runtime | grep "ptrace"
```

#### Kubernetes Issues
```bash
# Check pod status
kubectl describe pod killer-runtime-xxxxx

# Check resource limits
kubectl top pods

# Check events
kubectl get events

# Debug pod
kubectl exec -it pod/killer-runtime-xxxxx -- /bin/bash
```

### Log Locations

Development:
- Source: `src/`
- Tests: `tests/`
- Examples: `examples/`

Docker:
- Container logs: `docker logs <container-id>`
- Audit logs: `/app/logs/audit`

Kubernetes:
- Pod logs: `kubectl logs <pod-name>`
- Audit logs: Within pod at `/app/logs/audit`

PostgreSQL:
- Audit table: `SELECT * FROM audit_events;`

---

## Support & Documentation

### Resources
- **GitHub**: https://github.com/killer-lang/killer
- **Documentation**: https://docs.killer-lang.org
- **Issues**: https://github.com/killer-lang/killer/issues
- **Security Issues**: security@killer-lang.org

### Version Compatibility

| Component | Version | Status |
|-----------|---------|--------|
| Rust | 1.75+ | Required |
| Docker | 20.10+ | Required for containers |
| Kubernetes | 1.24+ | For K8s deployment |
| Ubuntu | 20.04+ | Recommended for Linux |
| LLVM | 14+ | Optional, for better compilation |

---

Last Updated: January 2024
Version: 2.1.0 (Production Ready)
