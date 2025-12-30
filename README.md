# RepoOptimizer

> 🚀 Distributed Code Optimization Platform with Multi-Language Support, Static Analysis, and ML-Powered Suggestions

**Status**: Active Development | **Language**: Rust | **License**: MIT

---

## 📋 Table of Contents

- [Overview](#overview)
- [Features](#features)
- [Architecture](#architecture)
- [Technologies](#technologies)
- [License](#license)

---

## Overview

**RepoOptimizer** is a distributed system designed to automate code analysis and optimization across multiple programming languages. The platform helps developers quickly identify and fix code issues, focusing on what truly matters:

- **Performance bottlenecks**: N+1 loops, inefficient algorithms, memory leaks
- **Security vulnerabilities**: SQL injection, missing input validation, hard-coded credentials
- **Code smells**: Long methods, large classes, duplicated code, deep nesting
- **Best practices**: Naming conventions, documentation, error handling, test coverage

### The Problem We Solve

Developers spend hours manually reviewing code for quality issues. Existing tools are:
- ❌ Language-specific (not universal)
- ❌ Expensive (SaaS pricing models)
- ❌ Hard to integrate (complex configurations)
- ❌ Lacking actionable suggestions (just "there's a problem")

**RepoOptimizer** solves this with:
- ✅ Multi-language support (Python, JavaScript, Go, Java, Rust, and more)
- ✅ Affordable/free (open source + low-cost SaaS)
- ✅ Easy integration (CLI tool + Web UI + API)
- ✅ Precise suggestions (before/after code examples with explanations)

---

## Features

### 🎯 Core Features (Project Phase)

#### Multi-Language Code Parsing
- **Python**: PEP 8 violations, idiom detection
- **JavaScript/TypeScript**: ES6+ patterns, async issues
- **Go**: Idiomatic patterns, error handling
- **Java**: OOP patterns, resource management
- **Rust**: Memory safety, borrowing violations (educational)

#### Static Analysis Engine
- **Code Smell Detection**:
  - Long methods (>20 lines)
  - Large classes (>50 methods)
  - Unused variables/imports
  - Magic numbers
  - Deep nesting (>4 levels)
  - Duplicated code blocks
  - God classes & Feature envy

- **Performance Issues**:
  - N+1 loops
  - Inefficient algorithms (O(n²) where O(n log n) is possible)
  - Memory leaks (retained references)
  - Inefficient string operations

- **Security Issues**:
  - SQL injection vulnerabilities
  - Missing input validation
  - Hard-coded credentials
  - Insecure random generation
  - Missing CSRF protection

- **Style & Best Practices**:
  - Naming conventions
  - Function complexity scoring
  - Missing documentation
  - Error handling coverage
  - Test coverage gaps

#### ML-Powered Ranking
- **XGBoost Model** for prioritizing problems by impact
- **Severity Classification**: Critical, High, Medium, Low
- **Impact Scoring**: How much will fixing this improve the codebase?
- **Model Versioning** for consistency

#### Suggestion Generation
- **Before/After Code Examples**: Exact how to fix the problem
- **Detailed Explanations**: Why it's an issue and how fixing helps
- **Language-Specific Suggestions**: Pythonic vs Rustic patterns
- **Safe Refactoring**: Never breaks functionality

#### Web UI & CLI
- **Web IDE**: Browser-based code analysis with syntax highlighting
- **CLI Tool**: `repo-optimizer analyze myproject/` for quick local analysis
- **API Gateway**: REST API for programmatic access
- **Real-time Feedback**: See issues as you code (optional)

---

## Architecture

### System Overview

```
User Input
    ↓
API Gateway (REST API + Auth)
    ↓
RabbitMQ/Kafka (Job Queue)
    ↓
Parser Service
(Tree-sitter Multi-lang AST)
    ↓
Analysis Service
(Detects Problems)
    ↓
ML Ranker Service
(XGBoost Ranking)
    ↓
Suggestion Generator Service
(Generates Fixes)
    ↓
Database (PostgreSQL + MongoDB)
    ↓
API Gateway (Returns Results)
    ↓
Frontend/CLI (User Output)
```

### Microservices Architecture

| Service | Responsibility | Tech Stack | Database |
|---------|----------------|-----------|----------|
| **Auth_Service** | User management, JWT tokens, OAuth | Actix-web, JWT | PostgreSQL |
| **Parser_Service** | Multi-language AST parsing | Tree-sitter, Tokio | MongoDB |
| **Analysis_Service** | Static code analysis, metrics | Custom rules, Tokio | PostgreSQL |
| **ML_Ranker_Service** | Problem prioritization | XGBoost, ONNX Runtime | Redis (cache) |
| **Suggestion_Generator_Service** | Generate fix suggestions | Template engine, LLM-ready | MongoDB |
| **API_Gateway** | REST API, rate limiting, composition | Actix-web, JWT middleware | N/A |

### Data Flow

```
CLI/Web UI
    ↓
[Upload Code] → API_Gateway (authenticate)
    ↓
RabbitMQ: Create "analyze_code" job
    ↓
Parser_Service: Parse into AST
    ↓ (AST → MongoDB)
Analysis_Service: Run detection rules
    ↓ (Problems → PostgreSQL)
ML_Ranker_Service: Score & prioritize
    ↓ (Scores → MongoDB)
Suggestion_Generator_Service: Create fixes
    ↓ (Suggestions → MongoDB)
API_Gateway: Return to user
    ↓
Web UI / CLI: Display results
```

---

## Technologies

### Backend Stack
- **Language**: Rust
- **Web Framework**: Actix-web (fast, reliable)
- **Async Runtime**: Tokio (non-blocking I/O)
- **Serialization**: Serde (JSON/binary)
- **Database Client**: SQLx (PostgreSQL), MongoDB driver

### Data Processing
- **Code Parsing**: Tree-sitter (supports 15+ languages)
- **AST Analysis**: Custom walkers and pattern matchers
- **Job Queue**: RabbitMQ or Kafka (async processing)
- **Caching**: Redis (frequently accessed results)

### Machine Learning
- **Inference Framework**: ONNX Runtime (lightweight, fast)
- **ML Model**: XGBoost (problem ranking)
- **Feature Engineering**: Code metrics → model input

### Frontend
- **Framework**: React 18+
- **Code Editor**: CodeMirror 6 (syntax highlighting, themes)
- **Charts**: Recharts (visualize issues distribution)
- **HTTP Client**: Axios (API communication)
- **Build Tool**: Vite (fast development)

### DevOps
- **Containerization**: Docker
- **Orchestration**: Docker Compose (dev), Kubernetes (diplomski)
- **Database**: PostgreSQL + MongoDB
- **Message Queue**: RabbitMQ or Kafka

### Quality Assurance
- **Testing**: Rust's built-in test framework
- **CLI Tool**: Clap (command-line parsing)
- **Documentation**: cargo doc, MkDocs

---

## Future Enhancements

### Phase 1: LLM Integration (Code Generation)
```
✨ NEW: LLM_Fixer_Service (Service 7)
- Integrate with OpenAI GPT-4 or Llama 2
- Auto-generate refactored code (not just suggestions)
- Generate unit tests based on detected issues
- Verify generated code doesn't break existing tests
- Human review workflow before applying changes
```

### Phase 2: Automated Testing Integration
```
✨ NEW: Testing Service
- Auto-generate unit tests for problematic code
- Verify refactoring doesn't break tests
- Code coverage analysis and gaps
- Test quality scoring
- Mutation testing for test quality
```

### Phase 3: Resilience
```
🛡️ Circuit Breaker: Prevent cascading failures
🛡️ Bulkhead: Isolate critical paths
🛡️ Timeout: Prevent hanging requests
```

### Phase 4: Performance Optimization
```
⚡ Caching Strategy: Redis intelligent caching
⚡ Batch Processing: Analyze multiple files in parallel
⚡ Database Indexing: Optimize query performance
⚡ Kubernetes Deployment: Full orchestration with auto-scaling
```

---

## Security Considerations

### Data Protection
- ✅ HTTPS only
- ✅ User code encrypted at rest
- ✅ JWT tokens with expiration
- ✅ SQL injection prevention (parameterized queries)
- ✅ XSS prevention (input sanitization)

### Credential Management
- ✅ API keys stored securely
- ✅ GitHub tokens in environment variables
- ✅ No secrets in git history
- ✅ Regular security audits

### Compliance
- ✅ GDPR data deletion on request
- ✅ Data residency options
- ✅ Audit logging
- ✅ SOC 2 compliance (enterprise)

---

## License

This project is licensed under the **MIT License** - see the [LICENSE](LICENSE) file for details.

---

## Changelog

### Version 0.1.0 (Planning Phase)
- 📋 Project specification complete
- 🎯 Microservices architecture designed
- 📚 Documentation prepared
- 🚀 Ready for development

---

**Made with ❤️ by Milan Lazarevic**

Last Updated: December 30, 2025
