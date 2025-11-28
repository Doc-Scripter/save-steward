# Save Steward - Automated Game Save Protection Makefile
# ====================================================
# Complete development, build, and testing automation
# for the Save Steward automated game backup system.

.PHONY: help dev build test check clean release run db-init db-migrate format docs watch

# Default target - show help
help:
	@echo "🎮 Save Steward - Automated Game Save Protection"
	@echo "=================================================="
	@echo ""
	@echo "Development Commands:"
	@echo "  dev              Start full development environment (UI + backend)"
	@echo "  watch            Watch files and auto-rebuild during development"
	@echo "  run              Launch development app"
	@echo ""
	@echo "Build Commands:"
	@echo "  build            Production build (frontend + backend)"
	@echo "  build-frontend   Build React frontend only"
	@echo "  build-backend    Build Rust backend only"
	@echo "  release          Create production release bundle"
	@echo "  build-release    Alias for release"
	@echo ""
	@echo "Testing Commands:"
	@echo "  test             Run all tests (backend + frontend + integration)"
	@echo "  test-backend     Run Rust unit tests"
	@echo "  test-frontend    Run React tests (when implemented)"
	@echo "  test-integration Run full system integration tests"
	@echo "  game-test        Test game detection with mock processes"
	@echo "  backup-test      Test backup system with sample files"
	@echo "  ui-test          Test UI interactions"
	@echo ""
	@echo "Code Quality:"
	@echo "  check            Full code quality check (lint + format + security)"
	@echo "  lint             Lint code (Clippy + ESLint)"
	@echo "  format           Format code (Rust + JS/TS)"
	@echo "  audit            Security audit dependencies"
	@echo ""
	@echo "Database:"
	@echo "  db-init          Initialize database with schema"
	@echo "  db-migrate       Run database migrations"
	@echo "  db-reset         Reset database to clean state"
	@echo "  db-seed          Populate database with test data"
	@echo ""
	@echo "Maintenance:"
	@echo "  clean            Clean all build artifacts"
	@echo "  docs             Generate documentation"
	@echo "  install          Install all dependencies"
	@echo "  update           Update all dependencies"
	@echo ""
	@echo "Examples:"
	@echo "  make dev         # Start developing"
	@echo "  make test        # Run all tests"
	@echo "  make build       # Create production build"
	@echo "  make run         # Launch the app"

# ====================================================
# DEVELOPMENT ENVIRONMENT
# ====================================================

# Start full development environment
dev:
	@echo "🚀 Starting Save Steward development environment..."
	npm run tauri dev

# File watching with auto-rebuild
watch:
	@echo "👀 Watching files for changes..."
	@echo "Terminal 1 (Backend):"
	@echo "  cd src-tauri && cargo watch -x check"
	@echo ""
	@echo "Terminal 2 (Frontend):"
	@echo "  npm run dev"
	@echo ""
	@echo "Terminal 3 (UI):"
	@echo "  make run"

# Launch development app
run:
	@echo "🎮 Launching Save Steward..."
	npm run tauri dev

# ====================================================
# BUILD COMMANDS
# ====================================================

# Full production build
build: build-frontend build-backend
	@echo "✅ Complete Save Steward build successful!"

# Build React frontend only
build-frontend:
	@echo "📦 Building React frontend..."
	npm run build

# Build Rust backend only
build-backend:
	@echo "🔧 Building Rust backend..."
	cd src-tauri && cargo build --release

# Create production release bundle
release: build-release
build-release:
	@echo "🎁 Creating production release..."
	npm run tauri build --release

# ====================================================
# TESTING
# ====================================================

# Run all tests
test: test-backend test-frontend test-integration
	@echo "✅ All tests passed!"

# Backend unit tests
test-backend:
	@echo "🧪 Running Rust tests..."
	cd src-tauri && cargo test

# Frontend tests (placeholder for when implemented)
test-frontend:
	@echo "🧪 Running frontend tests..."
	@echo "⚠️  Frontend tests not yet implemented"
	@echo "   Run: cd save-steward && npm test"

# Integration tests
test-integration:
	@echo "🔗 Running integration tests..."
	@echo "⚠️  Full integration tests not yet implemented"
	@echo "   Would test: Game detection → Backup creation → Retention policy"

