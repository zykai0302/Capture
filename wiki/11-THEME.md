# 11 — 主题系统

## 1 CSS 变量 (`src/styles/variables.css`)

```css
:root {
  /* 背景层级 (由深到浅) */
  --bg-deep: #0a0e17;
  --bg-primary: #0f1523;
  --bg-secondary: #151d2e;
  --bg-tertiary: #1a2438;
  --bg-card: #141c2d;
  --bg-hover: #1e2a42;

  /* 边框 */
  --border: #1e2d45;
  --border-active: #2a4a6b;

  /* 文字层级 */
  --text-primary: #e8edf5;
  --text-secondary: #8b9dc3;
  --text-muted: #5a6e8f;

  /* 强调色 */
  --accent-cyan: #00d4ff;
  --accent-cyan-dim: #00a8cc;
  --accent-cyan-glow: rgba(0, 212, 255, 0.15);

  --accent-green: #00ff88;
  --accent-green-dim: #00cc6a;
  --accent-green-glow: rgba(0, 255, 136, 0.15);

  --accent-orange: #ff8c00;
  --accent-orange-glow: rgba(255, 140, 0, 0.15);

  --accent-red: #ff3860;
  --accent-red-glow: rgba(255, 56, 96, 0.15);

  --accent-purple: #b44dff;

  /* 字体 */
  --font-display: 'Outfit', -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif;
  --font-mono: 'JetBrains Mono', 'Cascadia Code', 'Fira Code', monospace;

  /* 圆角 */
  --radius-sm: 6px;
  --radius-md: 10px;
  --radius-lg: 14px;
}
```

## 2 全局样式 (`src/styles/global.css`)

```css
@import './variables.css';

* { margin: 0; padding: 0; box-sizing: border-box; }

body {
  font-family: var(--font-display);
  background: var(--bg-deep);
  color: var(--text-primary);
  overflow: hidden;
  height: 100vh;
  user-select: none;
}

/* 噪点纹理叠加 */
body::before {
  content: '';
  position: fixed;
  inset: 0;
  background: url("data:image/svg+xml,...feTurbulence...");  /* SVG 噪点 */
  pointer-events: none;
  z-index: 9999;
}

/* 滚动条 */
::-webkit-scrollbar { width: 5px; }
::-webkit-scrollbar-track { background: transparent; }
::-webkit-scrollbar-thumb { background: var(--border); border-radius: 3px; }
::-webkit-scrollbar-thumb:hover { background: var(--border-active); }

/* 重置 */
button { font-family: var(--font-display); cursor: pointer; border: none; background: none; color: inherit; outline: none; }
input, select { font-family: var(--font-mono); outline: none; }
```

## 3 动画定义

```css
@keyframes pulse-dot {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.5; }
}

@keyframes blink-live {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.4; }
}

@keyframes slideInUp {
  from { transform: translateY(10px); opacity: 0; }
  to { transform: translateY(0); opacity: 1; }
}

@keyframes flow-particle {
  0% { left: 4px; opacity: 0; }
  20% { opacity: 1; }
  80% { opacity: 1; }
  100% { left: calc(100% - 8px); opacity: 0; }
}
```

| 动画 | 用途 | 时长 | 循环 |
|------|------|------|------|
| `pulse-dot` | 状态指示灯脉冲 | 2s | infinite |
| `blink-live` | "LIVE" 徽章闪烁 | 1.5s | infinite |
| `slideInUp` | SourceItem 入场动画 | 0.3s | 一次 |
| `flow-particle` | PipelineVisual 箭头粒子 | 1.5s | infinite |

## 4 色彩使用规范

| 场景 | 颜色 | 变量 |
|------|------|------|
| 选中项 | 青色 | `--accent-cyan` |
| 推流中 / 运行中 | 绿色 | `--accent-green` |
| 警告 / CPU 编码 | 橙色 | `--accent-orange` |
| 错误 / 停止 | 红色 | `--accent-red` |
| RTSP 推流节点 | 紫色 | `--accent-purple` |
| 编码器标签(GPU) | 绿色 | `--accent-green` |
| 编码器标签(CPU) | 橙色 | `--accent-orange` |
| 状态指示灯(运行) | 绿色发光 | `--accent-green` + `box-shadow` |
| 状态指示灯(空闲) | 灰色 | `--text-muted` |
| 状态指示灯(错误) | 红色发光 | `--accent-red` + `box-shadow` |
