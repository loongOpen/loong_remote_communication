import socket
import threading
import argparse
import os


def handle_client(conn, addr):
    print(f"Connected by {addr}")
    with conn:
        while True:
            data = conn.recv(1024)
            if not data:
                break
            print(f"Received from {addr}: {data.decode()}")
            # 回显数据
            conn.sendall(data)
    print(f"Connection closed: {addr}")


def main():
    parser = argparse.ArgumentParser(description="TCP Echo Server")
    parser.add_argument(
        "--addr",
        type=str,
        default="0.0.0.0:12345",
        help="Listen address (host:port or unix:///path/to/socket, default: 0.0.0.0:12345)",
    )
    args = parser.parse_args()

    if args.addr.startswith("unix://"):
        # Unix socket
        socket_path = args.addr[7:]  # Remove "unix://" prefix
        # Remove existing socket file if it exists
        if os.path.exists(socket_path):
            os.unlink(socket_path)
        s = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        s.bind(socket_path)
        s.listen()
        print(f"Server listening on Unix socket: {socket_path}")

        while True:
            conn, addr = s.accept()
            # 为每个客户端启动线程
            threading.Thread(target=handle_client, args=(conn, addr), daemon=True).start()
    else:
        # TCP socket
        parts = args.addr.rsplit(":", 1)
        if len(parts) != 2:
            print(f"Invalid address format: {args.addr}. Expected host:port or unix:///path")
            return
        host, port = parts[0], int(parts[1])
        s = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        s.bind((host, port))
        s.listen()
        print(f"Server listening on {host}:{port}")

        while True:
            conn, addr = s.accept()
            # 为每个客户端启动线程
            threading.Thread(target=handle_client, args=(conn, addr), daemon=True).start()


if __name__ == "__main__":
    main()
