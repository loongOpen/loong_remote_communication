[![Documentation](https://img.shields.io/badge/docs-mdbook-brightgreen.svg)](https://loongOpen.github.io/loong_remote_communication/)
[![CI/CD](https://github.com/loongOpen/loong_remote_communication/actions/workflows/mdbook.yml/badge.svg)](https://github.com/loongOpen/loong_remote_communication/actions)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](./LICENSE)

# loong_remote_communication

> 基于 WebRTC 的跨 NAT 远程 RPC 中间件 | 让内网服务像公网服务一样易用

## 🎯 为什么选择 loong_remote_communication？

**传统方案的问题：** 内网设备无法直接暴露公网服务，VPN 配置复杂，WebSocket 中转需要维护中心化服务器。

**我们的解决方案：** 基于 WebRTC DataChannel 构建透明 TCP 隧道，通过 MQTT 信令实现 NAT 穿透，**零配置**将内网服务代理到本地，体验与局域网完全一致。

## ✨ 核心特性

- 🚀 **RaaS 架构** - 将远端 TCP 服务（gRPC/REST）代理到本地，公网访问与局域网完全一致
- ⚡ **高性能** - Rust + Tokio 异步运行时，协程并发，极低转发开销
- 🛡️ **内存安全** - Rust 所有权系统，编译期保证并发安全
- 📡 **极简部署** - MQTT 信令通道，无需 WebSocket 服务器，结合 EMQX 可快速实现统一鉴权
- 🖥️ **跨平台** - Linux (x86_64/aarch64) 和 Windows
- 🔌 **协议无关** - 支持所有基于 TCP 的应用层协议

## 🚀 快速开始

### 安装

```bash
git clone https://github.com/loongOpen/loong_remote_communication.git
cd loong_remote_communication
cargo build --release
```

### 3 步上手

**1. 启动设备端代理** (机器人侧)

```bash
./proxyd \
  --local-id robot_1 \
  --proxy-addr 127.0.0.1:12345 \
  --mqtt-broker mqtt://<public_ip>:1883
```

**2. 启动用户端入口** (控制端)

```bash
./portald \
  --local-id user_1 \
  --remote-id robot_1 \
  --portal-addr 127.0.0.1:54321 \
  --mqtt-broker mqtt://<public_ip>:1883
```

**3. 连接成功！** 访问 `127.0.0.1:54321` 即等同于访问机器人端的 `127.0.0.1:12345`

## 🎯 应用场景

- 🕹️ **远程遥操作机器人** - 跨公网实时控制，延迟低至毫秒级
- 📊 **IoT 数据采集** - 统一接入内网传感器，无需复杂网络配置
- ☁️ **云平台设备控制** - 将机器人集群抽象为标准化服务接口
- 🏭 **边缘计算服务暴露** - 内网边缘节点服务直接对外提供服务

## 🤝 贡献

欢迎贡献代码、报告问题或提出建议！

- 📝 [提交 Issue](https://github.com/loongOpen/loong_remote_communication/issues)
- 🔀 [提交 Pull Request](https://github.com/loongOpen/loong_remote_communication/pulls)
- 📖 [完整文档](https://loongOpen.github.io/loong_remote_communication/)

## 📄 许可证

本项目采用 [MIT 或 Apache-2.0](./LICENSE) 双许可证。
