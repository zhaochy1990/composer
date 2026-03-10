# Composer

An AI agent orchestration platform that turns work items into parallelized, workflow-driven coding tasks. Config once, run on any agent.

Composer pairs a Rust backend (Axum + SQLite) with a React frontend, shipping as a single binary via rust-embed.

## Documentation

- [Product Requirements (PRD)](./FHL%20project.md) — Product vision, design principles, and feature map
- [Technical Design](./CLAUDE.md) — Architecture, crate structure, data flow, and development workflow

## Quick Start

### Prerequisites

- **Rust** (stable toolchain) — [install via rustup](https://rustup.rs/)
- **Node.js** (v20+) — [download](https://nodejs.org/)
- **pnpm** (v9+) — `npm install -g pnpm`

### Setup and run

```bash
pnpm install        # Install all dependencies (Rust crates + frontend packages)
pnpm run build      # Build the full project
pnpm start          # Start the production server
```

Open [http://localhost:3000](http://localhost:3000) in your browser.

## Contributing

### Development environment

After running `pnpm install`, start the dev servers:

```bash
pnpm run dev
```

This starts both the Vite dev server (port 5173) and the Cargo watch server (port 3000) concurrently. Open [http://localhost:5173](http://localhost:5173) in your browser. The Vite server proxies API requests to the Rust backend.

You can also run them separately:

```bash
pnpm run dev:web       # Vite dev server only (proxies /api to :3000)
pnpm run dev:server    # Cargo watch on Rust crates only
```

### Development workflow

When adding a new feature, the typical flow is:

1. Define types in `crates/api-types/` (with `#[derive(TS)]`)
2. Add migration in `crates/db/migrations/`
3. Add DB model methods in `crates/db/src/models/`
4. Implement service logic in `crates/services/src/`
5. Add route handlers in `crates/server/src/routes/`
6. Run `pnpm run generate-types` to sync Rust types to the frontend
7. Build React components in `packages/web/src/components/`
8. Add TanStack Query hooks in `packages/web/src/hooks/`

### Testing

```bash
cargo test                # Rust unit tests
pnpm run test:e2e         # Playwright E2E tests (headless)
pnpm run test:e2e:headed  # Playwright with visible browser
```

To run E2E tests, install Playwright browsers first: `pnpm exec playwright install`

### Lint & format

```bash
pnpm run lint      # ESLint
pnpm run format    # Prettier + cargo fmt
```

### Building

```bash
pnpm run build:web     # React app → packages/web/dist/
pnpm run build:server  # Rust binary → target/release/composer-server
pnpm run build         # Both of the above
```

The production binary embeds the SPA via rust-embed and serves everything from a single executable.

## Project Structure

```
composer/
├── crates/
│   ├── api-types/    # Shared Rust ↔ TS types (ts-rs codegen)
│   ├── db/           # SQLx + SQLite (migrations, models)
│   ├── git/          # Git worktree management for agent isolation
│   ├── config/       # Layered configuration system (~/.composer/)
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
