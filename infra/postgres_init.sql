-- Users table
CREATE TABLE IF NOT EXISTS users (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email VARCHAR(255) UNIQUE NOT NULL,
    password_hash VARCHAR(255) NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_users_email ON users(email);

-- NAPOMENA: auth_tokens tabela (za refresh tokene) je namerno izostavljena.
-- Trenutna implementacija koristi kratkorocne JWT tokene (24h expiry iz JWT_EXPIRATION_HOURS).
-- Refresh token mehanizam je odlozena funcionalnost - tabela ce biti dodata kad se
-- implementira /api/auth/refresh i /api/auth/logout ruta.

-- Analysis jobs table
-- OVO je izvor istine za status/vlasnistvo posla (zamenjuje ad-hoc "job_status"
-- kolekciju u MongoDB - vidi obrazlozenje u odgovoru, ukratko: ima FK na users
-- pa mozemo da provericmo vlasnistvo i da pravimo "moje analize" listing).
CREATE TABLE IF NOT EXISTS analysis_jobs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    status VARCHAR(50) NOT NULL DEFAULT 'pending', -- pending, processing, completed, failed
    language VARCHAR(50) NOT NULL,
    file_name VARCHAR(255),
    total_files INT NOT NULL DEFAULT 1,
    processed_files INT NOT NULL DEFAULT 0,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    completed_at TIMESTAMP
);

CREATE INDEX idx_analysis_jobs_user_id ON analysis_jobs(user_id);
CREATE INDEX idx_analysis_jobs_status ON analysis_jobs(status);

-- NAPOMENA: Postgres "problems" tabela je namerno IZBACENA odavde.
-- Puni Problem zapisi (sa svim poljima: severity, linije, kod, rank_score...)
-- vec zive u MongoDB "problems" kolekciji, indeksiranoj po analysis_job_id.
-- Drzati isti podatak u dve baze bi znacilo pisati u oba mesta na svaki nalaz
-- i rizik da se desinhronizuju - nema realne potrebe da Postgres ima svoju kopiju.

-- Insert test user for development (lozinka: "TestPassword123!", hash generisan sa bcrypt cost=12,
-- kompatibilan sa Rust `bcrypt` crate-om koji koristi auth_service)
INSERT INTO users (id, email, password_hash, created_at, updated_at)
VALUES (
    '00000000-0000-0000-0000-000000000001'::uuid,
    'test@repo-optimizer.local',
    '$2b$12$rQ5Lmn5bRQpXkPGlxP3kYeb9uccDqirWcs/Gj5O9nU0b7wcYX7uu.',
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
