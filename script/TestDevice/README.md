# TestDevice (Virtual LED Device)

这是一个用于 **Light** 项目的虚拟硬件（UDP）：

- 支持 **Device / Output / Segment** 的后端模型（输出口信息通过协议查询返回）
- 支持多输出口：`Matrix` / `Linear` / `Single`
- 支持 **稀疏矩阵**（`Matrix.matrix.map` 允许 `null`）

> 重要：主应用不应读取这个 JSON。  
> JSON 仅用于启动虚拟设备；应用侧通过 `led_matrix_udp` 控制器的接口获取输出配置。

## 快速开始

### 1) 启动虚拟设备

在 `script/TestDevice` 下创建虚拟环境并安装依赖：

```bash
python -m venv .venv
.\.venv\Scripts\python -m pip install -r requirements.txt
```

启动（默认 48x27 单矩阵）：

```bash
.\.venv\Scripts\python virtual_led_device.py
```

使用 JSON 启动（多输出口 / 稀疏矩阵）：

```bash
.\.venv\Scripts\python virtual_led_device.py --config example_config.json
```

### 2) 用 GUI 生成 JSON

安装 GUI 依赖：

```bash
.\.venv\Scripts\python -m pip install -r requirements-gui.txt
```

启动生成器：

```bash
.\.venv\Scripts\python config_generator_gui.py
```

## 配置格式（概要）

顶层：
- `schema_version`: 配置版本（当前为 `1`）
- `device_name`: mDNS + 设备名
- `udp_port`: UDP 监听端口
- `pixel_size`: 虚拟窗口缩放倍率
- `outputs`: 输出口数组（顺序决定“物理顺序”拼接顺序）

输出口（Matrix）：
- `output_type`: `"Matrix"`
- `matrix.width` / `matrix.height`
- `matrix.map`: 长度 `width*height` 的数组，元素为 **整数索引** 或 `null`
  - 整数必须覆盖 `0..leds_count-1` 且不重复

输出口（Linear）：
- `output_type`: `"Linear"`
- `length` / `leds_count`: 物理 LED 数量

