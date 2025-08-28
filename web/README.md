# 🎨 Wikify 前端

基于 React + TypeScript + Vite 构建的现代化前端应用，为 Wikify 项目提供用户界面。

## ✨ 特性

- 🚀 **现代化技术栈**: React 18 + TypeScript + Vite
- 🎨 **优美的 UI**: shadcn/ui + Tailwind CSS
- 🔄 **实时通信**: WebSocket 支持实时聊天
- 📊 **状态管理**: Zustand + React Query
- 🌙 **主题切换**: 支持明暗主题
- 📱 **响应式设计**: 支持桌面和移动端
- 🔒 **类型安全**: 完整的 TypeScript 支持

## 🛠️ 技术栈

### 核心框架
- **React 18** - 用户界面库
- **TypeScript** - 类型安全的 JavaScript
- **Vite** - 快速的构建工具

### 状态管理
- **Zustand** - 轻量级状态管理
- **React Query** - 服务端状态管理

### UI 组件
- **shadcn/ui** - 高质量组件库
- **Tailwind CSS** - 实用优先的 CSS 框架
- **Lucide React** - 美观的图标库

### 网络通信
- **Axios** - HTTP 客户端
- **WebSocket** - 实时通信

### 开发工具
- **ESLint** - 代码检查
- **Prettier** - 代码格式化
- **Vitest** - 单元测试
- **Playwright** - E2E 测试

## 🚀 快速开始

### 环境要求

- Node.js >= 18.0.0
- npm >= 9.0.0

### 安装依赖

```bash
cd web
npm install
```

### 环境配置

复制环境变量配置文件：

```bash
cp .env.example .env.local
```

根据实际情况修改 `.env.local` 中的配置：

```env
# API 配置
VITE_API_BASE_URL=http://localhost:8080/api
VITE_WS_BASE_URL=ws://localhost:8080/ws

# 开发配置
VITE_DEV_MODE=true
VITE_DEBUG_WEBSOCKET=true
VITE_DEBUG_API=true
```

### 启动开发服务器

```bash
npm run dev
```

应用将在 http://localhost:5173 启动。

### 构建生产版本

```bash
npm run build
```

### 预览生产版本

```bash
npm run preview
```

## 📁 项目结构

```
web/
├── public/                 # 静态资源
├── src/
│   ├── components/         # React 组件
│   │   ├── ui/            # shadcn/ui 组件
│   │   ├── RepositoryManager.tsx
│   │   ├── ChatInterface.tsx
│   │   └── ThemeToggle.tsx
│   ├── hooks/             # 自定义 hooks
│   │   ├── use-api.ts     # API hooks
│   │   ├── use-websocket.ts # WebSocket hooks
│   │   └── use-toast.ts   # Toast hooks
│   ├── lib/               # 工具库
│   │   ├── api-client.ts  # API 客户端
│   │   ├── websocket-client.ts # WebSocket 客户端
│   │   ├── constants.ts   # 常量定义
│   │   └── utils.ts       # 工具函数
│   ├── pages/             # 页面组件
│   │   ├── Index.tsx      # 首页
│   │   └── NotFound.tsx   # 404 页面
│   ├── store/             # 状态管理
│   │   ├── app-store.ts   # 全局状态
│   │   └── chat-store.ts  # 聊天状态
│   ├── types/             # 类型定义
│   │   ├── api.ts         # API 类型
│   │   ├── ui.ts          # UI 类型
│   │   └── websocket.ts   # WebSocket 类型
│   ├── utils/             # 工具函数
│   │   ├── formatters.ts  # 格式化工具
│   │   └── validators.ts  # 验证工具
│   ├── App.tsx            # 应用根组件
│   ├── main.tsx           # 应用入口
│   └── index.css          # 全局样式
├── .env.example           # 环境变量示例
├── .env.local             # 本地环境变量
├── package.json           # 项目配置
├── tailwind.config.ts     # Tailwind 配置
├── tsconfig.json          # TypeScript 配置
└── vite.config.ts         # Vite 配置
```

## 🔧 开发指南

### 添加新组件

1. 在 `src/components/` 目录下创建组件文件
2. 使用 TypeScript 定义组件 props
3. 遵循现有的代码风格和命名约定

