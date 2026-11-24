use thiserror::Error;

/// Application-wide HTTP errors (cf. M3): each variant becomes a clean API
/// response via `IntoResponse` — never a raw panic, never a leaked detail.
#[derive(Debug, Error)]
pub enum AppError {
    /// Well-formed JSON, semantically invalid payload: 400, field-precise.
    #[error("{0}")]
    Validation(String),

    /// Username or email already taken: 409, deliberately generic —
    /// a register probe must not reveal WHICH of the two exists.
    #[error("username or email already taken")]
    Conflict,

    /// Anything unexpected: 500 with a generic body — the real cause is
    /// logged server-side only (cf. ADR-001: no internal detail in answers).
    #[error("internal server error")]
    Internal,
}
