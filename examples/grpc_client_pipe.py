#!/usr/bin/env python3
"""
gRPC 客户端示例 - 通过 Portal Hub 访问远端 hello 服务

使用方法：
    1. 生成 gRPC Python 代码：
       python3 -m grpc_tools.protoc \
         -I crates/grpc/proto \
         -I examples/protos \
         --python_out=examples/generated \
         --grpc_python_out=examples/generated \
         hello.proto

       python3 -m grpc_tools.protoc \
         -I crates/grpc/proto \
         -I examples/protos \
         --python_out=examples/generated \
         --grpc_python_out=examples/generated \
         lrc_user_rpc.proto


    2. 运行客户端（INET 类型）：
       python3 examples/grpc_client_pipe.py \
         --hub-addr [::1]:50051 \
         --robot-id robot_1 \
         --service-name hello_service \
         --portal-type INET \
         --inet-port 50051 \
         --name "World"
       
       或使用 UNIX socket：
       python3 examples/grpc_client_pipe.py \
         --hub-addr [::1]:50051 \
         --robot-id robot_1 \
         --service-name hello_service \
         --portal-type UNIX \
         --unix-file /tmp/hello.sock \
         --name "World"
"""
import grpc
import argparse
import time
import sys
import os

sys.path.insert(0, os.path.join(os.path.dirname(__file__), "generated"))


def create_portal(hub_addr, robot_id, service_name, portal_type="INET", inet_port=None, unix_file=None, user_id=None):
    """创建 Portal 并返回服务地址

    Args:
        portal_type: "INET" 或 "UNIX"
        inet_port: 仅当 portal_type="INET" 时有效
        unix_file: 仅当 portal_type="UNIX" 时有效
    """
    try:
        import lrc_user_rpc_pb2 as pb
        import lrc_user_rpc_pb2_grpc as pb_grpc

        channel = grpc.insecure_channel(hub_addr)
        stub = pb_grpc.PortalLauncherStub(channel)

        config = pb.Config()
        config.robot_id = robot_id
        config.service_name = service_name

        if portal_type == "UNIX":
            config.type = pb.PortalType.UNIX
            if unix_file:
                config.unix_file = unix_file
        else:
            config.type = pb.PortalType.INET
            if inet_port:
                config.inet_port = inet_port

        if user_id:
            config.user_id = user_id

        response = stub.CreatePortal(config)
        channel.close()
        return response.uri
    except ImportError:
        print("❌ 请先生成 gRPC 代码")
        sys.exit(1)
    except Exception as e:
        print(f"❌ 创建 Portal 失败: {e}")
        sys.exit(1)


def call_hello_service(portal_uri, name):
    """调用远端 hello 服务"""
    try:
        import hello_pb2 as pb
        import hello_pb2_grpc as pb_grpc

        # 处理 Unix socket
        if portal_uri.startswith("unix://"):
            portal_addr = "unix:" + portal_uri[7:]  # 移除 "unix://" 前缀
        # 处理 TCP socket：将 "0.0.0.0:port" 转换为 "127.0.0.1:port"
        elif portal_uri.startswith("0.0.0.0:"):
            portal_addr = "127.0.0.1:" + portal_uri.split(":")[1]
        else:
            portal_addr = portal_uri

        channel = grpc.insecure_channel(portal_addr)
        stub = pb_grpc.GreeterStub(channel)

        request = pb.HelloRequest(name=name)
        response = stub.SayHello(request)

        print(f"✅ 响应: {response.message}")
        channel.close()
    except ImportError:
        print("❌ 请先生成 hello.proto 的 gRPC 代码")
        sys.exit(1)
    except Exception as e:
        print(f"❌ 调用服务失败: {e}")
        sys.exit(1)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--hub-addr", default="[::1]:50051", help="Portal Hub 地址")
    parser.add_argument("--robot-id", required=True, help="设备 ID")
    parser.add_argument("--service-name", required=True, help="服务名称")
    parser.add_argument("--portal-type", choices=["INET", "UNIX"], default="INET", help="Portal 类型")
    parser.add_argument("--inet-port", help="TCP 端口（仅 portal-type=INET 时有效）")
    parser.add_argument("--unix-file", help="Unix socket 路径（仅 portal-type=UNIX 时有效）")
    parser.add_argument("--user-id", help="用户 ID")

    args = parser.parse_args()

    portal_addr = create_portal(
        args.hub_addr,
        args.robot_id,
        args.service_name,
        args.portal_type,
        args.inet_port,
        args.unix_file,
        args.user_id,
    )
    print(f"✅ Portal 地址: {portal_addr}")

    call_hello_service(portal_addr, "Hello World")


if __name__ == "__main__":
    main()