```tsx
// src/components/MyComponent.tsx
import { ComponentProps } from '@/types/ui';

interface MyComponentProps extends ComponentProps {
  title: string;
  onAction: () => void;
}

export function MyComponent({ title, onAction, className }: MyComponentProps) {
  return (
    <div className={className}>
      <h2>{title}</h2>
      <button onClick={onAction}>Action</button>
    </div>
  );
}
```

### 添加新的 API 调用

1. 在 `src/types/api.ts` 中定义数据类型
2. 在 `src/lib/api-client.ts` 中添加 API 方法
3. 在 `src/hooks/use-api.ts` 中创建 React Query hooks

```typescript
// 1. 定义类型
export interface NewDataType {
  id: string;
  name: string;
}

// 2. 添加 API 方法
async getNewData(): Promise<NewDataType[]> {
  return this.request<NewDataType[]>({
    method: 'GET',
    url: '/new-data',
  });
}

// 3. 创建 hook
export function useNewData() {
  return useQuery(
    createQueryConfig(
      ['newData'],
      () => apiClient.getNewData()
    )
  );
}
```

### 状态管理

使用 Zustand 管理客户端状态：

```typescript
// src/store/my-store.ts
import { create } from 'zustand';

interface MyState {
  count: number;
  increment: () => void;
  decrement: () => void;
}

export const useMyStore = create<MyState>((set) => ({
  count: 0,
  increment: () => set((state) => ({ count: state.count + 1 })),
  decrement: () => set((state) => ({ count: state.count - 1 })),
}));
```

### WebSocket 使用

使用自定义 WebSocket hooks：

```typescript
import { useChatWebSocket } from '@/hooks/use-websocket';

function ChatComponent() {
  const { sendMessage, isConnected } = useChatWebSocket(sessionId);
  
  const handleSend = () => {
    if (isConnected) {
      sendMessage('Hello, world!');
    }
  };
  
  return (
    <button onClick={handleSend} disabled={!isConnected}>
      Send Message
    </button>
  );
}
```

## 🧪 测试

### 运行单元测试

```bash
npm run test
```

### 运行 E2E 测试

```bash
npm run test:e2e
```

### 测试覆盖率

```bash
npm run test:coverage
```

## 📦 构建和部署

### 构建优化

- 代码分割和懒加载
- 图片优化和压缩
- CSS 和 JS 压缩
- Tree shaking

### 部署选项

1. **静态部署**: Vercel, Netlify, GitHub Pages
2. **容器部署**: Docker + Nginx
3. **CDN 部署**: AWS CloudFront, Cloudflare

### Docker 部署

```dockerfile
FROM node:18-alpine as builder
WORKDIR /app
COPY package*.json ./
RUN npm ci
COPY . .
RUN npm run build

FROM nginx:alpine
COPY --from=builder /app/dist /usr/share/nginx/html
COPY nginx.conf /etc/nginx/nginx.conf
EXPOSE 80
CMD ["nginx", "-g", "daemon off;"]
```

## 🔍 调试

### 开发工具

- React Developer Tools
- Redux DevTools (for Zustand)
- Network 面板查看 API 请求
- WebSocket 连接状态监控

### 日志记录

```typescript
// 开发环境下启用详细日志
if (import.meta.env.DEV) {
  console.log('Debug info:', data);
}
```

## 🤝 贡献指南

1. Fork 项目
2. 创建功能分支 (`git checkout -b feature/amazing-feature`)
3. 提交更改 (`git commit -m 'Add some amazing feature'`)
4. 推送到分支 (`git push origin feature/amazing-feature`)
5. 创建 Pull Request

### 代码规范

- 使用 TypeScript 严格模式
- 遵循 ESLint 规则
- 使用 Prettier 格式化代码
- 编写单元测试
- 添加适当的注释

## 📄 许可证

本项目采用 MIT 许可证 - 查看 [LICENSE](../LICENSE) 文件了解详情。

## 🆘 支持

如果遇到问题或有疑问，请：

1. 查看 [Issues](https://github.com/your-repo/wikify/issues)
2. 创建新的 Issue
3. 查看文档和 FAQ

---

**Happy Coding! 🚀**