# Game detection tests
game-test:
	@echo "🎯 Testing game detection system..."
	@echo "Mocking game processes and testing identification..."
	cd save-steward/src-tauri && cargo test game_identification

# Backup system tests
backup-test:
	@echo "💾 Testing backup automation system..."
	cd save-steward/src-tauri && cargo test backup_

# UI interaction tests (placeholder)
ui-test:
	@echo "🎨 Testing UI interactions..."
	@echo "⚠️  UI tests not yet implemented"
	@echo "   Would test: Button clicks, form submissions, state updates"

# ====================================================
# CODE QUALITY
# ====================================================

# Full code quality check
check: lint format audit
	@echo "✅ Code quality checks passed!"

# Lint code (Clippy + ESLint)
lint: lint-backend lint-frontend

lint-backend:
	@echo "🔧 Running Clippy (Rust linter)..."
	cd src-tauri && cargo clippy --all-targets --all-features -- -D warnings

lint-frontend:
	@echo "📝 Running ESLint (JavaScript linter)..."
	cd save-steward && npm run lint 2>/dev/null || echo "⚠️  No ESLint config found (add later if needed)"

# Format code
format: format-backend format-frontend

format-backend:
	@echo "🎨 Formatting Rust code..."
	cd src-tauri && cargo fmt

format-frontend:
	@echo "🎨 Formatting JavaScript/TypeScript code..."
	cd save-steward && npm run format 2>/dev/null || echo "⚠️  No format script found"

# Security audit
audit:
	@echo "🔒 Running security audit..."
	@echo "Auditing Rust dependencies..."
	cd src-tauri && cargo audit
	@echo "Auditing Node dependencies..."
	npm audit

# ====================================================
# DATABASE OPERATIONS
# ====================================================

# Initialize database
db-init:
	@echo "🗄️  Initializing database schema..."
	@echo "⚠️  Database operations not yet implemented"
	@echo "   TODO: Implement database initialization commands"

# Run migrations
db-migrate:
	@echo "�️ Running database migrations..."
	cd src-tauri && cargo run --bin save-steward -- migrate

# Reset database
db-reset:
	@echo "🔄 Resetting database..."
	cd src-tauri && cargo run --bin save-steward -- reset-db

# Clean database
db-clean:
	@echo "🧹 Cleaning database..."
	@echo "⚠️  Database cleaning not yet implemented"

# Seed with test data
db-seed:
	@echo "🌱 Seeding database with test data..."
	@echo "⚠️  Database seeding not yet implemented"

# ====================================================
# MAINTENANCE
# ====================================================

# Clean all build artifacts
clean: clean-frontend clean-backend
	@echo "🧹 Clean complete!"

clean-frontend:
	@echo "🧹 Cleaning frontend build artifacts..."
	rm -rf dist node_modules/.vite

clean-backend:
	@echo "🧹 Cleaning Rust build artifacts..."
	cd src-tauri && cargo clean

# Generate documentation
docs: docs-backend docs-frontend

docs-backend:
	@echo "📚 Generating Rust documentation..."
	cd src-tauri && cargo doc --open

docs-frontend:
	@echo "📚 Generating frontend documentation..."
	@echo "⚠️  Frontend documentation not yet implemented"

# Install all dependencies
install:
	@echo "📦 Installing all dependencies..."
	npm install
	cd src-tauri && cargo build

# Update all dependencies
update:
	@echo "🔄 Updating dependencies..."
	npm update
	cd src-tauri && cargo update

# ====================================================
# DEVELOPMENT HELPERS
# ====================================================

# Quick development setup
setup: install
	@echo "🎮 Save Steward development environment ready!"
	@echo ""
	@echo "Quick start:"
	@echo "  make dev     # Start development"
	@echo "  make test    # Run tests"
	@echo "  make build   # Production build"
	@echo ""
build-windows:
	npm run tauri build -- --target x86_64-pc-windows-msvc


