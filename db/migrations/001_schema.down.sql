-- =========================================================
-- ChatISEN: migration 001 (down) — revert db/migrations/001_schema.up.sql
--
-- Drops everything in the exact REVERSE order of creation (children
-- before parents), in one transaction: the database is either fully
-- reverted or untouched.
-- =========================================================
BEGIN;

-- Tables, children first (reverse of the up migration).
DROP TABLE attachments;
DROP TABLE quality_metrics;
DROP TABLE calls;
DROP TABLE memberships;
DROP TABLE messages;
DROP TABLE conversations;
DROP TABLE refresh_tokens;
DROP TABLE users;

-- Enumerated types.
DROP TYPE call_status;
DROP TYPE call_type;
DROP TYPE member_role;
DROP TYPE conversation_kind;
DROP TYPE presence_status;

COMMIT;
