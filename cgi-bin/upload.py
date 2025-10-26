#!/usr/bin/env python3

import os
import sys
import io
from email.parser import BytesParser
import datetime

UPLOAD_DIR = './www/uploads'

def log(message):
    with open('./logs/upload.log', 'a') as f:
        f.write(f"{datetime.datetime.now()}: {message}\n")

def main():
    log("--- upload.py called ---")
    try:
        content_type = os.environ['CONTENT_TYPE']
        content_length = os.environ.get('CONTENT_LENGTH', 0)
        log(f"CONTENT_TYPE: {content_type}")
        log(f"CONTENT_LENGTH: {content_length}")
        
        headers = f"Content-Type: {content_type}\n\n"
        body = sys.stdin.buffer.read()
        
        msg = BytesParser().parsebytes(headers.encode() + body)

        for part in msg.walk():
            if part.get_content_maintype() == 'multipart':
                continue
            if part.get('Content-Disposition') is None:
                continue
                
            filename = part.get_filename()
            if filename:
                log(f"Found file to upload: {filename}")
                if not os.path.exists(UPLOAD_DIR):
                    os.makedirs(UPLOAD_DIR)
                
                filepath = os.path.join(UPLOAD_DIR, os.path.basename(filename))
                with open(filepath, 'wb') as f:
                    f.write(part.get_payload(decode=True))
                log(f"Successfully uploaded {filepath}")
                break
    except (KeyError, IndexError, ValueError) as e:
        log(f"Error during upload: {e}")
        pass

    # Redirect back to the file list

    print("Status: 303 See Other")
    print("Location: /cgi-bin/list_files.py")
    print()

if __name__ == "__main__":
    main()
