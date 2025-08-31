# Wikify Web 集成测试

本目录包含了 Wikify Web 服务器的全面集成测试，参考了 zero-to-production 项目的最佳实践。

## 🏗️ 测试架构

### 测试结构

```
tests/
├── helpers.rs                    # 测试辅助工具和共享代码
├── auth_permissions_test.rs      # 认证和权限测试
├── api_endpoints_test.rs         # API端点功能测试
├── integration_tests.rs          # 现有的集成测试
├── research_*.rs                 # 研究功能测试
└── README.md                     # 本文档
```

### 核心设计原则

1. **真实应用测试** - 启动完整的 Wikify Web 应用，而不是使用 mock handlers
2. **权限模式覆盖** - 测试 open、private、enterprise 三种权限模式
3. **HTTP 客户端测试** - 使用 `reqwest::Client` 发送真实的 HTTP 请求
4. **测试隔离** - 每个测试使用独立的端口和内存数据库
5. **全面覆盖** - 测试所有主要 API 端点和权限组合

## 🧪 测试文件详解

### `helpers.rs` - 测试辅助工具

提供了测试所需的核心基础设施：

- **`TestApp`** - 测试应用实例，包含地址和 HTTP 客户端
- **`spawn_app_with_mode()`** - 创建指定权限模式的测试应用
- **`TestUser`** - 生成测试用户数据
- **HTTP 方法封装** - 为所有 API 端点提供便捷的测试方法

### `auth_permissions_test.rs` - 认证权限测试

全面测试认证和权限系统：

#### 测试覆盖范围

1. **Open 模式测试**
   - 验证认证状态返回正确的模式信息
   - 确保所有端点允许匿名访问
   - 测试公开端点的可访问性

2. **Private 模式测试**
   - 验证认证状态要求认证
   - 确保受保护端点返回 401 未授权
   - 验证公开端点仍然可访问

3. **Enterprise 模式测试**
   - 验证企业级功能配置
   - 测试高级权限控制

4. **认证流程测试**
   - 用户注册功能测试
   - 用户登录功能测试
   - JWT token 验证测试

### `api_endpoints_test.rs` - API端点功能测试

测试所有主要 API 端点的功能性：

#### 端点分类测试

1. **公开端点**
   - `/api/health` - 健康检查
   - `/api/config` - 配置信息
   - `/api/auth/status` - 认证状态
   - `/api/research/templates` - 研究模板

2. **认证端点**
   - 用户注册和登录
   - Token 刷新
   - 用户信息获取

3. **仓库管理端点**
   - 仓库列表、创建、获取、删除
   - 仓库重新索引

4. **聊天端点**
   - RAG 查询功能
   - 流式响应（如果实现）

5. **Wiki 端点**
   - Wiki 生成和获取
   - Wiki 导出功能

6. **研究端点**
   - 研究模板管理
   - 研究会话启动和进度跟踪

## 🚀 运行测试

### 运行所有集成测试

```bash
# 在 wikify-web 目录下
cargo test --test auth_permissions_test
cargo test --test api_endpoints_test
```

### 运行特定测试

```bash
# 认证权限测试
cargo test auth_permissions_comprehensive --test auth_permissions_test

# API端点测试
cargo test api_endpoints_comprehensive --test api_endpoints_test
```

### 启用测试日志

```bash
# 查看详细测试输出
TEST_LOG=1 cargo test --test auth_permissions_test
```

## 📊 测试结果解读

### 成功指标

- ✅ 所有测试通过
- ✅ 权限模式正确切换
- ✅ API 端点返回预期状态码
- ✅ 认证流程正常工作

### 常见状态码含义

- **200 OK** - 端点正常工作
- **401 Unauthorized** - 权限控制正确（在 private 模式下预期）
- **404 Not Found** - 资源未找到（使用测试数据时预期）
- **501 Not Implemented** - 功能未实现（开发中的功能）
- **500 Internal Server Error** - 服务器错误（需要调查）

## 🔧 测试配置

### 权限模式

测试支持三种权限模式：

1. **Open 模式** (`"open"`)
   - 所有功能对匿名用户开放
   - 不需要认证
   - 适合公开部署

2. **Private 模式** (`"private"`)
   - 需要用户注册和登录
   - 受保护端点需要认证
   - 适合私有部署

3. **Enterprise 模式** (`"enterprise"`)
   - 企业级功能和权限控制
   - 高级安全特性
   - 适合企业部署

### 测试数据

- 使用内存 SQLite 数据库 (`:memory:`)
- 随机端口避免冲突
- 自动生成测试用户数据
- 每个测试独立的应用实例

## 🐛 故障排除

### 常见问题

1. **端口冲突**
   - 测试使用随机端口，通常不会冲突
   - 如果遇到问题，重新运行测试

2. **数据库错误**
   - 使用内存数据库，每次测试都是全新的
   - 确保 SQLite 功能已启用

3. **网络超时**
   - 测试有 60 秒超时限制
   - 如果应用启动慢，可能需要调整等待时间

4. **依赖问题**
   - 确保所有依赖都已正确安装
   - 运行 `cargo build` 确保编译成功

## 📈 扩展测试

### 添加新测试

1. 在 `helpers.rs` 中添加新的 HTTP 方法封装
2. 创建新的测试文件或在现有文件中添加测试函数
3. 使用 `spawn_app_with_mode()` 创建测试应用
4. 编写断言验证预期行为

### 测试最佳实践

- 每个测试应该独立运行
- 使用描述性的测试名称
- 包含足够的断言验证行为
- 添加适当的错误处理和日志
- 考虑边界情况和错误场景

## 🎯 未来改进

1. **性能测试** - 添加负载和压力测试
2. **安全测试** - 测试 SQL 注入、XSS 等安全问题
3. **并发测试** - 测试多用户并发访问
4. **数据持久化测试** - 测试真实数据库的数据持久化
5. **WebSocket 测试** - 测试实时功能
6. **文件上传测试** - 测试文件处理功能

---

这个测试套件为 Wikify Web 提供了全面的质量保证，确保所有核心功能在不同权限模式下都能正常工作。
