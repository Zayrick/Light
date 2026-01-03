# Usage

## 启动一体化 UI

```bash
.\.venv\Scripts\python main.py
```

UI 内可完成：
- 预设切换与保存
- 输出口配置编辑
- 预览与服务启动/停止

可选：传入 `--config some.json` 作为初始配置载入。

## 仅启动服务（Headless）

默认配置（48x27 单矩阵）：

```bash
.\.venv\Scripts\python main.py --headless
```

指定 JSON 配置（多输出口 / 稀疏矩阵）：

```bash
.\.venv\Scripts\python main.py --headless --config example_config.json
```

## 协议说明（关键点）

- UDP 像素更新的 `index` 是 **全设备物理顺序**（输出口按 `outputs[]` 顺序拼接）
- 设备输出配置通过 `CMD_QUERY_CONFIG (0x14)` 获取，响应为 JSON（可能分片）
- Rust 侧 `led_matrix_udp` 控制器会优先通过该接口获取输出定义；UI 不读取 JSON
