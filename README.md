## System Architecture

The system follows a classic client-server architecture optimized for Web3 security and low-latency execution, fully containerized for development and production parity.

#### 1. Authentication Flow (EIP-191)

- **Challenge:** React frontend requests a unique nonce from the Rust server via `/api/user/auth`.
- **State:** Server generates a UUID nonce and stores it in a **Moka** cache (5-min TTL).
- **Signature:** User signs the nonce using **Ethers v6** via their browser wallet.
- **Verification:** Server recovers the address; if valid, it establishes a session via HttpOnly cookie.

#### 2. Core Components

- **State Manager:** Centralized `AppState` cloned across Actix workers (PostgreSQL pool, KuCoin client, Moka caches).
- **Trading Engine:** (Planned) Volatility monitoring and automated execution.
- **Orchestration:** **Docker Compose** manages networking and environment synchronization between the UI and API.

---

### File System Structure

```text
.
├── docker-compose.yml           # Orchestration for Frontend & Backend
├── server/                      # Rust Backend (Actix-Web)
│   ├── src/                     # Handlers, Models, Routes, State
│   ├── Cargo.toml               # Backend dependencies
│   └── Dockerfile               # Multi-stage Rust build
└── react/                       # Frontend (Vite + TS)
    ├── src/                     # Components, Hooks, Pages, Services
    ├── package.json             # Frontend dependencies
    └── Dockerfile               # Node.js development environment
```

---

### Tech Stack

| Layer        | Technology                                             |
| :----------- | :----------------------------------------------------- |
| **Backend**  | Rust (Actix-Web), PostgreSQL (SQLx), Moka Cache        |
| **Frontend** | React 19, TypeScript, Vite, Ethers v6, Axios           |
| **DevOps**   | **Docker**, **Docker Compose**                         |
| **Web3**     | EIP-191 Signature Verification (ethers-rs / ethers.js) |

---

### Development Setup

**Prerequisites:** Docker & Docker Compose installed.

1. **Configure Environment:**
   Ensure your `.env` file exists in the root (or as specified in `docker-compose.yml`) with `DATABASE_URL` and KuCoin credentials.

2. **Launch System:**

   ```bash
   docker compose up --build
   ```

   _Note for Fedora users: The volume mounts utilize the `:Z` flag to handle SELinux permissions._

3. **Access:**
   - **Frontend:** `http://localhost:5173`
   - **Backend API:** `http://localhost:8080`

---
