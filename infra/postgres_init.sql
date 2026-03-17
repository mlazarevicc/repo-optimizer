-- Users table
CREATE TABLE IF NOT EXISTS users (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email VARCHAR(255) UNIQUE NOT NULL,
    password_hash VARCHAR(255) NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_users_email ON users(email);

-- Auth tokens table (for refresh tokens)
CREATE TABLE IF NOT EXISTS auth_tokens (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token_hash VARCHAR(255) NOT NULL,
    expires_at TIMESTAMP NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_auth_tokens_user_id ON auth_tokens(user_id);
CREATE INDEX idx_auth_tokens_expires_at ON auth_tokens(expires_at);

-- Analysis jobs table
CREATE TABLE IF NOT EXISTS analysis_jobs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    status VARCHAR(50) NOT NULL DEFAULT 'pending', -- pending, processing, completed, failed
    language VARCHAR(50) NOT NULL,
    file_name VARCHAR(255),
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    completed_at TIMESTAMP
);

CREATE INDEX idx_analysis_jobs_user_id ON analysis_jobs(user_id);
CREATE INDEX idx_analysis_jobs_status ON analysis_jobs(status);

-- Problems table (metadata only, full data in MongoDB)
CREATE TABLE IF NOT EXISTS problems (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    analysis_job_id UUID NOT NULL REFERENCES analysis_jobs(id) ON DELETE CASCADE,
    problem_type VARCHAR(100) NOT NULL,
    severity VARCHAR(20) NOT NULL, -- critical, high, medium, low
    line_start INT NOT NULL,
    line_end INT NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_problems_analysis_job_id ON problems(analysis_job_id);
CREATE INDEX idx_problems_severity ON problems(severity);

-- Insert test user for development (before auth_service is implemented)
INSERT INTO users (id, email, password_hash, created_at, updated_at)
VALUES (
    '00000000-0000-0000-0000-000000000001'::uuid,
    'test@repo-optimizer.local',
    '$2b$12$dummy.hash.for.testing.purposes.only',
    NOW(),
    NOW()
)
ON CONFLICT (email) DO NOTHING;

-- Verify test user was created
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM users WHERE id = '00000000-0000-0000-0000-000000000001'::uuid) THEN
        RAISE NOTICE 'Test user created successfully with ID: 00000000-0000-0000-0000-000000000001';
    END IF;
END $$;
