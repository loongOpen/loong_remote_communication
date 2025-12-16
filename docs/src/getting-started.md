# 🚀 快速开始

本指南将协助您快速搭建 `remote_rpc` 运行环境。系统依赖公网基础设施进行信令交换与 NAT 穿透。

## 📦 1. 基础设施准备

为确保 WebRTC 在复杂网络环境（如对称 NAT）下正常工作，需在公网服务器上部署：

### 必需服务

1. **📡 MQTT Broker**（信令交换）

   - 推荐：Eclipse Mosquitto 或 EMQX
   - [Mosquitto Docker 指南](https://hub.docker.com/_/eclipse-mosquitto)
   - [EMQX Docker 指南](https://hub.docker.com/_/emqx)

2. **🔀 TURN Server**（NAT 穿透中继）
   - 推荐：Coturn
   - [Coturn Docker 指南](https://hub.docker.com/r/coturn/coturn)

### 可选服务

3. **🔍 STUN Server**（NAT 检测）
   - 可使用公共服务器：`stun.l.google.com:19302` 或 `stun.cloudflare.com:3478`

## 🚀 2. 快速运行

**场景**：从笔记本电脑访问内网机器人上的 TCP 服务（端口 `12345`）

### Step 1: 🤖 启动设备端代理 (Proxy)

```bash
./proxyd \
  --local-id robot_1 \
  --proxy-addr 127.0.0.1:12345 \
  --mqtt-broker mqtt://<public_ip>:1883 \
  --peer-turn turn:user:pass@host:port
```

### Step 2: 🖥️ 启动用户端入口 (Portal)

```bash
./portald \
  --local-id user_1 \
  --remote-id robot_1 \
  --portal-addr 127.0.0.1:54321 \
  --mqtt-broker mqtt://<public_ip>:1883 \
  --peer-turn turn:user:pass@host:port
```

### Step 3: ✅ 验证连接

当 `portald` 提示连接成功后，访问 `127.0.0.1:54321` 即等同于访问机器人端的 `127.0.0.1:12345`。

> 💡 此时 gRPC Client 可以直接连接 127.0.0.1:54321 进行操作。

---

## 📖 3. 命令行参数详解 (CLI Reference)

### 🤖 proxyd (Robot Side)

proxyd 负责驻守在设备端，等待来自 Portal 的连接请求，并桥接本地 TCP 服务。

```bash
$ ./proxyd -h
Usage: proxyd [OPTIONS]

Options:
  -l, --local-id        <LOCAL_ID>     本地 ID [必须]
  -p, --proxy-addr      <PROXY_ADDR>   需要被代理的目标服务地址 [必须] (例如: 127.0.0.1:9000 或 unix:///tmp/sock)
  -b, --mqtt-broker     <BROKER>       MQTT Broker 地址 [默认: mqtt://localhost:1883]
      --mqtt-username   <USERNAME>     MQTT 用户名 [可选]
      --mqtt-password   <PASSWORD>     MQTT 密码   [可选]
      --peer-stun       <STUN>         STUN 服务器地址 (可指定多个) [默认: stun:stun.l.google.com:19302]
      --peer-turn       <TURN>         TURN 服务器地址 (可指定多个) (格式: turn:user:pass@host:port)
      --online-timeout  <SEC>          等待对端上线超时时间 [默认: 5]
      --connect-timeout <SEC>          WebRTC 建连超时时间 [默认: 5]
  -h, --help                           显示帮助信息
```

### 🖥️ portald (User Side)

portald 运行在控制端，负责开启本地入口端口，并寻找远程 Peer 建立隧道。

```bash
$ ./portald -h
Usage: portald [OPTIONS]

Options:
  -l, --local-id        <LOCAL_ID>     本地 ID [必须]
  -r, --remote-id       <REMOTE_ID>    目标设备的 ID [必须]
  -p, --portal-addr     <PORTAL_ADDR>  代理到本地的地址 [必须] (例如: 127.0.0.1:9000 或 unix:///tmp/sock)
  -b, --mqtt-broker     <BROKER>       MQTT Broker 地址 [默认: mqtt://localhost:1883]
      --mqtt-username   <USERNAME>     MQTT 用户名 [可选]
      --mqtt-password   <PASSWORD>     MQTT 密码   [可选]
      --peer-stun       <STUN>         STUN 服务器地址 (可指定多个) [默认: stun:stun.l.google.com:19302]
      --peer-turn       <TURN>         TURN 服务器地址 (可指定多个) (格式: turn:user:pass@host:port)
      --online-timeout  <SEC>          等待对端上线超时时间 [默认: 5]
      --connect-timeout <SEC>          WebRTC 建连超时时间 [默认: 5]
  -h, --help                           显示帮助信息
```
