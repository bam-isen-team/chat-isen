-- =========================================================
-- ChatISEN: migration 001 (up) - initial schema
-- =========================================================
BEGIN;

-- ---------- Enumerated types ----------
CREATE TYPE presence_status   AS ENUM ('online', 'away', 'offline');
CREATE TYPE conversation_kind AS ENUM ('private', 'group');
CREATE TYPE member_role       AS ENUM ('member', 'admin');
CREATE TYPE call_type         AS ENUM ('voice', 'video');
CREATE TYPE call_status       AS ENUM ('pending', 'ongoing', 'ended', 'rejected', 'missed');

-- ---------- users ----------
CREATE TABLE users (
    id              uuid            PRIMARY KEY DEFAULT gen_random_uuid(),
    username        varchar(32)     NOT NULL UNIQUE
                                    CHECK (username ~ '^[A-Za-z0-9_.-]{3,32}$'),
    email           varchar(254)    NOT NULL,
    -- argon2id hash (PHC format, e.g. "$argon2id$v=19$..."), computed in Rust.
    -- The plaintext password is NEVER stored or logged.
    password_hash   text            NOT NULL CHECK (password_hash LIKE '$argon2%'),
    display_name    varchar(64)     NOT NULL,
    avatar_url      text,
    presence_status presence_status NOT NULL DEFAULT 'offline',
    -- Bumped on global sign-out / password change; access tokens carry the
    -- matching "ver" claim so stale ones get rejected (cf. ADR-001 §3).
    token_version   int             NOT NULL DEFAULT 0,
    created_at      timestamptz     NOT NULL DEFAULT now(),
    updated_at      timestamptz     NOT NULL DEFAULT now()
);
-- Case-insensitive email uniqueness (Alice@x.fr = alice@x.fr).
CREATE UNIQUE INDEX users_email_lower_uq ON users (lower(email));

-- ---------- refresh_tokens (cf. ADR-001: JWT access + rotating opaque refresh) ----------
CREATE TABLE refresh_tokens (
    id          uuid        PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id     uuid        NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    -- We store the SHA-256 of the token, not the token itself: a database leak
    -- does not allow any token to be replayed.
    token_hash  bytea       NOT NULL UNIQUE,
    -- Tokens issued by the same login form a family, so that a replayed
    -- token revokes the whole lineage at once.
    family      uuid        NOT NULL,
    created_at  timestamptz NOT NULL DEFAULT now(),
    expires_at  timestamptz NOT NULL,
    revoked_at  timestamptz,
    -- Successor token after a rotation (cf. ADR-001 §3: the consumed token
    -- is marked as replaced); keeps the chain auditable.
    replaced_by uuid        REFERENCES refresh_tokens (id) ON DELETE SET NULL,
    user_agent  text,
    ip_address  inet,
    CHECK (expires_at > created_at)
);
-- "A user's refresh tokens" (global sign-out: revoke everything at once).
CREATE INDEX refresh_tokens_user_id_idx ON refresh_tokens (user_id);
-- Family revocation on reuse detection (cf. ADR-001 §3).
CREATE INDEX refresh_tokens_family_idx ON refresh_tokens (family);
-- Purging expired refresh tokens.
CREATE INDEX refresh_tokens_expires_at_idx ON refresh_tokens (expires_at);

-- ---------- conversations ----------
CREATE TABLE conversations (
    id              uuid              PRIMARY KEY DEFAULT gen_random_uuid(),
    kind            conversation_kind NOT NULL,
    name            varchar(100),
    -- For a private conversation: "<smaller user_id>:<larger user_id>".
    -- The UNIQUE index prevents two private conversations between the same people.
    direct_pair_key text              UNIQUE,
    created_by      uuid              REFERENCES users (id) ON DELETE SET NULL,
    created_at      timestamptz       NOT NULL DEFAULT now(),
    CHECK (
        (kind = 'group'   AND name IS NOT NULL AND direct_pair_key IS NULL)
     OR (kind = 'private' AND name IS NULL     AND direct_pair_key IS NOT NULL)
    )
);

