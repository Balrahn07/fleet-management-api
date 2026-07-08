# Fleet Management API — Project Notes

## Goal

This project is a Rust backend API for managing vehicles in a fleet management platform.

It teaches backend fundamentals using:

* Rust
* Axum
* PostgreSQL
* SQLx
* REST APIs
* Layered backend architecture

---

## Current Architecture

Request flow:

```text
HTTP Request
    ↓
Router
    ↓
Handler
    ↓
Service
    ↓
Repository
    ↓
PostgreSQL
```

Each layer has a separate responsibility.

---

## `main.rs`

Application entry point.

Responsibilities:

* Load environment variables from `.env`
* Initialize logging
* Read `DATABASE_URL`
* Create the PostgreSQL connection pool
* Build `AppState`
* Create routes
* Start the Axum HTTP server

Important concept:

```rust
PgPool
```

is a PostgreSQL connection pool shared by the application.

---

## `routes.rs`

Defines API routes.

Example:

```rust
.route("/vehicles", get(list_vehicles))
.route("/vehicles", post(create_vehicle))
.route("/vehicles/{id}", get(get_vehicle))
```

Responsibilities:

* Map HTTP method + path to handler function
* Attach shared application state using `.with_state(state)`

Examples:

```text
GET    /vehicles      → list_vehicles
POST   /vehicles      → create_vehicle
GET    /vehicles/{id} → get_vehicle
```

---

## `handlers.rs`

HTTP layer.

Responsibilities:

* Extract data from the HTTP request
* Call the service layer
* Return HTTP responses

Examples of Axum extractors:

```rust
State(state): State<AppState>
Path(id): Path<Uuid>
Json(request): Json<CreateVehicleRequest>
```

Meaning:

* `State<AppState>` gets shared application state
* `Path<Uuid>` gets the ID from the URL
* `Json<T>` gets JSON body data

Handlers should not contain business logic.

---

## `services.rs`

Business logic layer.

Responsibilities:

* Validate input
* Apply business rules
* Call repositories
* Convert repository errors into HTTP-level errors for now

Example business rules:

* VIN cannot be empty
* Model cannot be empty
* New vehicles start with status `"offline"`
* Missing vehicle becomes `404 Not Found`
* Database error becomes `500 Internal Server Error`

Example:

```rust
Result<Vehicle, StatusCode>
```

means:

* `Ok(vehicle)` → success
* `Err(StatusCode)` → HTTP error response

---

## `repositories.rs`

Database access layer.

Responsibilities:

* Execute SQL queries using SQLx
* Read from PostgreSQL
* Insert into PostgreSQL
* Return database results to the service layer

Example:

```rust
sqlx::query_as!(
    Vehicle,
    "SELECT id, vin, model, status, created_at, updated_at FROM vehicles"
)
```

The repository should not know about HTTP status codes.

It returns database errors such as:

```rust
sqlx::Error
```

---

## `models.rs`

Data structures.

Current models:

```rust
Vehicle
CreateVehicleRequest
```

### `Vehicle`

Represents a vehicle stored in the system.

Contains backend-managed fields:

* `id`
* `status`
* `created_at`
* `updated_at`

### `CreateVehicleRequest`

Represents JSON sent by the client when creating a vehicle.

Contains only client-provided fields:

* `vin`
* `model`

Important concept:

```text
Vehicle != CreateVehicleRequest
```

DTOs protect the backend from letting clients control fields like `id` or `status`.

---

## `state.rs`

Shared application state.

Currently contains:

```rust
pub struct AppState {
    pub db: PgPool,
}
```

This allows handlers/services/repositories to access the shared PostgreSQL connection pool.

Before PostgreSQL, this project used:

```rust
Arc<Mutex<Vec<Vehicle>>>
```

That was temporary in-memory state.

Now PostgreSQL stores data persistently.

---

## `migrations/`

Contains SQL migration files.

Migrations describe database schema changes.

Example:

```sql
CREATE TABLE vehicles (
    id UUID PRIMARY KEY,
    vin TEXT NOT NULL UNIQUE,
    model TEXT NOT NULL,
    status TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
```

SQLx tracks applied migrations in:

```text
_sqlx_migrations
```

---

## Database Table: `vehicles`

Columns:

```text
id          UUID PRIMARY KEY
vin         TEXT NOT NULL UNIQUE
model       TEXT NOT NULL
status      TEXT NOT NULL
created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
updated_at  TIMESTAMPTZ NOT NULL DEFAULT now()
```

Important constraints:

* `id` is the primary key
* `vin` must be unique
* `vin`, `model`, and `status` cannot be null

---

## Important Rust Concepts Used

### `State<AppState>`

Axum extractor meaning:

```text
Get AppState from application state.
```

### `Path<Uuid>`

Axum extractor meaning:

```text
Get UUID from URL path.
```

### `Json<T>`

Axum extractor meaning:

```text
Deserialize request/response JSON as T.
```

### `.await`

Waits for an async operation to finish.

Used for database calls.

### `map_err`

Converts one error type into another.

Example:

```rust
.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
```

converts:

```rust
sqlx::Error
```

into:

```rust
StatusCode
```

### `ok_or`

Converts:

```rust
Option<T>
```

into:

```rust
Result<T, E>
```

Example:

```rust
option.ok_or(StatusCode::NOT_FOUND)
```

means:

* `Some(vehicle)` → `Ok(vehicle)`
* `None` → `Err(404)`

---

## Current Endpoints

### Health check

```http
GET /
```

Returns:

```text
OK
```

### List vehicles

```http
GET /vehicles
```

Returns all vehicles from PostgreSQL.

### Get vehicle by ID

```http
GET /vehicles/{id}
```

Returns:

* `200 OK` with vehicle JSON if found
* `404 Not Found` if missing

### Create vehicle

```http
POST /vehicles
```

Request body:

```json
{
  "vin": "VF123",
  "model": "Tesla Model Y"
}
```

Backend generates:

* `id`
* `status = "offline"`
* `created_at`
* `updated_at`

---

### Update vehicle status

```http
PUT /vehicles/{id}
```

Request body:

```json
{
  "status": "maintenance"
}
```
Returns:

- 200 OK with updated vehicle JSON if found
- 400 Bad Request if status is invalid
- 404 Not Found if missing

### Delete vehicle

```http
DELETE /vehicles/{id}
```

Returns:

- 204 No Content if deleted
- 404 Not Found if missing

## Next Steps

Planned improvements:

1. Add validation layer
2. Add tests
3. Add Docker Compose
4. Add authentication
5. Add Redis caching
6. Add Kafka/event-driven telemetry later
