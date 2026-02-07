# ADR 0001: MAL OAuth2 PKCE Without the oauth2 Crate

## Status

Accepted

## Date

2026-02-07

## Context

MyAnimeList requires OAuth2 with PKCE for API access. The `oauth2` crate (v4) is already a workspace dependency (used by the AniList stub). However, MAL has a non-standard PKCE requirement: it only supports the `plain` code challenge method, where the challenge equals the verifier verbatim. The standard method is `S256` (SHA-256 hash of the verifier), which is what the `oauth2` crate defaults to and what most providers expect.

We needed to decide how to implement the MAL auth flow.

## Options Considered

### Option A: Use the `oauth2` crate with `plain` PKCE override

The `oauth2` crate supports setting `PkceCodeChallengeMethod::new("plain")`, but this fights the crate's design:
- Requires manually constructing a `PkceCodeChallenge` that skips hashing
- The crate's `PkceCodeVerifier` generates verifiers and computes S256 challenges together; separating them requires working around the API
- Error handling wraps everything in the crate's own error types, adding conversion boilerplate
- The crate pulls in additional dependencies (base64, sha2) that are unnecessary for `plain` method

### Option B: Raw reqwest + url crate (chosen)

Implement the PKCE flow directly:
1. Generate a random 128-character URL-safe verifier string
2. Set `code_challenge = verifier` (plain method)
3. Build the authorization URL manually
4. Open browser via the `open` crate
5. Spawn a one-shot TCP listener on `localhost:19742` for the redirect
6. Parse the `?code=` query parameter with the `url` crate
7. POST to the token endpoint with `reqwest` (form-encoded)

### Option C: Add a dedicated MAL OAuth library

No mature, maintained crate exists specifically for MAL's OAuth flow. The few that existed were abandoned or tied to older API versions.

## Decision

Option B — raw `reqwest` + `url` for the MAL OAuth2 flow.

## Rationale

- **Simplicity**: The entire flow is ~175 lines across two functions (`authorize`, `refresh`) plus helpers. The `oauth2` crate abstraction would not reduce this significantly given the workarounds needed.
- **No new heavy dependencies**: `url` is lightweight and already used transitively. `open` is a thin wrapper around `xdg-open`/`open`/`start`. No SHA-256 or base64 needed.
- **MAL-specific quirks are explicit**: The `plain` challenge method, form-encoded PATCH bodies, and `nsfw=true` requirement are visible in the code rather than hidden behind crate configuration.
- **Debuggability**: When MAL returns cryptic errors (which it does), having direct control over the HTTP requests makes diagnosis straightforward.

## Consequences

- The `oauth2` crate remains in `Cargo.toml` for future AniList/Kitsu use (both support S256). It is not used by the MAL module.
- If MAL ever adds S256 support, we could switch, but there is no benefit — `plain` works and the code is already written.
- The localhost listener on port 19742 is blocking (synchronous TCP accept). This is intentional: the auth flow is user-initiated and one-shot. If the port is occupied, the error message is clear.
- Token refresh is a single POST — no crate needed for that either.

## References

- [MAL API v2 OAuth docs](https://myanimelist.net/apiconfig/references/authorization)
- [RFC 7636 — PKCE](https://datatracker.ietf.org/doc/html/rfc7636) (Section 4.2: plain method)
- `crates/kurozumi-api/src/mal/auth.rs` — implementation
