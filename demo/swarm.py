import sys
import socket
import threading

PORT = 3030

def listen(port, conns):
    listener = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    listener.bind(("127.0.0.1", port))
    listener.listen(10)

    while True:
        conns.append(listener.accept()[0])

def server(i):
    conns = []
    listen_thread = threading.Thread(target=listen, args=(PORT, conns))
    listen_thread.start()
    while True:
        cmd = input()
        

def client(i):
    pass


if __name__ == "__main__":
    i = int(sys.argv[1] if len(sys.argv) > 1 else 0)
    if i == 0:
        server(i)
    else:
        client(i)