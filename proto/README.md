# gRPC in CoinBot.v3

## What is gRPC?

A way for processes to call functions on each other over the network — like a local
function call, but crossing process boundaries. Built on HTTP/2 + Protocol Buffers.

## The Contract: `.proto` files

Every gRPC service starts with a `.proto` file. It defines **messages** (data shapes)
and **services** (RPC endpoints). This is the single source of truth — both server
and client compile from the same file.

### Messages — "what data looks like"

```protobuf
message TicketRequest {
  string tx_hash = 1;
  string uid = 2;
  string from = 3;
  string to = 4;
  string ticker = 5;
  string amount = 6;
}
message TicketResponse {
  string ticket_id = 1;
  string timestamp = 2;
  bool accepted = 3;
}
```

Rules:
- Every field has a **type** (`string`, `int32`, `bool`, etc.), a **name**, and a **field id** (integer)
- The field id is the **wire tag** — what gets sent over the network (not the name)
- Rename the field freely (`user_id` → `customer_id`), keep the same field id → **backward compatible**
- Change the field id → **breaks everything** (old clients still send old id)
- Field id range: **1 to 536,870,911** (2²⁹ - 1)
- **1–15** = 1 byte on wire (use for most common fields)
- **16–2047** = 2 bytes on wire
- **19000–19999** = **reserved by protobuf** — cannot use
- Removing a field? Comment it out, don't delete (reserve the id)

### Services — "what you can call"

```protobuf
service DepositService {
  rpc CreateTicket0(TicketRequest) returns (TicketResponse);
}
```

Compiled by `tonic-build` → generates:
- **Server trait** (`DepositService` + `DepositServiceServer` wrapper)
- **Client stub** (`DepositServiceClient`)

## Client vs Server

```
┌─ Server (deposit-worker) ────────────────┐
│                                           │
│  struct DepositServer;                    │
│                                           │
│  impl DepositService for DepositServer {   │
│    async fn create_ticket0(...) { ... }   │
│  }                                        │
│                                           │
│  Server::builder()                        │
│    .add_service(DepositServiceServer::new(...))
│    .serve("0.0.0.0:50051")               │
│    .await?                                │
└───────────────────────────────────────────┘

┌─ Client (api_gateway) ──────────────────┐
│                                          │
│  let mut client =                        │
│    DepositServiceClient::connect(        │
│      "http://deposit-worker:50051"       │
│    ).await?;                              │
│                                          │
│  let resp = client                       │
│    .create_ticket0(TicketRequest { ... })│
│    .await?;                               │
│                                          │
│  println!("{}", resp.ticket_id);         │
└──────────────────────────────────────────┘
```

### Why field id matters

```protobuf
message TicketRequest {
  string tx_hash = 1;   // id 1 → sent as 1 on the wire
}

// Safe evolution:
message TicketRequest {
  string transaction_hash = 1;  // renamed, id 1 still = same wire tag → old clients work
}

// Breaking change:
message TicketRequest {
  string tx_hash = 2;  // changed from 1 → 2, old clients send id 1 → server sees unknown field
}
```

## Proto Types → Rust Types

| Proto | Rust |
|---|---|
| `string` | `String` |
| `int32` | `i32` |
| `int64` | `i64` |
| `bool` | `bool` |
| `repeated T` | `Vec<T>` |
| `message Nested` | `struct Nested` |

## How `common` ties it together

```
proto/wallet.proto          ← single source of truth
        │
 common/build.rs            ← tonic-build compiles proto at compile time
        │
 common/src/lib.rs          ← re-exports generated types + traits
        │
 deposit-worker             ← implements DepositService trait
 api_gateway                ← can call via DepositServiceClient
```

The `common` crate is only a library — it has no `main.rs` and cannot run.
It exists so all services share the **exact same** message types and service
traits. If the proto changes, all consuming crates must recompile.

## Proto service organization

The proto currently defines a single service:

```protobuf
service DepositService {
  rpc CreateTicket0(TicketRequest) returns (TicketResponse);
}
```

- `DepositService` is **implemented** by `deposit-worker` (gRPC server)
- `api_gateway` can call it via `DepositServiceClient`

## Separation of Duties (SoD)

Each business function runs in an isolated process. No single service performs
both deposit and withdrawal logic.

| Process | Responsibility | Cannot do |
|---|---|---|
| `deposit-worker` | Record deposits via gRPC | Process withdrawals |
| `api_gateway` | Auth, routing | Modify balances |

Enforced by:

1. **Crate boundaries** — no crate imports another service's internals
2. **Database isolation** — each service has its own DB credentials
3. **gRPC contracts** — services only expose what the proto defines

## Ethical Walls

An ethical wall prevents information flow between services that should not share data.

In this architecture, the wall is **inherent in the process boundaries**:

```
┌─ deposit-worker ─────────────┐
│                               │
│  owns: deposits DB            │
│  exposes: CreateTicket0       │  ← gRPC: "record this deposit"
│                               │
│  ❌ Cannot: auth sessions     │  ← no access to users table
│  ❌ Cannot: read api keys     │  ← not in schema
│                               │
└───────────────────────────────┘
```

`api_gateway` never touches deposit data directly. It calls `CreateTicket0`
through gRPC and gets a controlled response. No shared database, no shared
memory, no shared secrets.

**Three layers of enforcement:**

| Layer | Mechanism |
|---|---|
| Rust compiler | Crates only import `common` — no cross-service imports |
| Network | Docker — each container isolated, no DB cross-connect |
| Proto | Services only expose what the `.proto` allows |

## Adding a new RPC

1. Add the message types and RPC to `proto/wallet.proto`
2. Rebuild — `tonic-build` regenerates stubs automatically
3. Implement the new trait method in the server crate
4. Call the new method from the client crate
5. If the server or client doesn't implement the new RPC, **compilation fails**

This is the guarantee — mismatched contracts don't compile.

## Running the services

```sh
# Terminal 1 — start deposit-worker gRPC server
cargo run -p deposit

# Terminal 2 — start api_gateway HTTP server
cargo run -p api-gateway
```

## Redis Communication

gRPC handles sync calls (request-response). Redis handles async messages (events, signals).
Two Redis patterns are used:

### Redis Streams — persistent, replayable messages

Best for: transaction events, trade signals (must not lose data).

```
deposit-worker ──▶ "tx:deposits" stream ──▶ api_gateway consumes → push to UI
```

Streams persist until explicitly trimmed. If a consumer disconnects, it can resume
from where it left off — unlike pub/sub.

**Message format** (JSON — easy cross-language):

```json
{
  "type": "deposit",
  "user_id": "user_abc",
  "amount": "500.00",
  "currency": "USDT",
  "tx_id": "dep-tx-001",
  "timestamp": 1718000000
}
```

### Redis Pub/Sub — fire-and-forget

Best for: live dashboard updates, status notifications (ok to lose).

```
deposit-worker ──▶ "status:updates" pub ──▶ api_gateway → WebSocket → React
```

If no one is listening, the message disappears. That's fine for live updates —
the next one will arrive.

### Key naming conventions

| Pattern | Example |
|---|---|
| `tx:<type>` | `tx:deposits`, `tx:withdrawals` |
| `trade:<topic>` | `trade:signals` |
| `status:<service>` | `status:deposit`, `status:withdrawal` |

All services share the same Redis instance but use different key prefixes.
Enforcement: a service only has permission for its own key prefix (Docker network policy).

## Further reading

- [tonic docs](https://docs.rs/tonic)
- [Protocol Buffers guide](https://protobuf.dev/programming-guides/proto3/)
