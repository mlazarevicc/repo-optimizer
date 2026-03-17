// Switch to repo_optimizer database
db = db.getSiblingDB('repo_optimizer');

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

db.suggestions.createIndex({ "problem_id": 1 });
db.suggestions.createIndex({ "analysis_job_id": 1 });

print("MongoDB initialized successfully!");
