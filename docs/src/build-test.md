# 编译与测试指南

本指南介绍如何 进行 `loong_remote_communication` 的跨平台编译和集成测试。

## 1. 编译

### Release:slow

```bash
docker compose run --rm build-release-x86_64
docker compose run --rm build-release-aarch64
docker compose run --rm build-release-win64
```

### Debug:fast

```bash
docker compose run --rm build-x86_64
docker compose run --rm build-aarch64
docker compose run --rm build-win64
```

编译产物位于 `target/<target-triple>/debug/` 或 `target/<target-triple>/release/` 目录。

## 2. 测试

### 步骤 1: 启动 MQTT Broker

```bash
docker compose up -d mqtt-broker
docker compose ps mqtt-broker
```

### 步骤 2: 设备端：启动 Echo 服务器和 proxyd

**使用 TCP socket：**

```bash
python3 test/py/server.py --addr 0.0.0.0:12345

./target/x86_64-unknown-linux-gnu/debug/proxyd \
  --local-id robot_01 \
  --mqtt-broker mqtt://127.0.0.1:1883 \
  --proxy-addr 127.0.0.1:12345
```

**使用 Unix socket：**

```bash
python3 test/py/server.py --addr unix:///tmp/echo-server.sock

./target/x86_64-unknown-linux-gnu/debug/proxyd \
  --local-id robot_01 \
  --mqtt-broker mqtt://127.0.0.1:1883 \
  --proxy-addr unix:///tmp/echo-server.sock
```

### 步骤 3: 用户端：启动 proxyd 和 Echo 客户端

**使用 TCP socket：**

```bash
./target/x86_64-unknown-linux-gnu/debug/portald \
  --local-id user_01 \
  --remote-id robot_01 \
  --mqtt-broker mqtt://127.0.0.1:1883 \
  --portal-addr 0.0.0.0:54321

python3 test/py/client.py --addr 127.0.0.1:54321
```

**使用 Unix socket：**

```bash
./target/x86_64-unknown-linux-gnu/debug/portald \
  --local-id user_01 \
  --remote-id robot_01 \
  --mqtt-broker mqtt://127.0.0.1:1883 \
  --portal-addr unix:///tmp/echo-client.sock

python3 test/py/client.py --addr unix:///tmp/echo-client.sock
```

### 监控 MQTT 消息

运行以下命令，可以实时查看 MQTT 信令消息：

```sh
# 查看设备的在线状态
mosquitto_sub -h 127.0.0.1 -t 'callee/+/status' -v

# 查看用户的在线状态
mosquitto_sub -h 127.0.0.1 -t 'caller/+/status' -v

# 查看用户向设备端发送的 offer信令
mosquitto_sub -h 127.0.0.1 -t 'callee/+/signal' -v

# 查看设备端向用户返回的 answer信令
mosquitto_sub -h 127.0.0.1 -t 'caller/+/signal' -v
```
