#!/usr/bin/env python3

import os

UPLOAD_DIR = './www/uploads'

def list_files():
    print("Content-Type: text/html")
    print()
    print("<html><head><title>File List</title>")
    print("<script>")
    print("function deleteFile(filename) {")
    print("    if (confirm('Are you sure you want to delete ' + filename + '?')) {")
    print("        fetch('/cgi-bin/delete.py', {")
    print("            method: 'DELETE',")
    print("            headers: {")
    print("                'Content-Type': 'application/x-www-form-urlencoded',")
    print("            },")
    print("            body: 'filename=' + encodeURIComponent(filename)")
    print("        })")
    print("        .then(response => {")
    print("            if (response.ok) {")
    print("                window.location.reload();")
    print("            } else {")
    print("                alert('Error deleting file: ' + response.statusText);")
    print("            }")
    print("        })")
    print("        .catch(error => {")
    print("            console.error('Error:', error);")
    print("            alert('Network error or server issue.');")
    print("        });")
    print("    }")
    print("    return false;")
    print("}")
    print("</script>")
    print("</head><body>")
    print("<h1>Files in uploads directory</h1>")

    # List files
    print("<ul>")
    if not os.path.exists(UPLOAD_DIR):
        os.makedirs(UPLOAD_DIR)
    for filename in os.listdir(UPLOAD_DIR):
        if filename == ".gitkeep":
            continue
        print(f"<li>{filename} ")
        print(f"<form onsubmit='return deleteFile(\"{filename}\");' style='display:inline;'>")
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