# Database migrations

Each numbered pair `NNN_name.up.sql` / `NNN_name.down.sql` is one migration:
its `up` file applies it, its `down` file reverts it. Files run in
alphabetical — hence numeric — order. Never renumber a migration that has
already been applied anywhere.

## Fresh databases (bootstrap)

`db/Dockerfile` copies every `*.up.sql` (and only those — `down` files are
never executed at bootstrap) into the postgres image init directory, so a
brand-new data volume applies the whole history automatically:

    docker compose down -v && docker compose up -d

The postgres entrypoint only runs on an EMPTY data volume: this bootstrap
targets databases that start from zero (CI, a new machine, a reset dev
volume).

## Existing databases

An already-initialized volume does NOT pick up new migrations by itself.
Until the backend embeds a real migration runner (planned alongside the
auth milestones), apply them by hand against the running database
(credentials come from your `.env`):

    docker compose exec -T db psql -U chatisen -d chatisen \
      < db/migrations/NNN_name.up.sql

## Reverting a migration

    docker compose exec -T db psql -U chatisen -d chatisen \
      < db/migrations/NNN_name.down.sql

The `down` scripts drop in strict reverse creation order, inside a
transaction, so a half-applied state can never survive a failure.
