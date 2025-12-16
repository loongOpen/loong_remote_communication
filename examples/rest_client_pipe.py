#!/usr/bin/env python3
"""
REST 客户端示例 - 通过 Portal Hub 访问远端 hello 服务

使用方法：
    1. 运行客户端（INET 类型）：
       python3 examples/rest_client_pipe.py \
         --hub-url http://127.0.0.1:3000 \
         --robot-id robot_1 \
         --service-name rest_service \
         --portal-type INET \
         --inet-port 8000
       
       或使用 UNIX socket：
       python3 examples/rest_client_pipe.py \
         --hub-url http://127.0.0.1:3000 \
         --robot-id robot_1 \
         --service-name rest_service \
         --portal-type UNIX \
         --unix-file /tmp/rest.sock
"""
import requests
import argparse
import sys


def create_portal(hub_url, robot_id, service_name, portal_type="INET", inet_port=None, unix_file=None, user_id=None):
    """创建 Portal 并返回服务地址

    Args:
        portal_type: "INET" 或 "UNIX"
        inet_port: 仅当 portal_type="INET" 时有效
        unix_file: 仅当 portal_type="UNIX" 时有效
    """
    url = f"{hub_url}/portal"
    payload = {
        "robot_id": robot_id,
        "service_name": service_name,
        "portal_type": portal_type.lower(),
    }

    if portal_type == "UNIX" and unix_file:
        payload["unix_file"] = unix_file
    elif portal_type == "INET" and inet_port:
        payload["inet_port"] = inet_port

    if user_id:
        payload["user_id"] = user_id

    try:
        response = requests.post(url, json=payload, timeout=10)
        response.raise_for_status()
        return response.json().get("uri")
    except Exception as e:
        print(f"❌ 创建 Portal 失败: {e}")
        sys.exit(1)


def call_hello_service(portal_uri, name):
    """调用远端 hello 服务"""
    if portal_uri.startswith("unix://"):
        print("⚠️  Unix socket 需要特殊处理，本示例仅支持 TCP")
        return

    if portal_uri.startswith("0.0.0.0:"):
        portal_addr = "127.0.0.1:" + portal_uri.split(":")[1]
    else:
        portal_addr = portal_uri

    try:
        response = requests.post(f"http://{portal_addr}/", json={"name": name}, timeout=5)
        response.raise_for_status()
        print(f"✅ 响应: {response.json().get('message')}")
    except Exception as e:
        print(f"❌ 调用服务失败: {e}")
        sys.exit(1)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--hub-url", default="http://127.0.0.1:3000", help="Portal Hub 地址")
    parser.add_argument("--robot-id", required=True, help="设备 ID")
    parser.add_argument("--service-name", required=True, help="服务名称")
    parser.add_argument("--portal-type", choices=["INET", "UNIX"], default="INET", help="Portal 类型")
    parser.add_argument("--inet-port", help="TCP 端口（仅 portal-type=INET 时有效）")
    parser.add_argument("--unix-file", help="Unix socket 路径（仅 portal-type=UNIX 时有效）")
    parser.add_argument("--user-id", help="用户 ID")

    args = parser.parse_args()

    portal_addr = create_portal(
        args.hub_url,
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
