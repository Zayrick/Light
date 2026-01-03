# Usage

## 启动虚拟设备

默认配置（48x27 单矩阵）：

```bash
.\.venv\Scripts\python virtual_led_device.py
```

指定 JSON 配置（多输出口 / 稀疏矩阵）：

```bash
.\.venv\Scripts\python virtual_led_device.py --config example_config.json
```

## 启动 JSON 生成器（GUI）

```bash
.\.venv\Scripts\python -m pip install -r requirements-gui.txt
.\.venv\Scripts\python config_generator_gui.py
```

## 协议说明（关键点）

- UDP 像素更新的 `index` 是 **全设备物理顺序**（输出口按 `outputs[]` 顺序拼接）
- 设备输出配置通过 `CMD_QUERY_CONFIG (0x14)` 获取，响应为 JSON（可能分片）
- Rust 侧 `led_matrix_udp` 控制器会优先通过该接口获取输出定义；UI 不读取 JSON

