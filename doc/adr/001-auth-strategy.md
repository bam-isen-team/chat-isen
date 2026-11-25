# ADR-001: Authentication Strategy — JWT (Access + Refresh)

| | |
|---|---|
| **Status** | Accepted |
| **Date** | 2026-11-23 |
| **Deciders** | chat-isen team (auth discussion) |
| **Scope** | Issues #7 (backend auth) and #8 (frontend auth); constrains #12/#16 (WebSocket handshake) |
| **Relates to** | Class diagram (`User`, `Session`), architecture diagram |

## Context

Issue #7 requires defining and documenting the backend authentication strategy: account persistence, secure credential storage, a request verification mechanism, and a **documented** choice between stateful server-side sessions and stateless JWTs.

Key context elements:

- **Product**: a real-time chat application (single-origin Preact SPA, a single Rust/Axum backend, PostgreSQL).
- **Real time**: messaging relies on a WebSocket gateway (#12) — browsers cannot send an `Authorization` header during the WebSocket handshake.
- **Confidentiality**: conversations are private; the ability to quickly revoke a compromised account is a major requirement (threat model #26).
- **Scale**: single instance initially; Redis is planned by #18 for horizontal scaling.
- **Outlook**: the intent is to adopt an **API-first** posture and to support **non-browser** clients in the future (native mobile).
- **The class diagram** envisioned a `Session` class (token, `isValid()`, `revoke()`) — to be transposed into the JWT world through revocable refresh tokens.

## Decision

**JWT-based stateless authentication**: a short-lived signed **access token**, paired with **opaque, rotating, revocable refresh tokens** persisted in the database.

### 1. Tokens

**Access token**

- JWT signed with **HS256** (HMAC-SHA256); secret of at least 64 random bytes supplied through the `JWT_SECRET` environment variable — never hard-coded.
- Algorithm **pinned** during verification (any other `alg` is rejected).
- Lifetime: **10 minutes**.
- Minimal claims, no sensitive data (the payload is readable base64): `sub` (user UUID), `iat`, `exp`, `jti`, `ver` (token version, see §3).
- Transport: `Authorization: Bearer <access>` header; on the web client, it is kept **in memory only** (never in `localStorage`) — see #8.

**Refresh token**

- **Opaque** random string (CSPRNG, 256 bits) with no decryptable meaning.
- Stored **hashed** (SHA-256) in the `refresh_tokens` table — a database leak does not allow replaying any token.
- Lifetime: **30 days**.
- Delivered to the web client in a cookie: `HttpOnly; Secure; SameSite=Lax; Path=/auth/refresh` (JS cannot read it; XSS cannot exfiltrate it; minimal CSRF surface).
- For a future mobile client: delivery in the response body — a documented extension, out of scope for #7.

### 2. Endpoints (aligned with issue #7)

| Endpoint | Purpose | Status codes |
|---|---|---|
| `POST /auth/register` | Validation, Argon2id hashing, username/email uniqueness, access + refresh issuance | `201`, `400`, `409` |
| `POST /auth/login` | Constant-time verification, access + refresh issuance | `200`, `400`, `401` |
| `POST /auth/refresh` | Consumes the refresh cookie, **rotation**: new refresh + new access; replay detection → family revocation | `200`, `401` |
| `POST /auth/logout` | Revokes the current refresh token + its family; the client discards the in-memory access token | `204` |

**Generic** error responses ("invalid credentials", without revealing whether the email or the password is at fault).

### 3. Rotation, replay and revocation

- **Systematic rotation** on every `/auth/refresh`: the consumed refresh token is marked as replaced (`replaced_by`), and a new one takes its place.
- **Reuse detection**: if an already-consumed or revoked refresh token is presented again → likely compromise → **immediate revocation of the entire token family**.
- **Global sign-out** (password change, "sign out everywhere"): deletion of all refresh tokens **and** increment of `users.token_version`; since access tokens carry a `ver` claim checked against this version, every already-issued access token (≤ 10 min) also becomes invalid.
- `users.token_version` is additionally checked on protected requests only when required (sensitive operations) — otherwise revocation relies on the short access-token lifetime.

### 4. WebSocket authentication (for #12)

Since browsers cannot send headers during the WS handshake, and putting a JWT in a query string is **prohibited** (leakage through proxy logs):

- authenticated endpoint `POST /auth/ws-ticket` → issues a **single-use ticket** (random identifier, TTL ≈ 30 s);
- the client uses it in the **first message** after the WebSocket connection is established;
- the ticket is consumed and invalidated immediately (single use, minimal window).

### 5. SQL schema (`up`/`down` migrations)

- `users`: `id` UUID PK, `username` UNIQUE, `email` UNIQUE, `password_hash` (Argon2id), `token_version` INT, `created_at`, `updated_at`.
- `refresh_tokens`: `id` UUID PK, `user_id` FK `ON DELETE CASCADE`, `token_hash` UNIQUE, `family UUID`, `created_at`, `expires_at`, `revoked_at` (NULL), `replaced_by` (NULL, FK).
- No `sessions` table — no server-side sessions.

### 6. Cross-cutting security measures

- Passwords hashed with **Argon2id** (unique salt, default OWASP parameters); never stored in plaintext.
- **Rate limiting** on `/auth/login`, `/auth/register` and `/auth/refresh`.
- Secrets supplied only through environment variables (`JWT_SECRET`, `DATABASE_URL`).
- No token, password or secret in logs; minimal JWT payload.
- End-user TLS everywhere (terminated at the reverse proxy, #14).
- Negative tests (rstest): invalid credentials, duplicates, expired/revoked/replayed refresh tokens, unexpected `alg`, stale `ver`.

## Consequences

**Positive**

- No database access per authenticated request → local verification through the signature.
- Horizontal scaling without shared state (aligned with the future multi-instance architecture #18).
- **API-first** posture: non-browser clients (mobile) possible without adaptation.
- Revocable, rotating, hashed refresh tokens: revocation capability where it is needed, signature and opacity where they are needed.

**Negative / accepted risks**

- A stolen access token remains usable for **≤ 10 min** (no `jti` denylist; deemed unnecessary given the lifetime and the `ver` claim).
- Higher complexity: rotation, replay detection, `token_version`, WS ticket.
- Server-side logout does not kill the still-living access token — the client commits to discarding it (#8).
- The `JWT_SECRET` key is a critical asset: compromising it allows forging any identity → rotation possible by simply changing the variable (access tokens survive ≤ 10 min).

## Alternatives considered

**Server-side sessions (stateful)** — rejected after an argued comparison:

- *Strengths*: trivially immediate revocation, `HttpOnly` cookies immune to XSS token theft, transparent WebSocket handshake, perfect match with the `Session` class of the diagram, implementation simplicity.
- *Reasons for rejection*: priority was given to the **multi-platform** outlook (non-browser clients), to **offloading the database** on every request (a benefit that grows with traffic and multi-instance deployments), and to the **API-first** posture; the drawbacks encountered (per-request lookup, CSRF hardening) were deemed less costly to bear than the long-term constraints of sessions.
- *Consensus*: both approaches were technically defensible for our use case; this decision is a trade-off, not a technical knock-out.

**Opaque sessions stored in Redis** — variant not retained at this stage (it would introduce an infrastructure dependency prematurely; Redis will arrive with #18).

**Signed-cookie sessions (JWT in an `HttpOnly` cookie)** — discarded: it combines the CSRF exposure of cookies with the non-revocability of JWTs, with no net benefit here.

## References

- Issue #7 — backend auth; Issue #8 — frontend auth; Issue #12 — WebSocket gateway
- RFC 8725 (Best Current Practices for JWTs)
- `doc/diagram/` — architecture and class diagrams (`Session` class)
- Design discussion from 2026-11-23 (sessions vs JWT comparison; security/STRIDE aspects for #26)
