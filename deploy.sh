#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
cd "$SCRIPT_DIR"

# ── 启动 PostgreSQL ──────────────────────────────────
echo "==> 启动 PostgreSQL..."
docker compose up -d

echo "==> 等待 PostgreSQL 就绪..."
until docker compose exec db pg_isready -U schedule -d schedule &>/dev/null; do
    sleep 1
done
echo "==> PostgreSQL 已就绪"

# ── 编译 ─────────────────────────────────────────────
echo "==> 编译 schedule_db..."
cargo build --release

# ── 运行 ─────────────────────────────────────────────
echo "==> 启动成功，可以开始使用："
echo ""
echo "  cargo run --release -- list"
echo "  cargo run --release -- add \"测试任务\" --tag=start --priority=1"
echo "  cargo run --release -- done task_00001"
echo ""
exec "$@"
