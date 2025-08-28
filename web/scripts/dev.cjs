#!/usr/bin/env node

/**
 * Wikify 前端开发启动脚本
 * 检查环境并启动开发服务器
 */

const fs = require("fs");
const path = require("path");
const { execSync } = require("child_process");

// 颜色输出
const colors = {
  reset: "\x1b[0m",
  bright: "\x1b[1m",
  red: "\x1b[31m",
  green: "\x1b[32m",
  yellow: "\x1b[33m",
  blue: "\x1b[34m",
  magenta: "\x1b[35m",
  cyan: "\x1b[36m",
};

function log(message, color = "reset") {
  console.log(`${colors[color]}${message}${colors.reset}`);
}

function checkNodeVersion() {
  const nodeVersion = process.version;
  const majorVersion = parseInt(nodeVersion.slice(1).split(".")[0]);

  if (majorVersion < 18) {
    log("❌ Node.js version must be >= 18.0.0", "red");
    log(`Current version: ${nodeVersion}`, "yellow");
    process.exit(1);
  }

  log(`✅ Node.js version: ${nodeVersion}`, "green");
}

function checkPackageManager() {
  try {
    const pnpmVersion = execSync("pnpm --version", { encoding: "utf8" }).trim();
    log(`✅ pnpm version: ${pnpmVersion}`, "green");
  } catch (error) {
    log("❌ pnpm is not installed", "red");
    log("💡 Install pnpm: npm install -g pnpm", "yellow");
    process.exit(1);
  }
}

function checkEnvironmentFile() {
  const envPath = path.join(__dirname, "..", ".env.local");
  const envExamplePath = path.join(__dirname, "..", ".env.example");

  if (!fs.existsSync(envPath)) {
    if (fs.existsSync(envExamplePath)) {
      log("⚠️  .env.local not found, copying from .env.example", "yellow");
      fs.copyFileSync(envExamplePath, envPath);
      log("✅ Created .env.local from .env.example", "green");
    } else {
      log("❌ Neither .env.local nor .env.example found", "red");
      process.exit(1);
    }
  } else {
    log("✅ Environment file found", "green");
  }
}

function checkDependencies() {
  const packageJsonPath = path.join(__dirname, "..", "package.json");
  const nodeModulesPath = path.join(__dirname, "..", "node_modules");

  if (!fs.existsSync(nodeModulesPath)) {
    log("⚠️  node_modules not found, installing dependencies...", "yellow");
    try {
      execSync("pnpm install", {
        stdio: "inherit",
        cwd: path.join(__dirname, ".."),
      });
      log("✅ Dependencies installed successfully", "green");
    } catch (error) {
      log("❌ Failed to install dependencies", "red");
      process.exit(1);
    }
  } else {
    log("✅ Dependencies found", "green");
  }
}

function checkBackendConnection() {
  const envPath = path.join(__dirname, "..", ".env.local");
  const envContent = fs.readFileSync(envPath, "utf8");

  const apiUrlMatch = envContent.match(/VITE_API_BASE_URL=(.+)/);
  const wsUrlMatch = envContent.match(/VITE_WS_BASE_URL=(.+)/);

  if (apiUrlMatch) {
    const apiUrl = apiUrlMatch[1].trim();
    log(`🔗 API URL: ${apiUrl}`, "cyan");
  }

  if (wsUrlMatch) {
    const wsUrl = wsUrlMatch[1].trim();
    log(`🔗 WebSocket URL: ${wsUrl}`, "cyan");
  }

  log("⚠️  Make sure the backend server is running!", "yellow");
}

function printWelcomeMessage() {
  log("\n" + "=".repeat(50), "cyan");
  log("🎨 Wikify Frontend Development Server", "bright");
  log("=".repeat(50), "cyan");
  log("");
  log("📚 Documentation: web/README.md", "blue");
  log("🐛 Issues: https://github.com/your-repo/wikify/issues", "blue");
  log(
    "💬 Discussions: https://github.com/your-repo/wikify/discussions",
    "blue"
  );
  log("");
  log("🚀 Starting development server...", "green");
  log("");
}

function startDevServer() {
  try {
    execSync("pnpm run dev", {
      stdio: "inherit",
      cwd: path.join(__dirname, ".."),
    });
  } catch (error) {
    log("❌ Failed to start development server", "red");
    process.exit(1);
  }
}

function main() {
  log("🔍 Checking development environment...", "bright");
  log("");

  // 环境检查
  checkNodeVersion();
  checkPackageManager();
  checkEnvironmentFile();
  checkDependencies();
  checkBackendConnection();

  log("");
  log("✅ Environment check completed!", "green");

  // 显示欢迎信息
  printWelcomeMessage();

  // 启动开发服务器
  startDevServer();
}

// 处理未捕获的异常
process.on("uncaughtException", (error) => {
  log(`❌ Uncaught Exception: ${error.message}`, "red");
  process.exit(1);
});

process.on("unhandledRejection", (reason, promise) => {
  log(`❌ Unhandled Rejection at: ${promise}, reason: ${reason}`, "red");
  process.exit(1);
});

// 处理 Ctrl+C
process.on("SIGINT", () => {
  log("\n👋 Goodbye!", "yellow");
  process.exit(0);
});

// 运行主函数
main();
