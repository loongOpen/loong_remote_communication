#!/usr/bin/env python3
"""
gRPC hello 服务端示例

使用方法：
    1. 生成 gRPC 代码：
       python3 -m grpc_tools.protoc \
         -I examples/protos \
         --python_out=examples/generated \
         --grpc_python_out=examples/generated \
         examples/protos/hello.proto

    2. 启动 proxyd：
       ./proxyd --local-id robot_1 --proxy-addr 127.0.0.1:50051 --mqtt-broker mqtt://localhost:1883

    3. 启动此服务：
       python3 examples/grpc_server.py --addr 127.0.0.1:50051
       或使用 Unix socket：
       python3 examples/grpc_server.py --addr unix:///tmp/hello.sock
"""
import grpc
from concurrent import futures
import argparse
import os
import sys

sys.path.insert(0, os.path.join(os.path.dirname(__file__), "generated"))


class Greeter:
    def SayHello(self, request, context):
        import hello_pb2 as pb

        return pb.HelloReply(message=f"Hello {request.name}!")


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--addr",
        type=str,
        default="0.0.0.0:50051",
        help="Listen address (host:port or unix:///path/to/socket)",
    )
    args = parser.parse_args()

    try:
        import hello_pb2_grpc as pb_grpc
    except ImportError:
        print("❌ 请先生成 gRPC 代码")
        sys.exit(1)

    server = grpc.server(futures.ThreadPoolExecutor(max_workers=10))
    pb_grpc.add_GreeterServicer_to_server(Greeter(), server)

    if args.addr.startswith("unix://"):
        socket_path = args.addr[7:]
        if os.path.exists(socket_path):
            os.unlink(socket_path)
        server.add_insecure_port(f"unix:{socket_path}")
    else:
        server.add_insecure_port(args.addr)

    server.start()
    server.wait_for_termination()


if __name__ == "__main__":
    main()
