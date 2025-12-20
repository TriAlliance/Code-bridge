# Development Environment Synchronization Research 2025
**Research Date:** December 20, 2025
**Project:** Code Bridge - P2P Developer Platform
**Focus:** Cross-platform development environment synchronization (macOS ↔ Ubuntu)

---

## Executive Summary

This document provides comprehensive research on synchronizing development environments across platforms, with specific focus on P2P-compatible solutions for macOS and Ubuntu. Modern development requires seamless environment portability, and this research identifies cutting-edge tools and approaches for achieving consistent, reproducible development setups across multiple machines.

### Key Findings

1. **IDE Settings Sync** has matured significantly with VS Code's built-in Settings Sync and JetBrains' Backup and Sync plugin (2025)
2. **DevContainers** are becoming the standard for reproducible environments in 2025, with the Dev Container spec widely adopted
3. **Environment variables** are moving beyond .env files to centralized secret management (Phase, Infisical, Doppler)
4. **Language-specific tools** like uv (Python), proto (Node), and Rust's cargo are improving cross-machine synchronization
5. **Git 2.51.0** (2025) introduced native stash export/import, enabling stash synchronization across machines
6. **Workspace state sync** remains IDE-specific, with limited cross-platform solutions

---

## 1. IDE Settings Sync

### 1.1 VS Code Settings Sync

#### Built-in Settings Sync ⭐ **RECOMMENDED**

**Overview:**
VS Code's native Settings Sync lets you share configurations (settings, keyboard shortcuts, extensions) across machines using your Microsoft or GitHub account.

**Key Features:**
- Syncs settings, keybindings, UI state, snippets, and extensions
- Cross-platform compatibility (Windows, macOS, Linux)
- Platform-specific keybindings via `settingsSync.keybindingsPerPlatform`
- Extension sync with selective ignore via `settingsSync.ignoredExtensions`
- Cloud-backed with Microsoft/GitHub authentication

**Limitations:**
- Does NOT sync to remote windows (SSH, devcontainers, WSL)
- Requires internet connection
- Cloud-dependent (not P2P)

**Configuration:**
```json
// settings.json
{
  "settingsSync.keybindingsPerPlatform": false,  // Sync keybindings across platforms
  "settingsSync.ignoredExtensions": [
    "ms-vscode-remote.remote-ssh"  // Don't sync this extension
  ]
}
```

**P2P Integration Opportunity:**
Replace Microsoft/GitHub cloud sync with libp2p-based P2P sync. Store settings in content-addressed storage, use CRDT for conflict resolution.

#### Alternative: Crosside Sync

**Description:** Sync settings across VS Code and its forks (Cursor, VSCodium)

**Key Features:**
- Uses local storage (not cloud)
- Supports VS Code forks
- Profile management
- One-click sync

