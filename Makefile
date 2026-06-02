# 声明伪目标，防止与同名文件冲突
.PHONY: db-up db-stop db-down run clean

# 定义变量，方便后期统一修改
DC := docker-compose
CARGO := cargo

# 1. 基础设施层：精确控制数据库生命周期
db-up:
	@echo "[INFO] 拉起并确保数据库后台运行..."
	$(DC) up -d

db-stop:
	@echo "[INFO] 暂停数据库进程，释放物理内存..."
	$(DC) stop

db-down:
	@echo "[INFO] 销毁数据库容器与网络环境..."
	$(DC) down

db-init: db-up
	@echo "[INFO] 等待数据库引擎启动 (根据机器性能调整睡眠时间)..."
	sleep 3 
	@echo "[INFO] 创建数据库并执行数据模式迁移..."
	sqlx database setup
	@echo "[SUCCESS] 数据库初始化完美完成！"

# 2. 业务运行层：统筹基础设施与应用逻辑
run: db-up
	@echo "[INFO] 启动 Rust 应用..."
	$(CARGO) run

# 3. 工程清理层：一键回到干净状态
clean: db-down
	@echo "[INFO] 清理 Rust 编译产物..."
	$(CARGO) clean