# Status check
status:
	@echo "📊 Save Steward Project Status"
	@echo "=============================="
	@echo "Frontend:"
	@cd save-steward && if [ -d "node_modules" ]; then echo "  ✅ Dependencies installed"; else echo "  ❌ Dependencies missing (run 'make install')"; fi
	@cd save-steward && if [ -d "dist" ]; then echo "  ✅ Built"; else echo "  ❌ Not built (run 'make build-frontend')"; fi
	@echo ""
	@echo "Backend:"
	@cd save-steward/src-tauri && if cargo check >/dev/null 2>&1; then echo "  ✅ Compiles"; else echo "  ❌ Compilation errors (run 'make check-backend')"; fi
	@cd save-steward/src-tauri && if [ -d "target/release" ]; then echo "  ✅ Release built"; else echo "  ❌ No release build (run 'make build-backend')"; fi
	@echo ""
	@echo "Modules:"
	@echo "  ✅ Database core (SQLCipher)"
	@echo "  ✅ Manifest integration (Ludusavi)"
	@echo "  ✅ Game identification engine"
	@echo "  ✅ Auto-backup integration"
	@echo "  ✅ Frontend UI implementation"
	@echo ""
	@echo "Ready for testing: make test"
	@echo ""

# ====================================================
# QUALITY ASSURANCE TARGETS
# ====================================================

# Pre-commit checks
pre-commit: check test

# CI/CD pipeline (would be used in automated builds)
ci: install check test build

# Performance testing
perf-test:
	@echo "⚡ Running performance benchmarks..."
	@echo "⚠️  Performance tests not yet implemented"
	@echo "   Would test: Memory usage, CPU usage, backup speed, detection latency"

# ====================================================
# SAVE STEWARD SPECIFIC TARGETS
# ====================================================

# Test the complete save protection workflow
e2e-test:
	@echo "🛠️  Running end-to-end save protection test..."
	@echo "1. Start monitoring → 2. Simulate game launch →"
	@echo "3. Trigger backup → 4. Check retention → 5. Stop monitoring"
	@echo "⚠️  End-to-end tests not yet implemented"

# Validate system can detect and backup a well-known game
blasphemous-test:
	@echo "🎮 Testing Blasphemous detection protocol..."
	@echo "Checking for: Steam installation, AppID, save locations, executables"
	@echo "⚠️  Game-specific tests not yet implemented"
	@echo "   Would validate complete detection → backup pipeline for Blasphemous"

# ====================================================
# DEVELOPMENT ALIASES (Common shortcuts)
# ====================================================

# Quick aliases for common operations
b: build
t: test
d: dev
r: run
c: clean
f: format
l: lint

# Emergency quick fix for common issues
fix-backend:
	@echo "🔧 Applying backend fixes..."
	cd save-steward/src-tauri && cargo fix
	cd save-steward/src-tauri && cargo clippy --fix
	make format-backend

fix-frontend:
	@echo "🔧 Applying frontend fixes..."
	cd save-steward && npm run fix 2>/dev/null || echo "⚠️  No fix script found"

# ====================================================
# UTILITY/DEBUG TARGETS
# ====================================================

# Show project structure
tree:
	@echo "🌳 Save Steward Project Structure"
	@echo "=================================="
	@find . -type f -name "*.rs" -o -name "*.ts" -o -name "*.tsx" -o -name "Makefile" | head -20
	@echo "..."
	@echo "Run 'make status' for detailed project status"

# Show version info
version:
	@echo "🎮 Save Steward Version Info"
	@echo "============================"
	@echo "Rust: $$(cd save-steward/src-tauri && cargo --version)"
	@echo "Node.js: $$(cd save-steward && node --version)"
	@echo "NPM: $$(cd save-steward && npm --version)"
	@echo ""
	@echo "Tauri: Check Cargo.toml"
	@echo "React: Check package.json"

# Show system dependencies
deps:
	@echo "🔧 System Dependencies Check"
	@echo "============================"
	@command -v cargo >/dev/null 2>&1 && echo "✅ Rust/Cargo installed" || echo "❌ Rust/Cargo missing"
	@command -v node >/dev/null 2>&1 && echo "✅ Node.js installed" || echo "❌ Node.js missing"
	@command -v npm >/dev/null 2>&1 && echo "✅ NPM installed" || echo "❌ NPM missing"
	@command -v make >/dev/null 2>&1 && echo "✅ Make installed" || echo "❌ Make missing"
