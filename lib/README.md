# lib

Shared Rust crates for CoinBot.v3 services.

Currently a single crate, [`share`](share/), used by every backend process:

| Consumer       | Path                     |
| -------------- | ------------------------ |
| API gateway    | `process/api_gateway`    |
| Deposit worker | `process/deposit-worker` |
| Trade engine   | `process/trade-engine`   |

## Crate: `share`

Common building blocks: configuration, error types, logging, Redis, PostgreSQL
models, and on-chain helpers.

### Modules

| Module   | Purpose                                             | Key items                        |
| -------- | --------------------------------------------------- | -------------------------------- |
| `auth`   | EIP-191 (`personal_sign`) wallet signature recovery | `recover_wallet`                 |
| `cache`  | Redis key/value store and pub/sub                   | `Cache`                          |
| `config` | Environment configuration helpers                   | `ProcessConfig`, `ServiceConfig` |
| `db`     | PostgreSQL models, filters, and lookups             | see below                        |
| `erc20`  | ERC-20 `transfer` calldata decoding                 | `Erc20`                          |
| `error`  | Shared error types                                  | `ProcessError`, `ServiceError`   |
| `logger` | Global logger initialisation                        | `init_logger`                    |
| `models` | Domain models                                       | `TradeSignal`                    |
| `rpc`    | Ethereum JSON-RPC helpers                           | `Rpc`                            |

### Database (`db`)

| Item                                                           | Description                                                                                       |
| -------------------------------------------------------------- | ------------------------------------------------------------------------------------------------- |
| `db::user::User`                                               | User wallet, uid, `balance`, `locked_balance`                                                     |
| `db::contracts::{Contract, Contracts, ContractFilter, Status}` | Signed contracts and fund accounting (`snap_balance`, `init_fund`, `available_fund`, `used_fund`) |
| `db::deposit::{Deposit, DepositFilter, Status}`                | On-chain USDT deposits                                                                            |
| `db::trade::{TradeOrder, TradeOrders, Status}`                 | Order-level trade log                                                                             |
| `db::get_uid(pool, wallet)`                                    | wallet → uid                                                                                      |
| `db::wallet_for_uid(pool, uid)`                                | uid → wallet                                                                                      |

### Configuration

`ProcessConfig::new("api_gateway")` reads `API_GATEWAY_KEY` first, then falls
back to `KEY`, so shared variables (`DATABASE_URL`) work without per-process
duplication while overrides (`API_GATEWAY_DATABASE_URL`) are supported.

### Protobuf / gRPC

`build.rs` compiles `process/proto/wallet.proto` and
`process/proto/analyzer.proto` into gRPC stubs. The generated types are
re-exported at the crate root, e.g.:

```rust
use share::{TicketRequest, deposit_service_client::DepositServiceClient};
use share::analyzer_service_client::AnalyzerServiceClient;
```

Building requires `protoc` on the `PATH`.

## Build & test

```bash
cargo build -p share          # requires protoc
cargo test  -p share          # cache tests need Redis
```

The `tests/` directory holds integration tests: `cache.rs` (needs Redis) and
`erc20_transfer.rs` (pure).
