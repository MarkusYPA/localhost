#!/usr/bin/env python3

import os

print("Content-Type: text/plain")
print()
print("Hello from Python CGI!")
print(f"PATH_INFO: {os.environ.get('PATH_INFO')}")
