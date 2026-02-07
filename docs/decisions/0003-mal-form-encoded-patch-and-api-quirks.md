# ADR 0003: MAL API Quirks — Form-Encoded PATCH and NSFW Flag

## Status

Accepted

## Date

2026-02-07

## Context

During implementation of the MAL API v2 client, we encountered several behaviors that deviate from typical REST API conventions. These are documented here so future contributors understand the choices and don't "fix" code that looks wrong but is correct.

## Quirks

### 1. PATCH uses form-encoded body, not JSON

The `update_progress` endpoint (`PATCH /v2/anime/{id}/my_list_status`) requires `application/x-www-form-urlencoded` body:

```
num_watched_episodes=14
```

Sending JSON returns HTTP 400. This is documented in MAL's API reference but contradicts the convention that modern REST APIs accept JSON for mutation endpoints.

**Implementation**: We use `reqwest`'s `.form()` builder instead of `.json()`.

### 2. User list requires `nsfw=true`

The list endpoint (`GET /v2/users/@me/animelist`) filters out anime that MAL classifies as NSFW by default. This includes some mainstream titles (e.g., certain seinen anime). Without `nsfw=true`, users would have incomplete list imports with no indication that entries were silently dropped.

**Implementation**: We always pass `nsfw=true` on the list endpoint.

### 3. PKCE requires `plain` method only

See [ADR 0001](0001-mal-oauth2-pkce-without-oauth2-crate.md).

### 4. Pagination via full URL

List responses include `paging.next` as a complete URL (e.g., `https://api.myanimelist.net/v2/users/@me/animelist?offset=100&...`). We follow this URL directly rather than constructing offset parameters ourselves. When `paging.next` is `null`, we've reached the end.

### 5. Search query encoding

The search endpoint (`GET /v2/anime?q=...`) requires proper URL encoding of the query string, particularly for Japanese titles and special characters like `:` or `&`. We use `reqwest`'s `.query()` builder which handles percent-encoding correctly.

## Decision

Accept these quirks as-is and document them. The MAL client code reflects MAL's actual behavior, not REST conventions.

## Consequences

- Code reviewers should not change `.form()` to `.json()` on the PATCH endpoint.
- The `nsfw=true` parameter is not optional — removing it silently breaks list imports.
- Tests for type deserialization use sample JSON that matches MAL's actual response shape, not idealized schemas.
