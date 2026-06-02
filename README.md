# 终端朋友圈与社交关系系统 (Terminal Social Network)

本项目是一个基于 Rust 和 MySQL 构建的高性能、强类型的终端社交系统。系统实现了完整的领域驱动设计（DDD），在底层通过严格的外键约束与级联删除保证数据一致性，在表现层通过状态机隔离提供沉浸式的终端交互体验。

## 🌟 核心特性 (Features)

- **强类型状态管理**：利用 Rust 枚举与 `sqlx::Type` 实现零“魔法数字”的数据库状态映射。
- **完善的角色隔离**：
  - **普通用户态**：沉浸式终端朋友圈流、基于游标的分页、发布/编辑/删除动态、好友分组与请求流转。
  - **上帝审计态**：隐藏路由入口，无界限全站朋友圈分页扫描与强制物理抹除。
- **工业级数据约束**：底层利用 `ON DELETE CASCADE` 确保用户注销、关系解除时的嵌套数据（如评论、分组）瞬间安全湮灭。
- **自动化 DevOps 流水线**：通过 Makefile 统一接管 Docker 容器与数据模式（Schema）的生命周期。

## 🛠️ 技术栈 (Tech Stack)

- **Language**: Rust (edition 2024)
- **Database**: MySQL 8.0+ (运行于 Docker)
- **ORM / Driver**: `sqlx` (纯异步、编译期 SQL 校验)
- **Async Runtime**: `tokio`

## ⚙️ 环境依赖 (Prerequisites)

在运行本项目前，请确保系统已安装以下工具链：
1. **Rust & Cargo** (>= 1.80)
2. **Docker & Docker Compose**
3. **sqlx-cli**: 用于数据库迁移。如果未安装，请执行：
   ```bash
   cargo install sqlx-cli --no-default-features --features rustls,mysql

## 🚀 快速启动 (Quick Start)
1. 环境变量配置
在项目根目录确认或创建 .env 文件，配置数据库连接（注意特殊字符的 URL 编码）：
    ```bash
DATABASE_URL=mysql://root:root@localhost:3306/db_lab05?timezone=%2B00:00

2. 一键初始化环境
使用 Makefile 自动化流水线，该指令会自动拉起 MySQL 容器、等待引擎就绪，并注入所有建表脚本（Migrations）：
    ```bash
make db-init

3. 启动应用
编译并进入沉浸式终端系统：
    ```bash
make run

(提示：系统初始自带超级管理员账号 System_Root，密码为 adminpass，登入即自动切入上帝审计态。)

4. 常用运维指令 (Operations)
    ```bash
make db-up：仅拉起数据库容器（不重置数据）。
make db-down：⚠️ 停止容器并彻底销毁数据库持久化卷。用于开发环境的重置。

## 📁 核心架构目录 (Architecture)
    ```
src/
├── domain/         # 领域层：纯粹的业务实体与强类型枚举 (Role, RelationStatus)
├── infrastructure/ # 基础设施层：与 sqlx 强绑定的 DTO 和数据库操作接口
├── service/        # 业务逻辑层：权限阻断、参数清洗与领域调度
├── cli/            # 表现层：基于状态机 (State Machine) 的无损交互终端
└── main.rs         # 依赖注入与程序入口