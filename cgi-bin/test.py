#!/usr/bin/env python3

import os
from time import sleep

print("Content-Type: text/plain")
print()
print("Hello from Python CGI!")
#sleep(10)
print(f"PATH_INFO: {os.environ.get('PATH_INFO')}")
