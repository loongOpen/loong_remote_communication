#!/usr/bin/env python3
"""
REST hello 服务端示例

使用方法：
    1. 启动 proxyd：
       ./proxyd --local-id robot_1 --proxy-addr 127.0.0.1:8000 --mqtt-broker mqtt://localhost:1883

    2. 启动此服务：
       python3 examples/rest_server.py --addr 127.0.0.1:8000
       或使用 Unix socket：
       python3 examples/rest_server.py --addr unix:///tmp/rest.sock
"""
from http.server import HTTPServer, BaseHTTPRequestHandler
import argparse
import os
import json


class HelloHandler(BaseHTTPRequestHandler):
    def do_GET(self):
        response_data = {"message": f"Hello from REST server!"}
        self.send_response(200)
        self.send_header("Content-Type", "application/json")
        self.end_headers()
        self.wfile.write(json.dumps(response_data).encode())

    def do_POST(self):
        content_length = int(self.headers.get("Content-Length", 0))
        body = self.rfile.read(content_length) if content_length > 0 else b""
        request_data = json.loads(body.decode()) if body else {}

        response_data = {"message": f"Hello {request_data.get('name', 'World')}!"}
        self.send_response(200)
        self.send_header("Content-Type", "application/json")
        self.end_headers()
        self.wfile.write(json.dumps(response_data).encode())


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--addr",
        type=str,
        default="0.0.0.0:8000",
        help="Listen address (host:port or unix:///path/to/socket)",
    )
    args = parser.parse_args()

    if args.addr.startswith("unix://"):
        print("⚠️  Unix socket 需要自定义 HTTP 服务器实现，本示例仅支持 TCP")
        return

    parts = args.addr.rsplit(":", 1)
    if len(parts) != 2:
        print(f"❌ Invalid address format: {args.addr}")
        return

    host, port = parts[0], int(parts[1])
    server = HTTPServer((host, port), HelloHandler)
    server.serve_forever()


if __name__ == "__main__":
    main()
