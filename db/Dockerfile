# chat-isen PostgreSQL database image.
#
# The init.sql file copied below is executed automatically by the official
# postgres image entrypoint ONLY on the very first container start (empty
# data volume). Editing it afterwards has no effect until the volume is
# re-initialized:
#
#     docker compose down -v && docker compose up -d
#
# Tip: gen_random_uuid() is native since PostgreSQL 13, no need to install
# pgcrypto for SQL-side UUIDs.

FROM postgres:18-alpine

COPY init.sql /docker-entrypoint-initdb.d/init.sql
