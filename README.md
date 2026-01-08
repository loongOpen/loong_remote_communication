[![Documentation](https://img.shields.io/badge/docs-mdbook-brightgreen.svg)](https://loongOpen.github.io/loong_remote_communication/)
[![CI/CD](https://github.com/loongOpen/loong_remote_communication/actions/workflows/mdbook.yml/badge.svg)](https://github.com/loongOpen/loong_remote_communication/actions)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](./LICENSE)

# loong_remote_communication

> 基于 WebRTC 的跨 NAT 远程 RPC 中间件 | 让内网服务像公网服务一样易用

## 🎯 为什么选择 loong_remote_communication？

**传统方案的问题：** 内网设备无法直接暴露公网服务，VPN 配置复杂，WebSocket 中转需要维护中心化服务器。

**我们的解决方案：** 基于 WebRTC DataChannel 构建透明 TCP 隧道，通过 MQTT 信令实现 NAT 穿透，**零配置**将内网服务代理到本地，体验与局域网完全一致。

## ✨ 核心特性

- 🚀 **远程直连** - 将远端内网 TCP 服务代理至本地，使公网访问体验与局域网完全一致。两端仅需运行轻量可执行文件即可实现跨网段服务直连。
- 📡 **极简部署** - 仅需公网部署 MQTT 用于信令交换与 TURN 用于中继，无需开发维护有状态的 WS 信令服务或配置复杂的代理规则与访问控制列表。
- 🛡️ **高效安全** - 充分利用 Rust 现代异步生态，采用基于 MPSC 的事件驱动模型替代传统回调，规避回调地狱的同时降低系统耦合度，架构比 C++ 更稳定易迭代。
- 🔐 **安全管控** - 将复杂的权限管理转化为对 MQTT 话题的读写权限管理，基于 EMQX 与 Authing 可快速实现生产级的访问控制。
- 💻 **跨端通用** - 支持 Linux (x86/ARM) 与 Windows 平台和所有基于 TCP 的服务（gRPC/REST），现有的 C/S 端应用仅需修改一行连接地址即可快速接入。

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
