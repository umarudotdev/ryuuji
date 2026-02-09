# ADR 0002: User-Provided MAL Client ID via Config

## Status

Accepted

## Date

2026-02-07

## Context

MAL API v2 requires an OAuth2 Client ID for all API access. We needed to decide how users provide this credential.

## Options Considered

### Option A: Ship a bundled Client ID

Embed the Client ID in the binary. Simpler for users — auth works out of the box.

Downsides:
- MAL's terms require the Client ID owner to be responsible for usage. Shipping one for all users creates liability.
- If the key is revoked or rate-limited, all users are affected simultaneously.
- The Client ID would be trivially extractable from the binary, enabling abuse.
- MAL has historically revoked keys from open-source projects that shipped bundled credentials.

### Option B: User registers their own app (chosen)

The user creates an API application at https://myanimelist.net/apiconfig, sets the redirect URI to `http://localhost:19742`, and pastes their Client ID into `config.toml` under `services.mal.client_id`.

### Option C: Proxy server

Route all MAL API calls through a server we operate, hiding the Client ID server-side.

Downsides:
- Requires running and paying for infrastructure.
- Single point of failure.
- Privacy concern — all user data flows through our server.
- Defeats the purpose of a local-first desktop app.

## Decision

Option B — user-provided Client ID in config.

## Rationale

- **No infrastructure**: Ryuuji is a local-first application. No server to maintain or pay for.
- **No credential leakage risk**: The Client ID never leaves the user's machine.
- **Per-user rate limits**: Each user has their own API quota.
- **Precedent**: Other open-source MAL tools (Taiga, MALClient, Trackma) use this approach.
- **One-time setup**: Registering takes ~2 minutes. The redirect URI (`http://localhost:19742`) is documented.

## Config Shape

```toml
[services.mal]
enabled = false
client_id = ""
```

`MalConfig` replaced the generic `ServiceToggle` struct:

```rust
pub struct MalConfig {
    pub enabled: bool,
    pub client_id: Option<String>,
}
```

AniList and Kitsu retain `ServiceToggle` (they use different auth models — AniList uses implicit grant with a public Client ID, Kitsu uses password grant).

## Consequences

- Users must perform a one-time registration at myanimelist.net/apiconfig before using MAL sync.
- The Settings page includes MAL configuration with the client ID field.
- Empty `client_id` means MAL auth will fail with a clear error — no silent fallback.
- If MAL changes their API registration process, only documentation needs updating, not code.
