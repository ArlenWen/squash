# Squash - Docker 镜像层压缩工具

**中文** | [English](README.md)

一个用 Rust 编写的高性能 Docker 镜像层压缩命令行工具。

## 🚀 特性

- **🔄 多种输入源**: 支持 Docker 镜像名称:标签或导出/保存的镜像文件
- **📤 灵活输出**: 保存到文件或直接加载到 Docker 并指定镜像名称和标签
- **🎯 智能层合并**: 
  - 按数量: 将最新的 n 层合并为一层
  - 按层 ID: 从指定层 ID 到最新层进行合并
- **📁 临时目录支持**: 配置中间文件的存储位置
- **📝 详细输出**: 操作的详细日志记录
- **⚡ 内存高效**: 大文件流式处理防止内存溢出
- **🔒 安全操作**: 路径遍历保护和适当的错误处理
- **🧪 充分测试**: 全面的单元测试、集成测试和基准测试

## 📦 安装

### 前置要求
- Rust 1.70+ (从源码构建)
- Docker (用于处理 Docker 镜像)

### 从源码构建
```bash
git clone https://github.com/your-username/squash.git
cd squash
cargo build --release
```

二进制文件将位于 `target/release/squash`。

### 通过 Cargo 安装
```bash
cargo install --path .
```

## 🛠️ 使用方法

### 基本用法

```bash
# 压缩镜像的最新 3 层并保存到文件
squash squash --source nginx:latest --output nginx-squashed.tar --layers 3

# 压缩层并直接加载到 Docker
squash squash --source nginx:latest --load my-nginx:squashed --layers 2

# 使用保存的镜像文件作为源
squash squash --source /path/to/image.tar --output squashed.tar --layers 3

# 详细输出和自定义临时目录
squash squash --source nginx:latest --output nginx-squashed.tar --layers 3 --temp-dir /tmp/squash --verbose
```

### 📋 命令行选项

| 选项 | 简写 | 描述 |
|------|------|------|
| `--source` | `-s` | 源镜像 (名称:标签或文件路径) |
| `--output` | `-o` | 输出文件路径 (如果不使用 --load 则必需) |
| `--load` | | 将结果加载到 Docker 并指定名称:标签 |
| `--temp-dir` | `-t` | 中间文件的临时目录 |
| `--layers` | `-l` | 层规范 (数量或层 ID) |
| `--verbose` | `-v` | 启用详细输出 |

### 🎯 层规范示例

```bash
# 合并最新的 3 层
--layers 3

# 从特定层 ID 到最新层合并 (最少需要 8 个字符)
--layers "sha256:abc123def456"

# 使用部分摘要合并层 (8+ 个字符)
--layers "abc12345"
```

### 💡 高级示例

```bash
# 首先导出 Docker 镜像，然后压缩
docker save nginx:latest -o nginx.tar
squash squash --source nginx.tar --output nginx-squashed.tar --layers 2

# 压缩并立即加载新标签
squash squash --source nginx:latest --load nginx:optimized --layers 3 --verbose

# 为大镜像使用自定义临时目录
squash squash --source large-image:latest --output optimized.tar --layers 5 --temp-dir /tmp/squash-work
```


## ✅ 核心功能
- **🔧 CLI 界面**: 功能完整的命令行界面
- **📦 Docker 集成**: 原生 Docker 镜像导出/导入支持
- **🔍 镜像解析**: 完整的 Docker 镜像格式支持
- **🏗️ 镜像重建**: 智能镜像重构与合并层
- **🎯 灵活合并**: 支持基于数量和 ID 的层合并
- **🔄 Docker 加载**: 与 Docker 守护进程直接集成
- **🔐 完整性检查**: SHA256 摘要计算和验证
- **📁 归档处理**: 完整的 tar 归档操作

## 🚀 性能特性
- **💾 内存高效**: 大文件流式处理
- **🛡️ 安全性**: 路径遍历保护和输入验证
- **🧹 资源管理**: 临时文件自动清理

## 🔮 计划改进
- **📊 进度指示器**: 长时间操作的进度条
- **🗜️ 压缩选项**: 可配置的压缩算法


## 🧪 测试

### 单元测试
```bash
# 运行所有单元测试 (13 个测试)
cargo test

# 运行带详细输出的测试
cargo test -- --nocapture

# 运行特定测试模块
cargo test docker::layer
```

### 集成测试
```bash
# 运行集成测试
cargo test --test integration_test

# 运行依赖 Docker 的测试 (需要 Docker)
cargo test --test integration_test -- --ignored

# 运行包括被忽略测试在内的所有测试
cargo test --test integration_test -- --include-ignored
```

### 性能基准测试
```bash
# 运行所有基准测试
cargo bench

# 运行特定基准测试
cargo bench layer_merger_creation
```

### 🔧 开发测试

#### 测试镜像生成
```bash
# 生成测试 Docker 镜像
python3 create_test_image.py

# 测试基本压缩功能
cargo run -- squash --source test-docker-image.tar --output squashed.tar --layers 2 --verbose

# 测试基于层 ID 的合并
cargo run -- squash --source test-docker-image.tar --output squashed-by-id.tar --layers "abc12345" --verbose
```

#### 代码质量检查
```bash
# 运行 clippy 进行代码质量检查
cargo clippy --all-targets --all-features

# 格式化代码
cargo fmt

# 安全审计
cargo audit
```

### 📝 贡献指南
1. Fork 仓库
2. 创建功能分支
3. 进行更改并添加测试
4. 确保所有测试通过
5. 提交 pull request


## 🔍 故障排除

### 常见问题

**错误: "Layer ID must be at least 8 characters long"**
- 解决方案: 使用层 ID 匹配时提供至少 8 个字符
- 示例: 使用 `--layers "abc12345"` 而不是 `--layers "abc"`

**错误: "Cannot merge 0 layers"**
- 解决方案: 指定有效的层数进行合并 (1 或更多)
- 示例: 使用 `--layers 2` 而不是 `--layers 0`

**大镜像内存问题**
- 解决方案: 在有足够空间的磁盘上使用自定义临时目录
- 示例: `--temp-dir /path/to/large/disk/temp`

**Docker 守护进程连接问题**
- 解决方案: 确保 Docker 正在运行且可访问
- 检查: `docker info` 应该能正常工作

### 调试模式
```bash
# 启用详细输出进行调试
squash squash --source image:tag --output result.tar --layers 2 --verbose

# 检查日志以获取详细的处理信息
RUST_LOG=debug cargo run -- squash --source image:tag --output result.tar --layers 2
```


**用 ❤️ 和 Rust 制作** | **如果觉得有用请给个 ⭐ Star！**


## 📄 许可证

本项目采用 MIT 许可证 - 详情请参阅 [LICENSE](LICENSE) 文件。
