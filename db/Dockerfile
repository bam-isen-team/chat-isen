# chat-isen PostgreSQL database image.
#
# Every `*.up.sql` migration file is copied below and executed automatically
# by the official postgres image entrypoint, in alphabetical (migration)
# order, ONLY on the very first container start (empty data volume).
# Editing them afterwards has no effect until the volume is re-initialized:
#
#     docker compose down -v && docker compose up -d
#
# Note: `*.down.sql` files are intentionally NOT copied here — they exist
# to revert a migration on demand (cf. db/migrations/README.md).
#
# Tip: gen_random_uuid() is native since PostgreSQL 13, no need to install
# pgcrypto for SQL-side UUIDs.

FROM postgres:18-alpine

COPY migrations/*.up.sql /docker-entrypoint-initdb.d/
