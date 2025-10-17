import socket
import time

HOST = '127.0.0.1'
PORT = 8081

with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as s:
    s.connect((HOST, PORT))
    print(f"Connected to {HOST}:{PORT}")
    time.sleep(15)
    print("Finished sleeping")
