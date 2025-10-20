#!/usr/bin/env python3

import os
import sys
import urllib.parse
import datetime

UPLOAD_DIR = './www/uploads'

def log(message):
    with open('/Users/oleg.balandin/Desktop/work/localhost/python.log', 'a') as f:
        f.write(f"{datetime.datetime.now()}: {message}\n")

def main():
    log("---delete.py called ---")
    try:
        content_length = int(os.environ.get('CONTENT_LENGTH', 0))
        body = sys.stdin.read(content_length)
        form_data = urllib.parse.parse_qs(body)
        filename = form_data.get('filename', [None])[0]
        log(f"CONTENT_LENGTH: {content_length}")
        log(f"Body: {body}")
        log(f"Parsed form data: {form_data}")
        log(f"Filename to delete: {filename}")
    except (KeyError, IndexError, ValueError) as e:
        log(f"Error parsing form data: {e}")
        filename = None

    if filename:
        filepath = os.path.join(UPLOAD_DIR, os.path.basename(filename))
        if os.path.exists(filepath):
            try:
                os.remove(filepath)
                log(f"Successfully deleted {filepath}")
            except OSError as e:
                log(f"Error deleting {filepath}: {e}")
        else:
            log(f"File not found for deletion: {filepath}")

    # Redirect back to the file list
    print("Status: 303 See Other")
    print("Location: /cgi-bin/list_files.py")
    print()

if __name__ == "__main__":
    main()