**GitHub:** [jinghaihan/vscode-crosside-sync](https://github.com/jinghaihan/vscode-crosside-sync)

**Use Case:** Developers using multiple VS Code variants (VS Code + Cursor)

#### Legacy: Settings Sync by Shan Khan

**Description:** GitHub Gist-based settings sync (older approach)

**Features:**
- Uses GitHub Gists for storage
- Granular control over what's synced
- Pre-dates built-in Settings Sync

**When to Use:** Older VS Code versions or need for Gist-based backup

### 1.2 JetBrains IDE Settings Sync

#### Backup and Sync Plugin ⭐ **RECOMMENDED (2025)**

**Overview:**
As of April 2025, JetBrains' recommended approach uses the Backup and Sync plugin (formerly Settings Sync), which stores data under your JetBrains Account.

**What Gets Synced:**
- IDE themes
- Keymaps
- Color schemes
- System settings
- UI settings
- Menus and toolbar settings
- Project view settings
- Editor settings
- Code completion settings
- Parameter name hints

**How to Enable:**
1. Go to **Settings** (Ctrl+Alt+S)
2. Navigate to **Backup and Sync**
3. Click **Enable Backup and Sync**
4. Choose "Get Settings from Account" (to pull from another IDE instance)

**Important Notes:**
- Settings Repository plugin is **deprecated** (still available in Marketplace but not recommended)
- Bundled and enabled by default in IntelliJ IDEA
- Cloud-based (requires JetBrains Account)

**Recent Updates (IDE Services 2025.4):**
- Improved plugin sync behavior in private repositories
- Plugins are now synced via UI/API/scheduled cron (not on startup)
- Automatic cleanup during synchronization

**P2P Enhancement Opportunity:**
Create P2P sync layer using libp2p to replace JetBrains cloud dependency. Store settings in encrypted CRDT-based storage.

### 1.3 Neovim/Vim Configuration Sync

#### Dotfiles Management Approaches

**GNU Stow ⭐ RECOMMENDED for Modularity**

**Description:** Symlink farm manager for dotfiles

**Key Features:**
- Creates symlinks from central dotfiles directory
- Modular: each tool gets its own package
- Transparent: see exactly what's linked where
- Reversible: `stow -D` removes all symlinks
- Cross-platform (Mac, Linux, Windows)

**Setup:**
```bash
# Structure
~/dotfiles/
├── nvim/
│   └── .config/
│       └── nvim/
│           ├── init.lua
│           └── lua/
│               └── plugins/
└── zsh/
    └── .zshrc

# Usage
cd ~/dotfiles
stow nvim  # Creates ~/.config/nvim symlink
stow zsh   # Creates ~/.zshrc symlink
stow -D nvim  # Removes nvim symlinks
```

**Cross-Platform Considerations:**
Use `overrides.json` for OS-specific differences:
- macOS: `~/.config/nvim`
- Linux: `~/.config/nvim`
- Windows: `%LOCALAPPDATA%/nvim/init.vim`

**Chezmoi** (Alternative with Templating)

**Description:** Powerful dotfile manager with OS-specific templating

**Key Features:**
- Template engine for machine-specific differences
- Secret management (1Password, Bitwarden, pass integration)
- Encryption for sensitive files
- Automatic OS/hostname detection

**Template Example:**
```bash
# .bashrc.tmpl
export EDITOR={{ if eq .chezmoi.os "darwin" }}code{{ else }}vim{{ end }}

{{ if eq .chezmoi.hostname "work-laptop" }}
export WORK_ENV=true
{{ end }}
```

**Nix + Home Manager** (Declarative Approach)

**Description:** Declarative, reproducible system configuration

**Key Features:**
- Fully declarative configuration
- Reproducible across machines
- Atomic updates and rollbacks
- Language-agnostic package management

**Use Case:** Teams wanting fully reproducible environments with version pinning

#### Popular Neovim Distributions

**NvChad:** "Blazing fast Neovim framework providing solid defaults and a beautiful UI"

**AstroNvim:** "An aesthetic and feature-rich neovim config that is extensible and easy to use"

**LazyVim:** Uses lazy.nvim plugin manager with organized plugin files

### 1.4 Cross-IDE Setting Translation

#### IdeaVim (Vim in JetBrains)

**Description:** Vim engine for all JetBrains IDEs

**Key Features:**
- Supports normal, insert, visual modes
- Command-line and Ex modes
- Vim regexp and configuration
- Conditional configuration for IDE-specific settings

**Configuration Sharing:**
```vim
" ~/.ideavimrc
source ~/.vimrc  " Include your vim config

" IdeaVim-specific settings
if has('ide')
  " Mappings that only work in IdeaVim
  map <leader>r :action ReformatCode<CR>
endif

" IDE-specific conditionals
if &ide =~? 'intellij idea'
  " IntelliJ-specific settings
elseif &ide =~? 'pycharm'
  " PyCharm-specific settings
endif
```

**Conflict Resolution:**
Configure in **Settings → Editor → Vim** to choose between IDE shortcuts and Vim bindings.

#### JetBrains Keybindings in VS Code

**Extensions:**
- **IntelliJ IDEA Keybindings:** Port of IntelliJ keybindings for VS Code
- **JetBrains IDE Keymap:** Alternative keybinding port

**Use Case:** Switching between JetBrains and VS Code while maintaining muscle memory

#### VS Code to JetBrains Migration

**IntelliJ IDEA Import:**
- Go to **File → Manage IDE Settings → Import Settings**
- Select VS Code
- IntelliJ imports keymap, UI theme, extensions, and recent projects

#### jetbrains-vscode Converter

**Description:** Convert JetBrains Run/Debug configurations to VS Code

**GitHub:** [topscoder/jetbrains-vscode](https://github.com/topscoder/jetbrains-vscode)

**Usage:**
- Finds JetBrains workspace config file (`workspace.xml`)
- Converts all Run/Debug configurations to VS Code format
- Replaces existing VS Code configurations

---

## 2. Project Environment Sync

### 2.1 Package Version Synchronization

#### Node.js - proto ⭐ **NEW RECOMMENDATION (2025)**

**Description:** Modern, unified tool version manager

**Key Features:**
- Manages Node, npm, pnpm, yarn versions
- Creates `.prototools` file for version locking
- Automatic tool version detection per directory
- Faster than nvm (written in Rust)
- Cross-platform (macOS, Linux, Windows)

**Setup:**
```toml
# .prototools
node = "20.10.0"
pnpm = "8.12.0"
```

**Automatic Activation:**
```bash
# Every time you cd into project directory, proto auto-activates versions
cd my-project  # Automatically switches to Node 20.10.0
node --version  # v20.10.0
```

**Why Better than nvm:**
- Single config file (`.prototools`) vs. `.nvmrc` + `.npmrc`
- Manages multiple tools (not just Node)
- Committed to repo for team consistency
- Faster tool switching

#### Node.js - nvm (Traditional Approach)

**Description:** Node Version Manager (POSIX-compliant bash script)

**Key Features:**
- `.nvmrc` file for version locking
- LTS version support (`lts/*`, `lts/argon`)
- Per-directory version switching
- Shell integration for auto-switching

**Setup:**
```bash
# .nvmrc
20.10.0

# Auto-switch on cd (add to ~/.bashrc or ~/.zshrc)
autoload -U add-zsh-hook
load-nvmrc() {
  if [[ -f .nvmrc && -r .nvmrc ]]; then
    nvm use
  fi
}
add-zsh-hook chpwd load-nvmrc
```

**CI/CD Integration:**
```yaml
# GitHub Actions
- uses: actions/setup-node@v4
  with:
    node-version-file: '.nvmrc'
```

**Alternatives:**
- **fnm:** Written in Rust, much faster than nvm
- **volta:** LinkedIn's tool, handles Node + package managers
- **asdf:** Multi-language version manager (Node, Ruby, Python, etc.)

#### Python - uv ⭐ **REVOLUTIONARY (2025)**

**Description:** Rust-based, unified Python package and environment manager

**Key Features:**
- Complete replacement for pip, venv, pyenv, poetry
- Deterministic lockfile-based synchronization (`uv.lock`)
- 10-100x faster than pip
- Handles interpreter installation, dependency management, tool execution
- Cross-platform consistency

**Why Game-Changing:**
- **One tool** replaces pip, virtualenv, pyenv, poetry, pipx
- **Cargo-like** for Python: fast, deterministic, self-contained
- **Lockfile** ensures exact environment replication

**Setup:**
```bash
# Install uv
curl -LsSf https://astral.sh/uv/install.sh | sh

# Create project
uv init my-project
cd my-project

# Install dependencies
uv add requests pandas

# Lock dependencies (creates uv.lock)
uv lock

# Sync environment from lockfile
uv sync
```

**Cross-Machine Workflow:**
```bash
# Machine A
uv lock  # Creates uv.lock
git add uv.lock
git commit -m "Lock dependencies"
git push

# Machine B
git pull
uv sync  # Installs exact same versions
```

**Migration from Poetry:**
```bash
# uv can read poetry.lock
uv sync  # Works with existing poetry projects
```

#### Python - Poetry (Established)

**Description:** Dependency management and packaging

**Key Features:**
- `poetry.lock` for version locking
- Cross-platform lockfile
- Dependency resolution
- Virtual environment management

**Setup:**
```bash
# Install dependencies from lock
poetry install

# Sync (remove unnecessary packages)
poetry install --sync

# Update one package
poetry update requests
```

**Cross-Platform Note:**
Poetry lockfile works cross-platform, though technically it resolves for specific interpreter/environment.

#### Rust - Cargo

**Description:** Rust package manager and build system

**Key Features:**
- `Cargo.lock` for dependency locking
- Automatic lock file generation
- Cross-platform by design
- Workspace support for monorepos

**Configuration:**
```toml
# Cargo.toml
[package]
name = "my-project"
version = "0.1.0"

[dependencies]
serde = "1.0"
tokio = { version = "1.35", features = ["full"] }
```

**Sync Across Machines:**
```bash
# Cargo.lock is committed to version control
git clone repo
cd repo
cargo build  # Uses exact versions from Cargo.lock
```

**Important:** Cargo and Rustup configuration paths:
- Rustup home: `~/.rustup` (or `$RUSTUP_HOME`)
- Cargo home: `~/.cargo` (or `$CARGO_HOME`)
- Both can be synced via dotfiles, but environment variables take precedence

### 2.2 Environment Variables & Secrets

#### Phase ⭐ **RECOMMENDED (2025)**

**Description:** Modern secret management platform with E2E encryption

**Website:** [phase.dev](https://phase.dev/)

**Key Features:**
- Seamless secret injection as environment variables
- Git-styled diffs for tracking secret changes
- Point-in-time recovery
- Dynamic secrets (short-lived credentials on-demand)
- No code changes or dependencies required
- Cross-platform CLI

**Use Case:** Teams needing secure, version-controlled secrets with recovery

#### Infisical (Open Source)

**Description:** Free, open-source secret management

**Key Features:**
- Self-hosted or cloud
- Environment variable synchronization
- Flexible APIs
- Better than EnvKey for self-hosting
- Team collaboration features

**Setup:**
```bash
# Install CLI
brew install infisical/get-cli/infisical

# Login
infisical login

# Run app with secrets
infisical run -- npm start
```

**P2P Opportunity:** Self-hosted Infisical could use P2P sync via libp2p for distributed teams

#### Doppler (Cloud-Based)

**Description:** Cloud secret management with excellent DX

**Key Features:**
- Real-time secret synchronization
- CLI for local injection
- Built-in secret sharing for teams
- No .env file management

**Setup:**
```bash
# Install
brew install dopplerhq/cli/doppler

# Run with secrets
doppler run -- npm start
```

#### 1Password Developer Tools

**Description:** Developer-focused secret management

**Key Features:**
- **Secrets Automation API:** REST API for accessing 1Password items
- **SSH Agent:** Built-in SSH key management
- **CLI:** `op` command-line tool
- **SDK:** Go, JavaScript, Python SDKs

**SSH Key Management:**
- Generate/import SSH keys in 1Password
- Keys stored encrypted (never plaintext on disk)
- Built-in SSH agent (keys never leave 1Password app)
- Share public keys via browser extension
- Sync keys across devices

**Recent Updates (Jan 2025):**
- Create item sharing links via API
- Retrieve SSH key metadata (public key, type, fingerprint)
- Archive items programmatically

**Security Features:**
- Developer Watchtower: scans `~/.ssh` for exposed private keys
- Warns about credentials on disk

**Use Case:** Individual developers and teams needing secure credential sync

#### env-sync (Laravel)

**Description:** Laravel package for environment variable sync

**Features:**
- 1Password integration
- AWS Secrets Manager support
- Bidirectional sync
- Multi-environment support

**P2P Alternative:** Could build similar tool using Code Bridge's P2P infrastructure

#### Why Move Beyond .env Files

**Security Risks:**
- **GhostAction Attack (Sept 2025):** 327 GitHub users, 817 repositories, 3,325 secrets exfiltrated
- .env files often committed to Git accidentally
- No audit trail
- Difficult to rotate
- Duplication across projects

**Scaling Issues:**
- Manual sync across environments
- Team members forget to share updates
- Infrastructure misconfiguration
- Downtime from missing variables

**Modern Principles:**
- **Principle of Least Privilege:** Need-to-know basis
- **Regular Audits:** Monitor access patterns
- **Rotation Policies:** Automatic credential rotation
- **Centralized Storage:** Single source of truth

### 2.3 Docker Configuration Sync

#### DevContainers ⭐ **BECOMING STANDARD (2025)**

**Description:** Standardized development containers

**Key Features:**
- `.devcontainer.json` configuration file
- Docker or Podman support
- Dev Container Features (modular tools/runtimes)
- CI/CD integration (GitHub Actions, Azure DevOps)
- IDE support (VS Code, JetBrains, Cursor)

**Status in 2025:**
- Rapidly becoming standard practice
- Reduces "Works on My Machine" syndrome
- Identical environments across all platforms

**Basic Setup:**
```json
// .devcontainer/devcontainer.json
{
  "name": "Node.js & TypeScript",
  "image": "mcr.microsoft.com/devcontainers/typescript-node:20",
  "features": {
    "ghcr.io/devcontainers/features/github-cli:1": {}
  },
  "postCreateCommand": "npm install",
  "customizations": {
    "vscode": {
      "extensions": [
        "dbaeumer.vscode-eslint"
      ]
    }
  }
}
```

**Docker Compose Integration:**
```json
// .devcontainer/devcontainer.json
{
  "name": "Multi-Service App",
  "dockerComposeFile": [
    "../docker-compose.yml",
    "docker-compose.dev.yml"
  ],
  "service": "app",
  "workspaceFolder": "/workspace"
}
```

**Compose Watch (File Sync):**
```yaml
# compose.yaml
services:
  web:
    build: .
    develop:
      watch:
        - action: sync
          path: ./src
          target: /app/src
        - action: rebuild
          path: package.json
```

**CI/CD Integration:**
```yaml
# GitHub Actions
- uses: devcontainers/ci@v0.3
  with:
    runCmd: npm test
```

**Benefits:**
- Start locally, scale to Codespaces with no changes
- Version-controlled environment
- Team consistency
- Faster onboarding (5-10 min vs 10-15 min)

### 2.4 Local Development Certificates

#### mkcert ⭐ **RECOMMENDED**

**Description:** Zero-config tool for locally-trusted SSL certificates

**GitHub:** [FiloSottile/mkcert](https://github.com/FiloSottile/mkcert)

**Key Features:**
- Creates local Certificate Authority (CA)
- Installs CA in system trust stores (macOS, Windows, Linux, browsers)
- Generates certificates for any hostname/IP
- No configuration required

**Setup:**
```bash
# Install
brew install mkcert  # macOS
sudo apt install mkcert  # Ubuntu

# Install local CA
mkcert -install

# Generate certificate
mkcert example.com localhost 127.0.0.1 ::1

# Output: example.com.pem and example.com-key.pem
```

**Syncing Across Multiple Machines:**

**Export CA Certificate:**
```bash
# Machine A
mkcert -CAROOT  # Shows CA storage location
# Copy CA certificate (NOT the key!) to Machine B

# Machine B
mkcert -install
# Uses the shared CA certificate
```

**IMPORTANT SECURITY WARNING:**
- **DO NOT share `rootCA-key.pem`** - Gives complete MITM power
- Only share `rootCA.pem` (certificate, not key)
- Use for development only, never production

**Use Cases:**
- Multiple dev machines (laptop + desktop)
- Docker containers
- Team development (share CA cert for consistent HTTPS)

**Environment Variable:**
```bash
# Use $CAROOT to manage multiple CAs
export CAROOT=/path/to/custom/ca
mkcert -install  # Uses custom CA location
```

### 2.5 Database Seeds and Migrations

#### Best Practices for Consistency

**Problem:**
Teams work with different data sets:
- Local DB ≠ Staging ≠ Production
- "Works on My Machine" syndrome

**Solution:** Reproducible seeding and migrations

#### Supabase Approach ⭐ **RECOMMENDED**

**Workflow:**
1. All database changes captured in code (migrations)
2. Reset to known state anytime with seed data
3. Dashboard or SQL for schema changes
4. CLI to diff and create migrations

**Seeding:**
```sql
-- supabase/seed.sql
INSERT INTO users (id, name, email) VALUES
  (1, 'Alice', 'alice@example.com'),
  (2, 'Bob', 'bob@example.com');

INSERT INTO posts (id, user_id, title) VALUES
  (1, 1, 'First Post'),
  (2, 2, 'Second Post');
```

**Best Practices:**
- Only data insertions in seed files (no schema changes)
- Seeds run after migrations
- Idempotent seeds (safe to run multiple times)

#### Entity Framework Core (Microsoft)

**Key Features:**
- `UseSeeding` and `UseAsyncSeeding` for initial data
- Migrations compute insert/update/delete operations
- Automatic schema evolution

**Setup:**
```csharp
// Seeding configuration
modelBuilder.Entity<User>().HasData(
    new User { Id = 1, Name = "Alice" },
    new User { Id = 2, Name = "Bob" }
);
```

#### Prisma ORM v7 Changes

**Breaking Change (2025):**
- Seeding only triggered by `npx prisma db seed`
- No automatic seeding during `prisma migrate dev/reset`

**Seeding Script:**
```typescript
// prisma/seed.ts
import { PrismaClient } from '@prisma/client'
const prisma = new PrismaClient()

async function main() {
  await prisma.user.createMany({
    data: [
      { name: 'Alice', email: 'alice@example.com' },
      { name: 'Bob', email: 'bob@example.com' }
    ]
  })
}

main()
  .then(async () => await prisma.$disconnect())
  .catch(async (e) => {
    console.error(e)
    await prisma.$disconnect()
    process.exit(1)
  })
```

#### GitHub Actions Automation (May 2025)

**Workflow:** Combines automation with safety
- Run migrations in staging first
- Require approval for production
- Automatic rollback on failure
- Audit trail for compliance

**Ephemeral Environments:**
Data seeding involves running a job alongside services to populate the database with usable data. Great for PR previews and testing.

---

## 3. Workspace State Sync

### 3.1 Open Files and Cursor Position

#### Challenges

Most IDEs don't natively sync workspace state across machines:
- Open files
- Cursor positions
- Scroll positions
- Editor layout (split views)

#### Eclipse Che (Workspace State Issue)

**Open Issue:** On reload, workspace should remember:
- Project tree state
- Open files with cursor position
- Panel states and sizes
- Open terminals

**Status:** Feature request, not yet implemented

#### IDESync-VSCode-JetBrains

**Description:** Sync file and cursor position between VS Code and JetBrains

**GitHub:** [denisbalber/IDESync-VSCode-JetBrains](https://github.com/denisbalber/IDESync-VSCode-JetBrains)

**Features:**
- Maintain current file and cursor position when switching IDEs
- Available in VS Code Marketplace

**Use Case:** Developers who switch between VS Code and JetBrains IDEs

#### Cursor IDE Settings

**Note:** Cursor (AI-powered IDE) stores settings differently from VS Code:
- Uses `state.vscdb` file
- Supports `persistedDirectories` in `environment.json`
- Each remote-SSH window gets disposable Ubuntu box
- Team-wide identical workspaces

**Backup Recommendation:** Create backup of `state.vscdb` before changes

### 3.2 Git Stash Synchronization

#### Git 2.51.0 (2025) - Native Stash Export/Import ⭐ **NEW**

**Game-Changer:** Git now supports stash export and import!

**Commands:**
```bash
# Export stash to ref
git stash export --to-ref <ref> [<stash>…​]

# Import from commit
git stash import <commit>
```

**Modern Workflow for Syncing Stashes:**

**On Machine A:**
```bash
# Create stash with message
git stash push --include-untracked --message "WIP: feature X"

# Export to remote ref
git stash export --to-ref "refs/stashes/$USER"

# Push to remote
git push --no-verify --force origin "refs/stashes/$USER"
```

**On Machine B:**
```bash
# Configure to fetch stashes
git config set --global remote.origin.fetch \
  'refs/stashes/*:refs/remote/origin/stashes/*'

# Fetch stashes
git fetch

# Import stash
git stash import refs/remote/origin/stashes/$USER
```

**Recommended Configuration:**
```bash
# Show untracked files in stash
git config set --global stash.showIncludeUntracked true

# Show stash as patch
git config set --global stash.showPatch true
```

#### Legacy: Patch-Based Transfer (Pre-2.51.0)

**For Older Git Versions:**

**Machine A:**
```bash
git stash show -p > changes.patch
# Transfer changes.patch to Machine B via email/USB/cloud
```

**Machine B:**
```bash
git apply changes.patch
```

### 3.3 Debug Breakpoints Synchronization

#### JetBrains

**Breakpoint Persistence:**
- All breakpoints saved automatically
- Persist across IDE restarts
- Stored in `.idea/workspace.xml`

**Advanced Features:**
- Non-suspending breakpoints (for logging)
- Conditional breakpoints
- Trigger breakpoints (enable dependent breakpoints)

**Sync:**
Breakpoints stored in workspace.xml can be committed to version control, but this is generally **not recommended** (workspace.xml contains local paths and personal preferences).

#### VS Code

**Breakpoint Storage:**
- Stored in `.vscode/launch.json` (launch configurations)
- Breakpoints themselves stored in workspace state (not synced by default)

**Debug Configuration:**
```json
// .vscode/launch.json
{
  "version": "0.2.0",
  "configurations": [
    {
      "type": "node",
      "request": "launch",
      "name": "Launch Program",
      "program": "${workspaceFolder}/app.js"
    }
  ]
}
```

**Sync:** Launch configurations can be committed to repo, but breakpoint positions are workspace-specific.

#### jetbrains-vscode Converter

**Description:** Convert JetBrains Run/Debug configurations to VS Code

**GitHub:** [topscoder/jetbrains-vscode](https://github.com/topscoder/jetbrains-vscode)

**Process:**
- Reads `workspace.xml` from JetBrains IDE
- Converts to VS Code `launch.json`
- Replaces existing VS Code configurations

**Use Case:** Migrating from JetBrains to VS Code with existing debug configurations

### 3.4 TODO/Task List Sync

#### VS Code Extensions

**Todo Tree ⭐ RECOMMENDED**

**Features:**
- Scans codebase for TODO, FIXME, etc.
- Tree view in activity bar
- Click to navigate
- Highlighting

**Configuration:**
```json
{
  "todo-tree.general.tags": [
    "TODO",
    "FIXME",
    "HACK",
    "NOTE",
    "[ ]",  // Markdown checkboxes
    "[x]"
  ],
  "todo-tree.highlights.customHighlight": {
    "TODO": {
      "foreground": "yellow"
    },
    "FIXME": {
      "foreground": "red"
    }
  }
}
```

**Todo+ (Advanced)**

**Features:**
- TaskPaper compatibility
- Timekeeping (mark as started, track elapsed time)
- Embedded todos (find in code comments)
- Activity bar view

**Markdown Example:**
```markdown
Project:
  ✔ Completed task @done(2025-12-20 10:30)
  ☐ Pending task @started(2025-12-20 10:00)
  ☐ High priority @high
  ☐ Due soon @due(2025-12-25)
```

**Todo MD (todo.txt format)**

**Features:**
- Based on todo.txt format
- Recurring tasks
- Due dates
- Webview

**Use Case:** Teams using todo.txt methodology

#### Visual Studio (Full IDE)

**Task List:**
- Tracks code comments with tokens (TODO, HACK, UNDONE, etc.)
- Case-insensitive
- Customizable tokens
- Direct navigation to code

**Tokens:**
```csharp
// TODO: Implement error handling
// HACK: Temporary fix
// UNDONE: Incomplete feature
```

#### Sync Approach

**Problem:** TODO comments are in code, but external task lists (todo.md) are not.

**Solutions:**
1. **Commit TODO comments** to version control (automatically synced)
2. **External task tools:**
   - **Todoist** (cloud sync)
   - **Microsoft To Do** (cloud sync)
   - **Markdown files** (synced via git or P2P)
3. **P2P TODO Sync:** Store todo.md in Code Bridge project, sync via P2P

---

## 4. Development Containers

### 4.1 DevContainer Standard ⭐ **BECOMING STANDARD (2025)**

**Official Spec:** [containers.dev](https://containers.dev/)

**Description:** Development Containers Specification for full-featured development environments

**Maintained By:** devcontainers community

**Status in 2025:**
Rapidly becoming essential in modern DevOps. DevContainers are standard practice among developers working solo or in teams.

**Key Components:**
1. **Container Runtime:** Docker or Podman
2. **devcontainer.json:** Primary configuration file
3. **Dev Container Features:** Modular, shareable units (tools, runtimes, libraries)

**Architecture:**
```
┌─────────────────────────────────────────────────┐
│ IDE (VS Code, Cursor, JetBrains, GitHub        │
│ Codespaces)                                     │
├─────────────────────────────────────────────────┤
│ Dev Container Specification                     │
├─────────────────────────────────────────────────┤
│ devcontainer.json                               │
│ - Features                                      │
│ - Extensions                                    │
│ - Settings                                      │
├─────────────────────────────────────────────────┤
│ Docker / Podman                                 │
├─────────────────────────────────────────────────┤
│ Host OS (macOS, Linux, Windows)                │
└─────────────────────────────────────────────────┘
```

**Example Configuration:**
```json
{
  "name": "Python 3",
  "image": "mcr.microsoft.com/devcontainers/python:3.11",
  "features": {
    "ghcr.io/devcontainers/features/github-cli:1": {},
    "ghcr.io/devcontainers/features/docker-in-docker:2": {}
  },
  "customizations": {
    "vscode": {
      "extensions": [
        "ms-python.python",
        "ms-python.vscode-pylance"
      ],
      "settings": {
        "python.defaultInterpreterPath": "/usr/local/bin/python"
      }
    }
  },
  "postCreateCommand": "pip install -r requirements.txt",
  "remoteUser": "vscode"
}
```

**Benefits:**
- **Identical environments** regardless of host OS
- **Start locally** with Docker, scale to Codespaces with no config changes
- **Version-controlled** environment definition
- **Portable** across tools (VS Code, Codespaces, JetBrains)

**CI/CD Integration:**
```yaml
# GitHub Actions
- uses: devcontainers/ci@v0.3
  with:
    runCmd: npm test
```

### 4.2 GitHub Codespaces Alternatives

#### Comparison Table

| Tool | Type | Cost | Self-Host | Features |
|------|------|------|-----------|----------|
| **DevPod** | Client-only | Free | ✅ Yes | Multi-cloud, devcontainer.json, IDE-agnostic |
| **Daytona** | Self-hosted | Free/Paid | ✅ Yes | Multi-provider, SDK, AI-focused |
| **Coder** | Self-hosted | Free/Paid | ✅ Yes | Terraform-based, enterprise features |
| **Gitpod (Ona)** | Cloud | Paid | ❌ No | Ephemeral, Kubernetes-based |
| **Northflank** | Cloud | Paid | ✅ BYOC | MicroVM isolation, full cloud platform |
| **GitHub Codespaces** | Cloud | Paid | ❌ No | GitHub integration, standardized |

#### DevPod ⭐ **RECOMMENDED for Open Source**

**Description:** Open-source, client-only tool for reproducible dev environments

**Key Features:**
- Based on devcontainer.json standard
- Works with any IDE (VS Code, JetBrains)
- Deploy anywhere (cloud, Kubernetes, locally)
- Prebuilds for faster startup
- Auto-inactivity shutdown (cost savings)
- No vendor lock-in

**Why Choose DevPod:**
- Fully open source
- Switch between DevPod and Codespaces without modifications
- Client-only (no server to maintain)
- Cost-effective

#### Daytona

**Description:** Self-hosted development environment manager

**Key Features:**
- Multi-provider support (AWS, Azure, GCP)
- IDE support (VS Code, JetBrains)
- Free SDK for programmatic control
- Enterprise-grade

**Recent Pivot:** Focus shifting to AI features

#### Coder

**Description:** Self-hosted dev environments defined as Terraform

**Key Features:**
- Infrastructure as Code (Terraform)
- Granular access control
- Scalable for large teams
- Deploy on any cloud or on-premises

**Use Case:** Enterprises needing fine-grained control

#### Gitpod (now Ona)

**Description:** Ephemeral development environments

**Key Features:**
- Kubernetes-based
- Uses `gitpod.yml` file
- Integration with GitLab, Bitbucket
- Cloud-only (self-hosted option removed)

**Note:** Now rebranded as Ona

### 4.3 P2P Development Environment Sync

**Opportunity for Code Bridge:**

DevContainers are files (Dockerfile, devcontainer.json, docker-compose.yml). These can be:
1. **Stored** in Code Bridge's content-addressed storage
2. **Synced** via P2P to all team members
3. **Versioned** using CRDT for conflict-free updates
4. **Shared** without relying on Docker Hub or GitHub Container Registry

**Architecture:**
```
┌─────────────────────────────────────────────────┐
│ Code Bridge Dev Environment Sync                │
├─────────────────────────────────────────────────┤
│                                                  │
│  Developer A          P2P Sync          Developer B
│  ┌─────────────┐                     ┌─────────────┐
│  │ devcontainer│ ◄─────────────────► │ devcontainer│
│  │   .json     │                     │   .json     │
│  ├─────────────┤                     ├─────────────┤
│  │ Dockerfile  │ ◄─────────────────► │ Dockerfile  │
│  ├─────────────┤                     ├─────────────┤
│  │ docker-     │ ◄─────────────────► │ docker-     │
│  │ compose.yml │                     │ compose.yml │
│  └─────────────┘                     └─────────────┘
│                                                  │
│  Content-Addressed Storage                      │
│  CID: Qm...abc (devcontainer config v1)         │
│  CID: Qm...def (devcontainer config v2)         │
│                                                  │
└─────────────────────────────────────────────────┘
```

**Features:**
- **P2P container image sharing:** Pull images from peers instead of Docker Hub
- **Instant team updates:** Change devcontainer.json, sync to all team members
- **Offline-capable:** Work without internet, sync when reconnected
- **Version history:** CRDT-based versioning of environment configs

---

## 5. Language-Specific Tools

### 5.1 Rust - Cargo & Rustup

#### Cargo Configuration

**Config File Locations:**
- Global: `~/.cargo/config.toml`
- Project: `<project>/.cargo/config.toml`

**Key Paths:**
- Rustup home: `~/.rustup` (or `$RUSTUP_HOME`)
- Cargo home: `~/.cargo` (or `$CARGO_HOME`)

**Environment Variables:**
Environment variables take precedence over TOML config. For key `foo.bar`, use `CARGO_FOO_BAR`.

**Merging:**
Multiple config files are merged, with closest to current directory taking precedence. `$HOME/.cargo/config.toml` has lowest precedence.

**Sync Approach:**
- Commit `Cargo.lock` to version control
- Sync `~/.cargo/config.toml` via dotfiles
- Team members get exact dependency versions

**Cargo.toml Example:**
```toml
[dependencies]
serde = { version = "1.0", features = ["derive"] }
tokio = { version = "1.35", features = ["full"] }

[profile.release]
opt-level = 3
lto = true
```

**Cross-Platform Note:**
Cargo.lock works identically across platforms. Cargo is cross-platform by design.

#### Rustup Coordination with Cargo

**Issue:** Rustup and Cargo historically disagreed on CARGO_HOME definition.

**Current Status (2025):**
- Rustup sets CARGO_HOME to ensure agreement
- Likely to be changed: Rustup may stop setting CARGO_HOME
- Path coordination: considering using same paths or separate paths

**Best Practice:**
Let Rustup manage toolchains, let Cargo manage packages. Sync both via environment variables if needed.

### 5.2 Node.js - Version Management

#### proto ⭐ **RECOMMENDED (2025)**

**Description:** Modern, unified tool version manager

**Why Better:**
- Manages Node, npm, pnpm, yarn, Bun
- Single config file (`.prototools`)
- Written in Rust (fast)
- Automatic version switching

**Setup:**
```bash
# Install proto
curl -fsSL https://moonrepo.dev/install/proto.sh | bash

# Create .prototools
cat > .prototools << EOF
node = "20.10.0"
pnpm = "8.12.0"
EOF

# Commit to repo
git add .prototools
git commit -m "Pin tool versions"
```

**Team Usage:**
```bash
# Clone repo on new machine
git clone repo
cd repo

# proto automatically uses versions from .prototools
node --version  # v20.10.0 (auto-installed if needed)
pnpm --version  # 8.12.0
```

#### nvm (Traditional)

**Description:** Node Version Manager

**Setup:**
```bash
# Install nvm
curl -o- https://raw.githubusercontent.com/nvm-sh/nvm/v0.40.0/install.sh | bash

# Create .nvmrc
echo "20.10.0" > .nvmrc

# Use version
nvm use

# Auto-switch on cd (add to ~/.zshrc)
autoload -U add-zsh-hook
load-nvmrc() {
  if [[ -f .nvmrc && -r .nvmrc ]]; then
    nvm use
  fi
}
add-zsh-hook chpwd load-nvmrc
```

**CI/CD:**
```yaml
# GitHub Actions
- uses: actions/setup-node@v4
  with:
    node-version-file: '.nvmrc'
```

**Limitations:**
- Bash script (slower than Rust-based alternatives)
- Only manages Node (not package managers)
- Manual integration for auto-switching

#### Alternatives

**fnm (Fast Node Manager):**
- Written in Rust
- Much faster than nvm
- Cross-platform

**volta:**
- LinkedIn's tool
- Manages Node + package managers
- Pinning per project

**asdf:**
- Multi-language version manager
- Manages Node, Ruby, Python, etc.
- Plugin-based

### 5.3 Python - Environment Management

#### uv ⭐ **RECOMMENDED (2025)**

**Description:** Revolutionary Python package manager (Rust-based)

**Why Game-Changing:**
- Replaces pip, venv, pyenv, poetry, pipx
- 10-100x faster than pip
- Lockfile-based determinism
- Cross-platform consistency

**Setup:**
```bash
# Install uv
curl -LsSf https://astral.sh/uv/install.sh | sh

# Create project
uv init my-project
cd my-project

# Add dependencies
uv add requests pandas numpy

# Lock dependencies
uv lock  # Creates uv.lock

# Commit lockfile
git add uv.lock pyproject.toml
git commit -m "Lock dependencies"
```

**Cross-Machine Workflow:**
```bash
# Machine B
git pull
uv sync  # Installs exact versions from uv.lock
```

**Benefits:**
- **Determinism:** Exact same environment every time
- **Speed:** 10-100x faster than pip
- **Consolidation:** One tool for everything
- **Consistency:** Works identically on macOS, Linux, Windows (via WSL)

**Migration:**
```bash
# Works with existing poetry projects
uv sync  # Reads poetry.lock
```

#### pyenv + pyenv-virtualenv (Traditional)

**Description:** Python version management

**Cross-Platform Limitation:**
- **Does NOT work on Windows** (except WSL)
- UNIX-like systems only (macOS, Linux)

**Setup:**
```bash
# Install pyenv
curl https://pyenv.run | bash

# Install Python version
pyenv install 3.11.7

# Set local version
pyenv local 3.11.7

# Create virtualenv
pyenv virtualenv 3.11.7 my-project-env
pyenv local my-project-env
```

**Sync Approach:**
- Create `.python-version` file (commit to repo)
- Team members run `pyenv install $(cat .python-version)`
- Virtualenvs are NOT synced (recreated per machine)

**Modern Alternative:**
For cross-platform work, use **uv** or **micromamba**.

#### micromamba (For Non-Python Dependencies)

**Description:** Fast conda alternative

**When to Use:**
- Projects with non-Python dependencies (C libraries, etc.)
- Data science workflows
- Cross-platform including Windows

**Setup:**
```bash
# Install micromamba
"${SHELL}" <(curl -L micro.mamba.pm/install.sh)

# Create environment
micromamba create -f environment.yml

# Activate
micromamba activate myenv
```

**environment.yml:**
```yaml
name: myenv
channels:
  - conda-forge
dependencies:
  - python=3.11
  - numpy
  - pandas
  - scipy
```

**Recommendation:**
- **uv:** For pure Python projects (fast, simple)
- **micromamba:** For projects needing non-Python dependencies

### 5.4 Swift/Xcode - Version Management

#### xcodes ⭐ **RECOMMENDED**

**Description:** Best command-line tool for Xcode version management

**GitHub:** [XcodesOrg/xcodes](https://github.com/XcodesOrg/xcodes)

**Features:**
- Install and switch between Xcode versions
- GUI app version available (Xcodes.app)
- Automatic download and extraction
- Saves time vs manual DMG/XIP

**Setup:**
```bash
# Install
brew install xcodes

# Install Xcode version
xcodes install 15.2
xcodes install "16 Beta 3"
xcodes install --latest-prerelease

# Select Xcode
xcodes select 15.2

# Or use xcode-select
sudo xcode-select --switch /Applications/Xcode-15.2.app
```

**Team Sync:**
Create `.xcode-version` file:
```
15.2
```

Commit to repo. CI/CD can read this file.

#### swiftenv (Swift Version Manager)

**Description:** Manage multiple Swift versions

**Features:**
- Install Swift toolchains
- Switch versions locally or globally
- `.swift-version` file for project pinning

**Setup:**
```bash
# Install swiftenv
brew install swiftenv

# Install Swift version
swiftenv install 5.9

# Set local version
swiftenv local 5.9  # Creates .swift-version
```

**Important Note:**
- swiftenv manages **open-source Swift toolchains**
- Separate from Xcode-bundled Swift
- Use xcodes for Xcode versions, swiftenv for standalone Swift

**Xcode-Swift Relationship:**
Apple maintains tight coupling. Each Xcode release includes specific Swift version. Developers often need multiple Xcode versions simultaneously.

**Latest (2025):**
Xcode 26 announced June 9, 2025, released Sept 15, 2025. Includes AI-powered programming and chat tools (like GitHub Copilot).

---

## 6. Secrets Management

### 6.1 Overview of Modern Approaches

**Moving Beyond .env Files:**

**Risks:**
- Accidental commits to Git
- No encryption at rest
- No audit trail
- Difficult rotation
- Duplication across projects
- **GhostAction Attack (Sept 2025):** 327 users, 817 repos, 3,325 secrets compromised

**Modern Principles:**
- Centralized secret storage
- Encryption at rest and in transit
- Audit logging
- Automatic rotation
- Access control (least privilege)
- Secret versioning

### 6.2 Cloud-Based Solutions

#### Phase ⭐ **RECOMMENDED**

**Website:** [phase.dev](https://phase.dev/)

**Key Features:**
- Seamless environment variable injection
- Git-styled diffs for secret changes
- Point-in-time recovery (restore any version)
- Dynamic secrets (short-lived, on-demand credentials)
- Zero code changes
- Multi-environment support

**Use Case:**
Teams needing version-controlled secrets with time-travel recovery.

#### Doppler

**Key Features:**
- Real-time secret sync across environments
- CLI for local development
- Built-in team sharing
- No .env file management

**CLI Usage:**
```bash
doppler run -- npm start
```

**Use Case:**
Teams wanting cloud-managed secrets with excellent developer experience.

#### HashiCorp Vault (Enterprise)

**Key Features:**
- Dynamic secret generation (databases, cloud services)
- Fine-grained access control policies
- Comprehensive audit logging
- Industry standard for enterprises

**Use Case:**
Large organizations with complex security requirements.

### 6.3 Self-Hosted / Open Source

#### Infisical ⭐ **RECOMMENDED for Self-Hosting**

**Description:** Free, open-source secret management

**Deployment:** [Deploy on Railway](https://railway.com/deploy/infisical)

**Key Features:**
- Self-hosted or cloud
- Environment variable synchronization
- Flexible APIs
- Better self-hosting experience than EnvKey
- Team collaboration features
- Integrations with CI/CD

**CLI:**
```bash
# Install
brew install infisical/get-cli/infisical

# Login
infisical login

# Run app with secrets
infisical run -- npm start
```

**P2P Opportunity:**
Self-hosted Infisical could use Code Bridge's P2P layer for distributed sync without central server.

#### env-sync (Laravel)

**Description:** Laravel package for secret sync

**Features:**
- 1Password integration
- AWS Secrets Manager support
- Bidirectional sync
- Multi-environment (local, staging, prod)

**GitHub:** [Metacomet-Technologies/env-sync](https://github.com/Metacomet-Technologies/env-sync)

**Use Case:**
Laravel teams using 1Password or AWS Secrets Manager.

#### Envilder (AWS SSM)

**Description:** Fast CLI tool for AWS SSM Parameter Store

**Key Features:**
- Written in Go (fast)
- Fetches from AWS SSM to local .env
- Single-source-of-truth enforcement
- Developer-friendly

**Use Case:**
Teams using AWS SSM for secrets, wanting local .env sync.

### 6.4 Developer Tools - 1Password

#### 1Password Developer Features

**Secrets Automation API:**
- Private REST API for accessing 1Password items
- Connect server for apps and infrastructure
- Pre-built integrations and client libraries

**SSH Key Management:**
- Generate/import SSH keys in 1Password app
- Keys stored encrypted (never plaintext on disk)
- Built-in SSH agent
- Keys never leave 1Password app
- Usage requires explicit authorization

**Recent Updates (January 2025):**
- Create item sharing links via API
- Move items to archive programmatically
- Retrieve SSH key metadata (public key, type, fingerprint)
- Resolve secrets via `client.Secrets.Resolve`

**SDKs:**
- Go
- JavaScript
- Python

**Sharing & Collaboration:**
- Share public SSH keys via browser extension
- Keys synced across devices
- Shared vaults for team access
- Single source of truth

**Developer Watchtower:**
- Scans `~/.ssh` for private keys on disk
- Warns about exposed credentials
- Detects OpenSSH, PKCS#8, PKCS#1 formats

**Statistics (1Password Survey):**
- 80% of companies don't manage secrets well
- 60% have experienced secret leaks
- 75% of developers have access to former employers' infrastructure secrets

**Use Case:**
Individual developers and small teams needing secure credential sync with excellent UX.

### 6.5 P2P Secret Sharing Recommendations

**For Code Bridge:**

**Architecture:**
```
┌─────────────────────────────────────────────────┐
│ Code Bridge Encrypted Secret Sync              │
├─────────────────────────────────────────────────┤
│                                                  │
│  Developer A          P2P Sync          Developer B
│  ┌──────────────┐                    ┌──────────────┐
│  │ Encrypted    │ ◄────────────────► │ Encrypted    │
│  │ Secret Vault │    libp2p +        │ Secret Vault │
│  │              │    ChaCha20        │              │
│  │ .env.enc     │                    │ .env.enc     │
│  └──────────────┘                    └──────────────┘
│         │                                    │        │
│         ▼                                    ▼        │
│  Decrypt locally                      Decrypt locally │
│  with device key                      with device key │
│                                                  │
└─────────────────────────────────────────────────┘
```

**Features:**
1. **End-to-end encryption:** ChaCha20-Poly1305
2. **Device-specific decryption:** Each device has unique key
3. **Secret versioning:** CRDT-based updates
4. **Audit trail:** Log all secret access
5. **Selective sharing:** Grant access per secret
6. **Offline-capable:** Decrypt locally, sync when online

**Implementation:**
```rust
pub struct SecretVault {
    // Encrypted secrets
    secrets: HashMap<SecretId, EncryptedSecret>,

    // Device keypair (Ed25519)
    device_key: Ed25519Keypair,

    // Shared secrets (encrypted for multiple devices)
    shared_secrets: HashMap<SecretId, Vec<PeerId>>,

    // CRDT for conflict-free updates
    crdt: Automerge,
}

impl SecretVault {
    // Encrypt secret for specific devices
    pub fn share_secret(
        &mut self,
        name: &str,
        value: &str,
        devices: &[PeerId]
    ) -> Result<SecretId> {
        // Encrypt with each device's public key
        // Store in CRDT
        // Sync via P2P
    }

    // Decrypt secret for local use
    pub fn get_secret(&self, name: &str) -> Result<String> {
        // Decrypt using device private key
        // Audit log access
    }
}
```

**Security:**
- Secrets never transmitted in plaintext
- Each device can only decrypt secrets shared with it
- Revocation: remove device from shared_secrets list
- Audit trail: all access logged with timestamp + device

**Use Cases:**
- **Team .env sync:** Share database URLs, API keys
- **SSH key distribution:** Securely share public/private keys
- **Certificate sync:** Development SSL certificates
- **API tokens:** GitHub, AWS, etc.

---

## 7. Cross-Platform Recommendations for P2P Sync (macOS ↔ Ubuntu)

### 7.1 Priority: High-Value Syncs

#### 1. IDE Settings ⭐

**VS Code:**
- Use built-in Settings Sync for cloud
- **P2P enhancement:** Replace cloud with libp2p sync
- Sync `settings.json`, `keybindings.json`, extensions list

**JetBrains:**
- Use Backup and Sync plugin for cloud
- **P2P enhancement:** Sync `.idea` configs via Code Bridge

**Neovim:**
- Use GNU Stow or chezmoi
- **P2P sync:** `~/.config/nvim` via Code Bridge dotfiles feature

#### 2. DevContainers ⭐⭐⭐ **HIGHEST PRIORITY**

**Why Critical:**
- Eliminates "Works on My Machine"
- Identical environments on macOS and Ubuntu
- Version-controlled

**Code Bridge Integration:**
```bash
# Sync devcontainer configs via P2P
codebridge add .devcontainer/
codebridge commit -m "Update dev environment"
codebridge push --all
```

**Benefits:**
- Team gets identical environment instantly
- No Docker Hub/GHCR needed (P2P image sharing)
- Offline-capable

#### 3. Language-Specific Configs ⭐⭐

**Rust:**
- Sync `Cargo.lock` via git (already standard)
- Sync `~/.cargo/config.toml` via dotfiles

**Node:**
- Use proto (`.prototools` file in repo)
- Commit to version control

**Python:**
- Use uv (`uv.lock` in repo)
- Commit lockfile

#### 4. Secrets ⭐⭐⭐ **HIGHEST PRIORITY**

**Approach:**
- Use Code Bridge's encrypted P2P secret vault
- **Never** commit secrets to git
- Share via E2E encrypted channels

**Commands:**
```bash
# Share secret with team
codebridge secret add DATABASE_URL --share-with team

# Access secret
codebridge secret get DATABASE_URL
```

#### 5. Git Stash ⭐

**Approach:**
- Use Git 2.51.0's native stash export/import
- Sync via git remote (standard approach)
- **P2P enhancement:** Code Bridge could store stashes in content-addressed storage

### 7.2 Cross-Platform Considerations

#### Path Differences

**Handle Platform-Specific Paths:**

**Chezmoi Template:**
```bash
# .bashrc.tmpl
export EDITOR={{ if eq .chezmoi.os "darwin" }}code{{ else }}vim{{ end }}
```

**DevContainer:**
```json
{
  "mounts": [
    "source=${localEnv:HOME}/.ssh,target=/home/vscode/.ssh,type=bind,consistency=cached"
  ]
}
```

**Note:** macOS uses `$HOME`, Linux uses `$HOME`, both work.

#### Tool Availability

**macOS-Specific:**
- Xcode (obviously)
- Homebrew (primary package manager)

**Linux-Specific:**
- apt/dnf/pacman (package managers)
- Different clipboard managers

**Cross-Platform:**
- Rust tools (cargo, ripgrep, fd, bat, eza)
- Node/Python (via version managers)
- Docker/Podman
- Git

**Recommendation:**
Use DevContainers to abstract away host OS differences. Inside container, everything is Linux.

#### Network Filesystem Access

**macOS:**
- SMB shares work well
- AFP (legacy)

**Ubuntu:**
- SMB/CIFS
- NFS

**Code Bridge P2P:**
- No filesystem mounting needed
- Direct P2P file transfer
- Works identically on both platforms

### 7.3 Dotfiles Sync Strategy

**Recommended Approach:**

1. **Create dotfiles repo:**
```bash
~/dotfiles/
├── nvim/
│   └── .config/nvim/
├── zsh/
│   ├── .zshrc
│   └── .zsh/
├── git/
│   └── .gitconfig
└── .stow-local-ignore
```

2. **Use GNU Stow for symlinking:**
```bash
cd ~/dotfiles
stow nvim zsh git
```

3. **Sync via Code Bridge:**
```bash
codebridge init ~/dotfiles --type dotfiles
codebridge add .
codebridge commit -m "Initial dotfiles"
codebridge sync
```

4. **On new machine:**
```bash
codebridge pull dotfiles
cd ~/dotfiles
stow nvim zsh git
```

**Benefits:**
- P2P sync (no cloud)
- Version controlled
- Selective stowing (pick what you need)

---

## 8. Implementation Recommendations for Code Bridge

### 8.1 Phase 1: Core Syncs (Months 1-3)

**Priority Features:**

1. **Dotfiles Sync**
```bash
codebridge dotfiles init
codebridge dotfiles add ~/.config/nvim
codebridge dotfiles add ~/.zshrc
codebridge dotfiles sync
```

2. **DevContainer Sync**
```bash
codebridge devcontainer sync  # Auto-detects .devcontainer/
codebridge devcontainer apply  # Applies to local Docker
```

3. **Secret Management**
```bash
codebridge secret add API_KEY --encrypt
codebridge secret share DATABASE_URL --to team
codebridge secret sync
```

**Architecture:**
- Reuse existing libp2p infrastructure
- Store dotfiles in content-addressed storage
- Encrypt secrets with ChaCha20-Poly1305
- CRDT for conflict resolution

### 8.2 Phase 2: IDE Integration (Months 4-6)

**VS Code Extension:**
```typescript
// codebridge-vscode/
export class CodeBridgeExtension {
    // Sync settings via P2P
    async syncSettings(): Promise<void>;

    // Share workspace state
    async shareWorkspace(peers: PeerId[]): Promise<void>;

    // Sync devcontainer
    async syncDevContainer(): Promise<void>;
}
```

**JetBrains Plugin:**
```kotlin
// Similar functionality for IntelliJ, PyCharm, etc.
class CodeBridgePlugin {
    fun syncSettings()
    fun syncWorkspace()
}
```

**Features:**
- One-click sync to team
- Notification on peer settings update
- Conflict resolution UI

### 8.3 Phase 3: Language Tooling (Months 7-9)

**Cargo Integration:**
```bash
codebridge rust sync  # Syncs Cargo.lock, config.toml
codebridge rust verify  # Checks all team members have same versions
```

**Node Integration:**
```bash
codebridge node sync  # Syncs .prototools or .nvmrc
codebridge node install  # Installs exact versions
```

**Python Integration:**
```bash
codebridge python sync  # Syncs uv.lock or poetry.lock
codebridge python verify
```

**Architecture:**
- Detect lock files automatically
- Verify consistency across team
- Alert on version mismatches

### 8.4 Phase 4: Advanced Features (Months 10-12)

**Workspace State Sync:**
```bash
# Save workspace snapshot
codebridge workspace save "before refactor"

# Share workspace state
codebridge workspace share alice@laptop

# Restore workspace
codebridge workspace restore "before refactor"
```

**Git Integration:**
```bash
# Sync git stashes via P2P
codebridge git stash-sync

# Share branches (not pushed to remote)
codebridge git share-branch feature-x --to bob
```

**Database Seed Sync:**
```bash
# Share database seeds
codebridge db seed add seeds.sql
codebridge db seed sync
```

### 8.5 Security Architecture

**Encrypted Secret Vault:**
```rust
pub struct SecretManager {
    // ChaCha20-Poly1305 encryption
    cipher: ChaCha20Poly1305,

    // Per-device keypair
    device_key: Ed25519Keypair,

    // Shared secrets (encrypted for each device)
    vault: EncryptedVault,

    // CRDT for conflict-free updates
    crdt: Automerge,
}
```

**Access Control:**
- Per-secret permissions
- Device-based authentication
- Audit logging
- Automatic key rotation

**Threat Model:**
- Protect against MITM: TLS 1.3 + WireGuard
- Protect against tampering: Content addressing + signatures
- Protect against unauthorized access: E2E encryption
- Protect against key compromise: Device-specific keys, revocation

---

## 9. Technology Stack Recommendations

### 9.1 Core Stack (Already in Code Bridge)

**Existing:**
- **Rust:** Core language
- **libp2p:** P2P networking
- **Automerge:** CRDT for sync
- **SQLite:** Local database
- **ChaCha20-Poly1305:** Encryption
- **Ed25519:** Identity/signatures

**Additions for Dev Environment Sync:**
```toml
[dependencies]
# DevContainer support
bollard = "0.16"  # Docker API client

# Dotfiles management
stow = "2.3"  # GNU Stow Rust binding (or shell out)

# Secret encryption
age = "0.10"  # Modern encryption (alternative to GPG)

# Configuration parsing
toml = "0.8"
serde_json = "1.0"
serde_yaml = "0.9"

# Shell integration
nu-parser = "0.90"  # Nushell parser for .env files
```

### 9.2 Integration Layers

**VS Code Extension:**
```json
{
  "name": "codebridge",
  "version": "1.0.0",
  "engines": {
    "vscode": "^1.85.0"
  },
  "dependencies": {
    "@tauri-apps/api": "^2.1.0"
  }
}
```

**JetBrains Plugin:**
```kotlin
// build.gradle.kts
plugins {
    id("org.jetbrains.intellij") version "1.17.0"
}

intellij {
    version.set("2025.1")
    plugins.set(listOf("com.intellij.java"))
}
```

**CLI Enhancements:**
```bash
codebridge dotfiles <subcommand>
codebridge secret <subcommand>
codebridge devcontainer <subcommand>
codebridge workspace <subcommand>
```

---

## 10. Competitive Analysis

### 10.1 Code Bridge vs Alternatives

| Feature | Code Bridge | Settings Sync | Chezmoi | DevContainers | 1Password |
|---------|-------------|---------------|---------|---------------|-----------|
| **Architecture** | P2P | Cloud | Git | Local/Cloud | Cloud |
| **Privacy** | ✅ Full | ⚠️ Cloud | ✅ Full | ⚠️ Varies | ⚠️ Cloud |
| **Offline** | ✅ Full | ❌ No | ✅ Full | ✅ Local | ❌ No |
| **Vendor Lock-in** | ✅ None | ❌ High | ✅ None | ⚠️ Medium | ❌ High |
| **IDE Settings** | ✅ Yes | ✅ Yes | ⚠️ Manual | ❌ No | ❌ No |
| **DevContainers** | ✅ Yes | ❌ No | ❌ No | ✅ Yes | ❌ No |
| **Secrets** | ✅ E2E | ⚠️ Cloud | ⚠️ GPG | ❌ No | ✅ E2E |
| **Cross-IDE** | ✅ Yes | ⚠️ Limited | ✅ Yes | ⚠️ Limited | ❌ No |
| **Cost** | Free | Free/Paid | Free | Free | Paid |
| **Team Sync** | ✅ P2P | ✅ Cloud | ⚠️ Git | ⚠️ Manual | ✅ Cloud |

### 10.2 Unique Value Propositions

**Code Bridge Advantages:**

1. **True P2P:** No cloud dependency, direct peer sync
2. **Unified Platform:** One tool for files, settings, secrets, environments
3. **Privacy-First:** E2E encryption, no telemetry, local-first
4. **Cross-Platform:** macOS ↔ Ubuntu seamlessly
5. **Developer-Native:** Integrates with existing workflows (git, docker, IDEs)
6. **Offline-First:** Work without internet, sync when available
7. **No Vendor Lock-in:** Open source, standard formats

**Use Cases:**

**Individual Developers:**
- Sync laptop ↔ desktop seamlessly
- No subscription costs
- Full privacy

**Small Teams:**
- Share dev environments instantly
- No cloud costs
- Team autonomy

**Enterprises:**
- Air-gapped environments
- Compliance-friendly (data never leaves network)
- Audit trails

---

## 11. Research Sources

### IDE Settings Sync
- [VS Code Settings Sync](https://code.visualstudio.com/docs/configure/settings-sync)
- [Crosside Sync GitHub](https://github.com/jinghaihan/vscode-crosside-sync)
- [JetBrains Backup and Sync](https://www.jetbrains.com/help/idea/sharing-your-ide-settings.html)
- [JetBrains IDE Services 2025.4](https://www.jetbrains.com/help/ide-services/2025-4.html)
- [Neovim Dotfiles Inspiration](https://dotfiles.github.io/inspiration/)
- [GNU Stow for Dotfiles](https://www.penkin.me/development/tools/productivity/configuration/2025/10/20/my-dotfiles-setup-with-gnu-stow.html)

### Development Containers
- [VS Code Developing inside a Container](https://code.visualstudio.com/docs/devcontainers/containers)
- [Development Containers Spec](https://containers.dev/)
- [DevContainer Guide 2025](https://devtechinsights.com/what-is-devcontainer-developers-2025/)
- [Daytona Ultimate Guide](https://www.daytona.io/dotfiles/ultimate-guide-to-dev-containers)
- [Northflank Codespaces Alternatives](https://northflank.com/blog/github-codespaces-alternatives)
- [DevPod Open Source](https://openalternative.co/alternatives/github-codespaces)

### Package Management
- [How I Manage Node Versions in 2025](https://dev.to/michalbryxi/how-i-manage-node-package-manager-versions-in-2025-97d)
- [uv: Modern Python Package Management](https://medium.com/@debisreer/from-pip-to-uv-a-modern-revolution-in-python-package-management-62dd8ac91df2)
- [Poetry Documentation](https://python-poetry.org/docs/)
- [NVM Documentation](https://www.nvmnode.com/)
- [Cargo Configuration](https://doc.rust-lang.org/cargo/reference/config.html)

### Secrets Management
- [Phase Secret Management](https://phase.dev/)
- [Infisical on Railway](https://railway.com/deploy/infisical)
- [Beyond .env Files 2025](https://instatunnel.my/blog/beyond-env-files-the-new-best-practices-for-managing-secrets-in-development)
- [1Password Developer Security](https://1password.com/developer-security)
- [1Password SSH Key Management](https://www.acciyo.com/master-your-ssh-keys-with-1password-the-developers-secret-weapon/)
- [env-sync GitHub](https://github.com/Metacomet-Technologies/env-sync)

### Git & Workspace
- [Git 2.51.0 Stash Export/Import](https://devblogs.microsoft.com/visualstudio/streamlining-your-git-workflow-with-visual-studio-2026/)
- [Moving Git Stashes Between Devices](https://dev.to/hootanht/moving-git-stashes-between-devices-a-step-by-step-guide-4aaf)
- [IDESync VSCode-JetBrains](https://github.com/denisbalber/IDESync-VSCode-JetBrains)
- [Todo Tree VS Code](https://www.stepsize.com/blog/best-vs-code-extensions-to-handle-todos)

### Cross-Platform Tools
- [mkcert for Local SSL](https://github.com/FiloSottile/mkcert)
- [Database Seeding Best Practices](https://supabase.com/docs/guides/local-development/seeding-your-database)
- [xcodes for Xcode Management](https://github.com/XcodesOrg/xcodes)
- [IdeaVim Documentation](https://www.jetbrains.com/help/idea/using-product-as-the-vim-editor.html)
- [pyenv Python Version Management](https://hyscaler.com/insights/managing-python-versions-with-pyenv/)

---

## 12. Conclusion

Development environment synchronization is evolving rapidly in 2025, with key trends:

1. **DevContainers** becoming the standard for reproducible environments
2. **Secret management** moving beyond .env files to centralized, encrypted solutions
3. **Language-specific tools** (uv, proto) providing lockfile-based determinism
4. **Git enhancements** (stash export/import) enabling better cross-machine workflows
5. **P2P opportunities** for privacy-first, cloud-free synchronization

### Key Recommendations for Code Bridge

**High-Priority Integrations:**

1. **DevContainer Sync** - Highest ROI, solves "Works on My Machine"
2. **Encrypted Secret Vault** - Critical for teams, differentiator vs cloud solutions
3. **Dotfiles Management** - Foundation for all other syncs
4. **IDE Settings Sync** - High developer value, moderate complexity

**Competitive Advantages:**

- **Privacy:** True P2P, no cloud, E2E encryption
- **Unification:** One tool for files, settings, secrets, environments
- **Offline-First:** Works without internet
- **Developer-Native:** Integrates with existing tools (git, docker, IDEs)

**Market Position:**

Code Bridge can position as:
- **Syncthing for developers:** Privacy-first file sync + dev environment sync
- **1Password + DevContainers + Dotfiles:** Unified developer platform
- **Air-gapped Codespaces:** Enterprise-friendly, on-premises dev environments

### Next Steps

1. **Prototype:** Build DevContainer sync proof-of-concept
2. **User Research:** Interview developers about environment sync pain points
3. **Architecture:** Design P2P secret vault and dotfile sync
4. **Community:** Open source devcontainer sync format for adoption

---

**Research Compiled By:** Claude Code (Sonnet 4.5)
**Date:** December 20, 2025
**Version:** 1.0
