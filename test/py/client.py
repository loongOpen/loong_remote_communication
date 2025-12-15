import socket
import argparse
import os


def main():
    parser = argparse.ArgumentParser(description="TCP Echo Client")
    parser.add_argument(
        "--addr",
        type=str,
        default="127.0.0.1:54321",
        help="Server address (host:port or unix:///path/to/socket, default: 127.0.0.1:54321)",
    )
    args = parser.parse_args()

    if args.addr.startswith("unix://"):
        # Unix socket
        socket_path = args.addr[7:]  # Remove "unix://" prefix
        s = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        s.connect(socket_path)
        print(f"Connected to Unix socket: {socket_path}")
    else:
        # TCP socket
        parts = args.addr.rsplit(":", 1)
        if len(parts) != 2:
            print(f"Invalid address format: {args.addr}. Expected host:port or unix:///path")
            return
        host, port = parts[0], int(parts[1])
        s = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        s.connect((host, port))
        print(f"Connected to {host}:{port}")

    try:
        while True:
            msg = input("Enter message (q to quit): ")
            if msg.lower() == "q":
                break

            s.sendall(msg.encode())
            data = s.recv(1024)
            print(f"Received echo: {data.decode()}")
    finally:
        s.close()


if __name__ == "__main__":
    main()
