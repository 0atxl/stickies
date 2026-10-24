CREATE TABLE IF NOT EXISTS mutation_receipts (
    operation_id TEXT PRIMARY KEY,
    request_fingerprint TEXT NOT NULL,
    note_id INTEGER NOT NULL,
    revision TEXT NOT NULL,
    created_at INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_mutation_receipts_created_at ON mutation_receipts (created_at);
