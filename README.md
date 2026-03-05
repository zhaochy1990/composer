# Composer

An AI agent orchestration platform that turns work items into parallelized, workflow-driven coding tasks. Config once, run on any agent.

Composer pairs a Rust backend (Axum + SQLite) with a React frontend, shipping as a single binary via rust-embed.

## Documentation

- [Product Requirements (PRD)](./FHL%20project.md) — Product vision, design principles, and feature map
- [Technical Design](./CLAUDE.md) — Architecture, crate structure, data flow, and development workflow

## Quick Start

### Prerequisites

- **Rust** (stable toolchain) — [install via rustup](https://rustup.rs/)
- **Node.js** (v18+) — [download](https://nodejs.org/)
- **pnpm** (v8+) — `npm install -g pnpm`
- **SQLite** — included via SQLx, no separate install needed
- **Playwright** (for E2E tests only) — `pnpm exec playwright install`

### Install dependencies

```bash
pnpm install
```

### Development

```bash
pnpm run dev
```

This starts both the Vite dev server (port 5173) and the Cargo watch server (port 3000) concurrently. Open [http://localhost:5173](http://localhost:5173) in your browser.

You can also run them separately:

```bash
pnpm run dev:web       # Vite dev server only (proxies /api to :3000)
pnpm run dev:server    # Cargo watch on Rust crates only
```

### Production Build

```bash
pnpm run build
```

This builds the React app into `packages/web/dist/` and compiles the Rust server into `target/release/composer-server`. The production binary embeds the SPA and serves everything from a single executable.

### Run production server

```bash
npm start
```

### Run Tests

```bash
cargo test                # Rust unit tests
pnpm run test:e2e         # Playwright E2E tests (headless)
pnpm run test:e2e:headed  # Playwright with visible browser
```

### Lint & Format

```bash
pnpm run lint      # ESLint
pnpm run format    # Prettier + cargo fmt
```

## Project Structure

```
composer/
├── crates/
│   ├── api-types/    # Shared Rust ↔ TS types (ts-rs codegen)
│   ├── db/           # SQLx + SQLite (migrations, models)
│   ├── git/          # Git worktree management for agent isolation
│   ├── executors/    # Agent process spawning & stream-JSON protocol
│   ├── services/     # Business logic (tasks, agents, sessions, workflows)
│   └── server/       # Axum HTTP server, WebSocket hub, route handlers
├── packages/
│   └── web/          # React 18 + TanStack Router/Query + shadcn/ui
├── FHL project.md    # Product requirements document
├── CLAUDE.md         # Technical design document
└── README.md
```

## Configuration

Composer uses a layered config system: **env var > `~/.composer/config.toml` > defaults**.

| Setting | Default |
|---------|---------|
| `server.port` | 3000 |
| `server.bind_address` | 127.0.0.1 |
| `database.url_pattern` | `sqlite:composer.db?mode=rwc` |
| `logging.level` | `composer=debug,tower_http=debug` |

See [Technical Design](./CLAUDE.md#configuration-cratesconfig) for full configuration details.
