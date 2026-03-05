# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

PRD doc "FHL project.md"

## Project Overview

Composer is an AI agent orchestration platform that turns work items into parallelized, workflow-driven coding tasks. It pairs a Rust backend (Axum + SQLite) with a React frontend, built to ship as a single binary via rust-embed.

## Commands

### Development
```bash
pnpm install                # Install all dependencies
pnpm run dev                # Run both Vite dev server (port 5173) and Cargo watch concurrently
pnpm run dev:web            # Vite dev server only (port 5173, proxies /api to :3000)
pnpm run dev:server         # Cargo watch on Rust crates only
```

### Build
```bash
pnpm run build              # Build both web and server
pnpm run build:web          # Vite production build → packages/web/dist/
pnpm run build:server       # Cargo release build → target/release/composer-server
```

### Test
```bash
pnpm run test:e2e           # Playwright E2E tests (headless)
pnpm run test:e2e:ui        # Playwright with UI
pnpm run test:e2e:headed    # Playwright with visible browser
cargo test                  # Rust unit tests
```

### Lint & Format
```bash
pnpm run lint               # ESLint on web package
pnpm run format             # Prettier + cargo fmt
```

### Type Generation
```bash
pnpm run generate-types     # Export Rust types → packages/web/src/types/generated.ts
```

## Architecture

### Monorepo Structure
- **Cargo workspace** (`crates/`): 6 Rust crates
- **pnpm workspace** (`packages/`): 1 React/TypeScript package

### Rust Crates (dependency order)
1. **api-types** — Shared structs/enums with `#[derive(TS)]` for TypeScript codegen via ts-rs
2. **db** — SQLx + SQLite layer (WAL mode). Migrations in `crates/db/migrations/`
3. **git** — Git worktree management for agent isolation (creates under `.composer/worktrees/`)
4. **executors** — Spawns Claude Code CLI processes, parses stream-JSON protocol, emits events
5. **services** — Business logic: TaskService, AgentService, SessionService, WorktreeService, WorkflowEngine, EventBus
6. **server** — Axum HTTP server (port 3000), WebSocket hub, route handlers, embedded SPA serving

### Frontend (`packages/web/`)
- **React 18** with **TanStack Router** and **TanStack Query** for data fetching
- **Zustand** for client state, **shadcn/ui** (Radix + Tailwind) for components
- Path alias: `@/*` → `./src/*`
- Two main views: TaskBoard (kanban) and AgentPool (agent management)

### Data Flow
Frontend (React) → REST API (TanStack Query) → Axum routes → Services → DB + Executors + EventBus → WebSocket → Frontend

### Key Patterns
- **Rust→TS types**: Rust structs with `#[derive(TS)]` auto-generate `packages/web/src/types/generated.ts`
- **Event broadcasting**: tokio broadcast channel (EventBus) pushes `WsEvent` over WebSocket to frontend
- **Agent isolation**: Each agent session gets its own git worktree under `.composer/worktrees/`
- **Process management**: `AgentProcessManager` uses DashMap for lock-free tracking of spawned Claude Code processes
- **Single binary**: Production build embeds the React SPA into the Rust binary via rust-embed
- **Workflow engine**: Multi-step agent orchestration with human gates, automated PR review, and crash recovery

### Workflow Engine (`crates/services/src/workflow_engine.rs`)

The workflow engine orchestrates agent sessions through defined step sequences. Workflows are project-level templates; each task run creates a `workflow_run` instance.

**Step types:**
| Type | Behavior |
|------|----------|
| `plan` | Spawns a new agent session with a "plan only" prompt |
| `human_gate` | Pauses workflow, sets task to Waiting. User approves or rejects with comments |
| `implement` | Resumes the main agent session with implementation/fix prompt |
| `pr_review` | Spawns a separate review session; findings fed back to main session |
| `human_review` | Like human_gate but for PR review — rejection loops back to implement |

**Built-in "Feat-Common" workflow** (7 steps):
Plan → Review Plan → Implement & Create PR → Automated PR Review → Fix Review Findings → Human PR Review → Fix Human Comments

Rejection at human gates loops back to the preceding agent step with feedback. Steps 3-4 and 5-6 form review/fix cycles.

**Session model:** Most steps reuse a single Claude Code session via `--resume`. Only PR review creates a separate session. Worktrees are preserved throughout the workflow and cleaned up only on completion.

**Crash recovery:** On startup, `WorkflowEngine` finds orphaned running workflow runs, marks their current step as failed, pauses the run, and sets the task to Waiting. Users can resume from the UI.

**DB tables:**
- `workflows` — Definition templates (project-scoped, JSON step definitions)
- `workflow_runs` — Runtime instances (status, current_step_index, main_session_id)
- `workflow_step_outputs` — Per-step results with attempt tracking
- `tasks.workflow_run_id` — Links task to its active workflow run

**Task status derivation from workflow:**
- `backlog` → workflow not started
- `in_progress` → agent step running
- `waiting` → human gate or crash recovery
- `done` → all steps completed

### API Routes
All routes are under the Axum server at `:3000`:
- `/health` — Health check
- `/tasks` — CRUD + `/tasks/{id}/assign`, `/tasks/{id}/move`, `/tasks/{id}/sessions`, `/tasks/{id}/start-workflow`
- `/agents` — CRUD + `/agents/{id}/health`, `/agents/discover`
- `/sessions` — Create, get, resume, logs
- `/workflows` — CRUD
- `/workflow-runs` — `/{id}`, `/{id}/decision`, `/{id}/resume`, `/{id}/steps`
- `/worktrees` — List, delete
- `/ws` — WebSocket (bidirectional: WsCommand/WsEvent)

### Configuration (`crates/config/`)

Composer uses a layered configuration system. Precedence: **env var > `~/.composer/config.toml` > defaults**.

**`~/.composer/` directory structure:**
```
~/.composer/
├── config.toml          # Global user config
├── credentials.toml     # API keys (restricted permissions on Unix)
├── logs/                # Rotated server logs (when logging.log_to_file = true)
└── data/                # Runtime data (future use)
```

**Config sections & defaults:**
- `server.port` = 3000, `server.bind_address` = "127.0.0.1"
- `database.url_pattern` = "sqlite:composer.db?mode=rwc" (resolved to `~/.composer/data/composer.db` by default)
- `logging.level` = "composer=debug,tower_http=debug", `logging.log_to_file` = false
- `cors.origins` = [localhost:5173, localhost:3000 variants]


### Environment Variables
- `DATABASE_URL` — default: `sqlite:composer.db?mode=rwc`
- `CORS_ORIGINS` — default: localhost origins for ports 5173 and 3000
- `RUST_LOG` — default: `composer=debug,tower_http=debug`

## Development Workflow

When adding a new feature, the typical flow is:
1. Define types in `crates/api-types/` (with `#[derive(TS)]`)
2. Add migration in `crates/db/migrations/`
3. Add DB model methods in `crates/db/src/models/`
4. Implement service logic in `crates/services/src/`
5. Add route handlers in `crates/server/src/routes/`
6. Run `pnpm run generate-types` to sync types to frontend
7. Build React components in `packages/web/src/components/`
8. Add TanStack Query hooks in `packages/web/src/hooks/`

## E2E Tests

Located in `packages/web/e2e/tests/`. Uses Playwright with a test-specific DB (`sqlite:composer_test.db`). Tests run sequentially (1 worker). The Playwright config auto-starts both the Cargo server and Vite dev server.
