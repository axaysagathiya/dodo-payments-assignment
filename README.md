# Invoice Payment System

A production-ready invoice management and payment processing system built with Rust, Axum, and PostgreSQL. This system features robust concurrency control, strict idempotency, and a separate Mock Payment Service Provider (PSP) for end-to-end testing.

## Architecture Overview

### Concurrency Control
The system uses **Database-Level Row Locking** (`SELECT ... FOR UPDATE`) to synchronize concurrent payment requests for the same invoice. This ensures that even if multiple clients or nodes attempt to pay an invoice simultaneously, only one request can proceed to the PSP at a time.

### Idempotency & Lifecycle
- **Strict Idempotency**: Each payment attempt is tracked by an `idempotency_key` and an accompanying `request_hash` (Blake3). Re-using a key with a different payload will trigger a `409 Conflict`.
- **Staleness Recovery**: If a payment attempt is stuck in `processing` (e.g., due to a crash or network error) for more than **60 seconds**, the system automatically marks it as failed and allows a new attempt to take over.
- **Atomic State Transitions**: All critical state changes (locking, upserting attempts, and finalizing invoices) are performed within database transactions.

## Setup Instructions

### Option 1: Docker (Recommended)
The easiest way to run the entire system (Database, Mock PSP, and Main API) is using Docker Compose:

```bash
docker-compose up --build
```

This will:
1. Start a PostgreSQL database.
2. Build and start the Mock PSP on port `3001`.
3. Build and start the Main API on port `8080` (automatically applying migrations).

### Option 2: Local
####  Database Configuration
install and start postgreSQL

create `.env` file and add these parameter

required:
```
DATABASE_URL="postgres://<user>:<pswd>@localhost/<database-name>"
```

optional:
```
MOCK_PSP_SERVER_URL="0.0.0.0:3001"
```
### run rust binaries
- run API server:
`cargo run --bin invoice-payment-system`

- run mock psp: `cargo run --bin mock_psp`

## APIs
cURL_commands.md file contains all the command to run APIs.
cURL_calls_example.md contains many example API calls and the response, that also cover below cases:
-  Register Business
- Create Customer
- Create invoice.
- running payment api for all the tokens including tok_timeout, tok_network_error.
- running payment api with the same item potency key and a different body payload
- Database not connected
- mock_psp service not reachable


If you have [Rest Client](https://marketplace.visualstudio.com/items?itemName=humao.rest-client) vs code extension, you can call APIs directly using api_calls.http file

## Tech Stack
Axum, tokio, PostgreSQL, serde, Blake3

## Demo Video

https://drive.google.com/drive/folders/1oS8gf18SNf4Y90IyYI0k521oe1W9rIlr?usp=drive_link