-- ---------- messages ----------
CREATE TABLE messages (
    id              uuid        PRIMARY KEY DEFAULT gen_random_uuid(),
    conversation_id uuid        NOT NULL REFERENCES conversations (id) ON DELETE CASCADE,
    -- SET NULL: if an account is deleted, the other participants' history stays readable.
    sender_id       uuid        REFERENCES users (id) ON DELETE SET NULL,
    content         text        NOT NULL CHECK (char_length(content) BETWEEN 1 AND 4000),
    sent_at         timestamptz NOT NULL DEFAULT now(),
    edited_at       timestamptz
);
-- Most frequent query: a conversation's history, newest to oldest,
-- with cursor pagination on (sent_at, id).
CREATE INDEX messages_conversation_sent_at_idx
    ON messages (conversation_id, sent_at DESC, id DESC);

-- ---------- memberships ----------
CREATE TABLE memberships (
    conversation_id      uuid        NOT NULL REFERENCES conversations (id) ON DELETE CASCADE,
    user_id              uuid        NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    role                 member_role NOT NULL DEFAULT 'member',
    joined_at            timestamptz NOT NULL DEFAULT now(),
    -- Per-member read receipt (replaces a single status on the message,
    -- which makes no sense in a group).
    last_read_message_id uuid        REFERENCES messages (id) ON DELETE SET NULL,
    PRIMARY KEY (conversation_id, user_id)
);
-- "My conversations" (the PK already covers conversation_id -> user_id).
CREATE INDEX memberships_user_id_idx ON memberships (user_id);

-- ---------- calls ----------
CREATE TABLE calls (
    id              uuid        PRIMARY KEY DEFAULT gen_random_uuid(),
    conversation_id uuid        REFERENCES conversations (id) ON DELETE SET NULL,
    caller_id       uuid        REFERENCES users (id) ON DELETE SET NULL,
    callee_id       uuid        REFERENCES users (id) ON DELETE SET NULL,
    call_type       call_type   NOT NULL,
    status          call_status NOT NULL DEFAULT 'pending',
    created_at      timestamptz NOT NULL DEFAULT now(),   -- ringing started
    started_at      timestamptz,                          -- answered
    ended_at        timestamptz,
    CHECK (caller_id IS DISTINCT FROM callee_id OR caller_id IS NULL),
    CHECK (started_at IS NULL OR started_at >= created_at),
    CHECK (ended_at   IS NULL OR ended_at   >= COALESCE(started_at, created_at)),
    CHECK (status <> 'ongoing' OR started_at IS NOT NULL)
);
-- A user's call history (outgoing / incoming).
CREATE INDEX calls_caller_created_idx ON calls (caller_id, created_at DESC);
CREATE INDEX calls_callee_created_idx ON calls (callee_id, created_at DESC);

-- ---------- quality_metrics (data / AI track) ----------
CREATE TABLE quality_metrics (
    id           bigint      GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    call_id      uuid        NOT NULL REFERENCES calls (id) ON DELETE CASCADE,
    user_id      uuid        REFERENCES users (id) ON DELETE SET NULL,
    measured_at  timestamptz NOT NULL,
    latency_ms   real        CHECK (latency_ms  >= 0),
    packet_loss  real        CHECK (packet_loss BETWEEN 0 AND 1),
    jitter_ms    real        CHECK (jitter_ms   >= 0)
);
-- Time series for one call, and analyses by time period.
CREATE INDEX quality_metrics_call_measured_idx ON quality_metrics (call_id, measured_at);
CREATE INDEX quality_metrics_measured_brin   ON quality_metrics USING brin (measured_at);

-- ---------- attachments (BONUS) ----------
CREATE TABLE attachments (
    id              uuid         PRIMARY KEY DEFAULT gen_random_uuid(),
    message_id      uuid         NOT NULL REFERENCES messages (id) ON DELETE CASCADE,
    file_name       varchar(255) NOT NULL,
    size_bytes      bigint       NOT NULL CHECK (size_bytes > 0 AND size_bytes <= 25 * 1024 * 1024),
    mime_type       varchar(127) NOT NULL,
    -- Key in object storage; the download URL is generated on demand
    -- (signed, short-lived) after an access check.
    storage_key     text         NOT NULL UNIQUE,
    checksum_sha256 bytea        NOT NULL,
    created_at      timestamptz  NOT NULL DEFAULT now()
);
CREATE INDEX attachments_message_id_idx ON attachments (message_id);

COMMIT;