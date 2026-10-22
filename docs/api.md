# VibeBuddy API

How the daemon, the App and the box talk to `vibebuddy-api`, the service that holds accounts and voice (ADR-0017, ADR-0018). The service itself is the Worker `vibebuddy-api` in the closed-source `vibebuddy-cloud` repo; this is the part this repo implements.

It lives at `https://api.korekore.ai`; staging is `https://api-staging.korekore.ai`. Releases use production and every other build uses staging, as for the Relay. Production has no route until accounts are complete (vibebuddy-cloud#11).

## Accounts

An account belongs to a person and is signed into on a computer (ADR-0018). The daemon keeps the session; the App only opens the browser.

### Signing in

Sign-in follows OAuth for native apps (RFC 8252) with PKCE (RFC 7636): the browser proves who the person is, and the daemon receives the result on a loopback address.

1. The daemon makes a code verifier (43 to 128 characters of `A-Z a-z 0-9 - . _ ~`), its challenge `BASE64URL(SHA-256(verifier))` without padding, and a random `state`.
2. The App opens, in the system browser:

   ```text
   GET /v1/sign-in?redirect_uri=<loopback>&code_challenge=<challenge>&code_challenge_method=S256&state=<state>
   ```

   `redirect_uri` must be `http://127.0.0.1:<port>/…` or `http://[::1]:<port>/…`; the daemon uses `http://127.0.0.1:7331/v1/account/callback`. Anything else gets an error page.
3. The person enters an email address and then the 6-digit code mailed from `noreply@vibekeys.dev`. A code works for 10 minutes and allows 5 tries; an address gets at most 5 codes an hour. The pending sign-in lasts 15 minutes.
4. The browser is redirected to `<redirect_uri>?code=<code>&state=<state>`. The daemon checks `state` and, within 2 minutes, exchanges the code once:

   ```text
   POST /v1/session
   {"code":"…","code_verifier":"…","redirect_uri":"<the same loopback URI>"}
   ```

   It answers `{"token":"…"}`, a session token of 43 base64url characters, or 400 `{"error":"invalid_grant"}` (wrong, expired or reused code, wrong verifier or redirect URI) or `{"error":"invalid_request"}`. A code is spent by the first try, whatever its outcome.

Signing in again with the same address finds the same account. Each signed-in computer has its own session.

### Sessions

The daemon sends the token as `Authorization: Bearer <token>` and keeps it in `config_dir()/account` (mode 0600). A session doesn't expire. A 401 `{"error":"unauthorized"}` means it was revoked or the account was deleted: the daemon forgets it and reports signed out.

- `GET /v1/account` answers

  ```json
  {"id":"…","email":"person@example.com","github_id":null,"created_at":"2026-10-22T08:00:00.000Z","today":{"day":"2026-10-22","commands":0,"replies":0}}
  ```

  An account has an `email` or a `github_id`, never both. `today` counts voice in the current UTC day.
- `POST /v1/sign-out` revokes this session only: 204.
- `DELETE /v1/account` deletes the account with all its sessions and usage: 204.

## Voice

`POST /v0/voice/transcribe` and `POST /v0/voice/speak` need no account yet and run on staging only; they move to `/v1/voice/…` with sessions and the daily allowance in vibebuddy-cloud#7. Their bodies and answers are described in vibebuddy-cloud's README until then.
