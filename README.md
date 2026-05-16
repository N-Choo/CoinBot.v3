# Flear — Open Source Trading & Web3 Tools

Open source tools, bots, and architecture blueprints for trading, Web3, and DeFi. MIT licensed — use it, learn from it, break it, fix it.

---

## What's Inside

| Directory | Description |
|-----------|-------------|
| `server/` | Rust backend (Actix-Web) — Web3 auth, trading engine |
| `react/` | React 19 + TypeScript frontend |
| `system_architecture/` | Blueprints — News Trader Agent, system design docs |

---

### File System Structure

```text
.
├── docker-compose.yml         # Orchestration for Frontend & Backend
├── server/                    # Rust Backend (Actix-Web)
│   ├── src/                   # Handlers, Models, Routes, State, Transaction Logic
│   ├── Cargo.toml             # Backend dependencies
│   └── Dockerfile             # Multi-stage Rust build
├── react/                     # Frontend (Vite + TS)
│   ├── src/                   # Components, Hooks, Pages, Services
│   ├── package.json           # Frontend dependencies
│   └── Dockerfile             # Node.js development environment
└── system_architecture/       # Blueprints & architecture docs
    └── news_trader_agent_blueprint.md
```

---

### Tech Stack

| Component | Technology |
|-----------|-----------|
| **Backend** | Rust (Actix-Web), PostgreSQL (SQLx), Moka Cache |
| **Frontend** | React 19, TypeScript, Vite, Ethers v6, Axios |
| **DevOps** | Docker, Docker Compose |
| **Web3** | EIP-191 Signature Verification (ethers-rs / ethers.js) |
| **Trading Bots** | Python, yfinance, VADER, scikit-learn |

---

### Development Setup

**Prerequisites:** Docker & Docker Compose installed.

1. **Configure Environment:**
   Ensure your `.env` file exists in the root with `DATABASE_URL` and your specific API credentials.

2. **Launch System:**

```bash
   docker compose up --build
   
```

   _Note for Fedora users: The volume mounts utilize the `:Z` flag to handle SELinux permissions._

3. **Access:**
   - **Frontend:** `http://localhost:5173`
   - **Backend API:** `http://localhost:8080`

---

### Contributing

PRs are welcome. Found a bug? Open an issue. Want a feature? Build it and send a PR. This is open source — all contributions improve the project for everyone.

### License

MIT — do whatever you want. See [LICENSE](LICENSE).
