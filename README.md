# command_schedul_db

终端任务管理工具，基于 PostgreSQL 数据库存储。是 [command_schedule](https://github.com/Knightxxj/command_schedule) 的数据库版本，从 JSONL 文件存储迁移到了 PostgreSQL + Docker。

## 安装

```bash
cargo build --release
```

## 快速开始

### 1. 启动 PostgreSQL

```bash
docker compose up -d
```

### 2. 运行

```bash
cargo run -- <命令> [参数]
```

## 用法

```bash
schedule <命令> [参数]
```

### 命令

| 命令 | 说明 |
|------|------|
| `list` | 列出所有任务，支持 `--tag` / `--priority` 过滤 |
| `add` | 添加新任务 |
| `done` | 标记任务为完成 |
| `remove` | 删除任务 |
| `help` | 显示帮助信息 |

### 示例

```bash
# 列出所有任务
schedule list

# 按标签过滤
schedule list --tag=work

# 按优先级过滤
schedule list --priority=1

# 添加任务
schedule add "完成周报" --tag=work --priority=2

# 标记完成
schedule done task_00001

# 删除任务
schedule remove task_00001
```

## 环境变量

| 变量 | 说明 | 默认值 |
|------|------|--------|
| `SCHEDULE_DATABASE_URL` | PostgreSQL 连接串 | `postgres://schedule:schedule@localhost:5432/schedule` |

## Docker 管理

```bash
docker compose up -d       # 启动数据库
docker compose down        # 停止（保留数据）
docker compose down -v     # 停止并清除数据
docker compose logs db     # 查看日志
```

## 技术栈

- Rust (edition 2021)
- sqlx（PostgreSQL 驱动 + 编译时 SQL 校验 + migration）
- tokio（异步运行时）
- PostgreSQL 16 (Docker)

## 架构对比

| | command_schedule (JSONL) | command_schedul_db (PostgreSQL) |
|------|:--|:--|
| 原子性 | 两步写入无保护 | DB 事务保证 |
| 并发 | 无锁，多进程可能写坏数据 | 行锁 + 事务隔离 |
| done/remove 性能 | O(n) 全量读写 | O(1) 单条 UPDATE/DELETE |
| 查询性能 | 全量读到内存再过滤 | SQL 过滤下推到 DB |
| 数据校验 | 无 | NOT NULL + 主键唯一 |
| 首次运行 | 文件不存在则崩溃 | Migration 自动建表 |

## 相关项目

- [command_schedule](https://github.com/Knightxxj/command_schedule) — 基于 JSONL 文件的原始版本
