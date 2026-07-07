// Switch to repo_optimizer database
db = db.getSiblingDB('repo_optimizer');

// NAPOMENA: kolekcija "job_status" je UKLONJENA odavde namerno.
// Status/napredak posla (processed_files/total_files/status) sada zivi u
// Postgres tabeli `analysis_jobs` (vidi postgres_init.sql) - ona ima FK na
// users pa moze i da se proveri vlasnistvo nad poslom (ko je submit-ovao analizu).
// Mongo ovde drzi samo "teske", varijabilnog oblika podatke (AST/problemi/predlozi).

// Create collections
db.createCollection('source_code');
db.createCollection('parsed_asts');
db.createCollection('problems');
db.createCollection('suggestions');

// Create indexes
db.source_code.createIndex({ "analysis_job_id": 1 });
db.source_code.createIndex({ "created_at": 1 });

db.parsed_asts.createIndex({ "analysis_job_id": 1 });
db.parsed_asts.createIndex({ "language": 1 });

db.problems.createIndex({ "analysis_job_id": 1 });
db.problems.createIndex({ "severity": 1 });
// ml_ranker_service upisuje rank_score nazad preko update_one po "id" - treba index.
db.problems.createIndex({ "id": 1 }, { unique: true });

db.suggestions.createIndex({ "problem_id": 1 });
db.suggestions.createIndex({ "analysis_job_id": 1 });

print("MongoDB initialized successfully!");
