#!/usr/bin/env python3
import sys
import datetime

def log(message):
    with open('python.log', 'a') as f:
        f.write(f"{datetime.datetime.now()}: {message}\n")

def main():
    log("--- echo.py called ---")
    body = sys.stdin.read()
    log(f"Received body in echo.py: {body}")
    log(f"Body length: {len(body)}")
    
    print("Content-Type: text/plain")
    print(f"Content-Length: {len(body)}")
    print()
    print(body)

if __name__ == "__main__":
    main()