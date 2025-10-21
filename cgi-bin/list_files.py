#!/usr/bin/env python3

import os

UPLOAD_DIR = './www/uploads'

def list_files():
    print("Content-Type: text/html")
    print()
    print("<html><head><title>File List</title></head><body>")
    print("<h1>Files in uploads directory</h1>")

    # List files
    print("<ul>")
    if not os.path.exists(UPLOAD_DIR):
        os.makedirs(UPLOAD_DIR)
    for filename in os.listdir(UPLOAD_DIR):
        if filename == ".gitkeep":
            continue
        print(f"<li>{filename} ")
        print(f"<form action='/cgi-bin/delete.py' method='post' style='display:inline;'>")
        print(f"<input type='hidden' name='filename' value='{filename}'>")
        print(f"<input type='submit' value='Delete'>")
        print("</form></li>")
    print("</ul>")

    # Upload form
    print("<h2>Upload a file</h2>")
    print("<form action='/cgi-bin/upload.py' method='post' enctype='multipart/form-data'>")
    print("<input type='file' name='file'>")
    print("<input type='submit' value='Upload'>")
    print("</form>")

    print("</body></html>")

if __name__ == "__main__":
    list_files()