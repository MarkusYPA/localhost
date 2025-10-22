package go_tests

import (
	"io"
	"io/ioutil"
	"mime/multipart"
	"net"
	"net/http"
	"net/url"
	"os"
	"strings"
	"testing"
	"time"
)

func TestComprehensiveServer(t *testing.T) {

	t.Run("Port8080_Localhost", testPort8080_Localhost)
	t.Run("Port8081_Site1", testPort8081_Site1)
	t.Run("Port8081_Site2", testPort8081_Site2)
	t.Run("Port8082_Site1", testPort8082_Site1)
	t.Run("NotFound", testNotFound)
	t.Run("ServeStaticWebsite", testServeStaticWebsite)
	t.Run("MethodNotAllowed", testMethodNotAllowed)
	t.Run("CustomErrorPage", testCustomErrorPage)
	t.Run("ClientBodySizeLimit", testClientBodySizeLimit)
	t.Run("FileUploadAndDownload", testFileUploadAndDownload)
	t.Run("DeleteRequest", testDeleteRequest)
	t.Run("DirectoryListing", testDirectoryListing)
	t.Run("ChunkedRequest", testChunkedRequest)
	t.Run("Timeout", testTimeout)
}

func testChunkedRequest(t *testing.T) {
	pr, pw := io.Pipe()

	go func() {
		defer pw.Close()
		chunks := []string{"Hello, ", "world!\n", "This is ", "a chunked ", "request."}
		for _, chunk := range chunks {
			_, err := pw.Write([]byte(chunk))
			if err != nil {
				t.Error("Error writing chunk data:", err)
				return
			}
			time.Sleep(200 * time.Millisecond)
		}
	}()

	req, err := http.NewRequest("POST", "http://127.0.0.1:8081/cgi-bin/echo.py", pr)
	if err != nil {
		t.Fatal("Error creating request:", err)
	}

	client := &http.Client{}
	resp, err := client.Do(req)
	if err != nil {
		t.Fatal("Error sending request:", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		t.Errorf("Expected status code %d, got %d", http.StatusOK, resp.StatusCode)
	}

	body := readBody(t, resp)
	expectedBody := "Hello, world!\nThis is a chunked request."
	assertBodyContains(t, body, expectedBody)
}

func testPort8080_Localhost(t *testing.T) {
	resp := testGetRequest(t, "http://localhost:8080/", http.StatusOK)
	defer resp.Body.Close()
	body := readBody(t, resp)
	assertBodyContains(t, body, "<h1>Welcome to the test website!</h1>")
}

func testPort8081_Site1(t *testing.T) {
	resp := testHostRequest(t, "http://127.0.0.1:8081/", "site1.com", http.StatusOK)
	defer resp.Body.Close()
	body := readBody(t, resp)
	assertBodyContains(t, body, "<html><body><h1>Hello from site 1!</h1></body></html>")
}

func testPort8081_Site2(t *testing.T) {
	resp := testHostRequest(t, "http://127.0.0.1:8081/", "site2.com", http.StatusOK)
	defer resp.Body.Close()
	body := readBody(t, resp)
	assertBodyContains(t, body, "<html><body><h1>Hello from site 2!</h1></body></html>")
}

func testPort8082_Site1(t *testing.T) {
	resp := testGetRequest(t, "http://127.0.0.1:8082/", http.StatusOK)
	defer resp.Body.Close()
	body := readBody(t, resp)
	assertBodyContains(t, body, "<html><body><h1>Hello from site 1!</h1></body></html>")
}

func testNotFound(t *testing.T) {
	resp := testGetRequest(t, "http://localhost:8080/non-existent-file", http.StatusNotFound)
	resp.Body.Close()
}

func testServeStaticWebsite(t *testing.T) {
	// Test the style.css file
	resp := testGetRequest(t, "http://127.0.0.1:8080/style.css", http.StatusOK)
	defer resp.Body.Close()
	body := readBody(t, resp)
	assertBodyContains(t, body, "background-color: #f0f0f0;")

	// Test the script.js file
	resp = testGetRequest(t, "http://127.0.0.1:8080/script.js", http.StatusOK)
	defer resp.Body.Close()
	body = readBody(t, resp)
	assertBodyContains(t, body, "console.log(\"Hello from script.js!\");")

	// Test the image.png file
	resp = testGetRequest(t, "http://127.0.0.1:8080/image.png", http.StatusOK)
	defer resp.Body.Close()
	if resp.Header.Get("Content-Type") != "image/png" {
		t.Errorf("Expected Content-Type %q, got %q", "image/png", resp.Header.Get("Content-Type"))
	}
}

func testMethodNotAllowed(t *testing.T) {
	resp, err := http.Post("http://127.0.0.1:8080/", "text/plain", nil)
	if err != nil {
		t.Fatalf("Failed to send request: %v", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusMethodNotAllowed {
		t.Errorf("Expected status code %d, got %d", http.StatusMethodNotAllowed, resp.StatusCode)
	}
}

func testCustomErrorPage(t *testing.T) {
	resp := testGetRequest(t, "http://localhost:8080/non-existent-file", http.StatusNotFound)
	defer resp.Body.Close()
	body := readBody(t, resp)
	assertBodyContains(t, body, "<p>The page you requested could not be found.</p>")
}

func testClientBodySizeLimit(t *testing.T) {
	// Test body larger than limit
	largeBody := strings.NewReader("12345678901")
	resp, err := http.Post("http://127.0.0.1:8083/cgi-bin/echo.py", "text/plain", largeBody)
	if err != nil {
		t.Fatalf("Failed to send request: %v", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusRequestEntityTooLarge {
		t.Errorf("Expected status code %d, got %d", http.StatusRequestEntityTooLarge, resp.StatusCode)
	}

	// Test body smaller than limit
	smallBody := strings.NewReader("12345")
	resp, err = http.Post("http://127.0.0.1:8083/cgi-bin/echo.py", "text/plain", smallBody)
	if err != nil {
		t.Fatalf("Failed to send request: %v", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		t.Errorf("Expected status code %d, got %d", http.StatusOK, resp.StatusCode)
	}
}

func testFileUploadAndDownload(t *testing.T) {
	fileContent := "This is a test file for upload and download."
	fileName := "test_upload.txt"

	// Create a pipe to write the multipart request body
	pr, pw := io.Pipe()
	writer := multipart.NewWriter(pw)

	// Create a goroutine to write the multipart request body
	go func() {
		defer pw.Close()
		defer writer.Close()

		// Add the file part
		part, err := writer.CreateFormFile("file", fileName)
		if err != nil {
			t.Errorf("Failed to create form file: %v", err)
			return
		}
		_, err = io.Copy(part, strings.NewReader(fileContent))
		if err != nil {
			t.Errorf("Failed to write to form file: %v", err)
			return
		}
	}()

	// Create the request
	req, err := http.NewRequest("POST", "http://127.0.0.1:8081/cgi-bin/upload.py", pr)
	if err != nil {
		t.Fatalf("Failed to create request: %v", err)
	}
	req.Header.Set("Content-Type", writer.FormDataContentType())

	// Send the request
	client := &http.Client{}
	resp, err := client.Do(req)
	if err != nil {
		t.Fatalf("Failed to send request: %v", err)
	}
	defer resp.Body.Close()

	// Check the status code
	if resp.StatusCode != http.StatusOK {
		t.Errorf("Expected status code %d, got %d", http.StatusOK, resp.StatusCode)
	}

	// Check if the file was uploaded
	filePath := "../www/uploads/" + fileName
	_, err = os.Stat(filePath)
	if os.IsNotExist(err) {
		t.Errorf("Expected file to be uploaded, but it does not exist.")
	}

	// Clean up the uploaded file
	err = os.Remove(filePath)
	if err != nil {
		t.Errorf("Failed to clean up uploaded file: %v", err)
	}
}

func testDeleteRequest(t *testing.T) {
	fileName := "test_file_for_delete.txt"
	filePath := "../www/uploads/" + fileName
	fileContent := "This is a test file for deletion."

	if err := os.WriteFile(filePath, []byte(fileContent), 0644); err != nil {
		t.Fatalf("Failed to create test file: %v", err)
	}

	// Create the request body
	form := url.Values{}
	form.Add("filename", fileName)
	body := strings.NewReader(form.Encode())

	// Create the request
	req, err := http.NewRequest("POST", "http://127.0.0.1:8081/cgi-bin/delete.py", body)
	if err != nil {
		t.Fatalf("Failed to create request: %v", err)
	}
	req.Header.Set("Content-Type", "application/x-www-form-urlencoded")

	// Send the request
	client := &http.Client{}
	resp, err := client.Do(req)
	if err != nil {
		t.Fatalf("Failed to send request: %v", err)
	}
	defer resp.Body.Close()

	// Check the status code
	if resp.StatusCode != http.StatusOK {
		t.Errorf("Expected status code %d, got %d", http.StatusOK, resp.StatusCode)
	}

	// Check if the file has been deleted
	if _, err := os.Stat(filePath); !os.IsNotExist(err) {
		t.Errorf("Expected file to be deleted, but it still exists.")
	}
}

func testDirectoryListing(t *testing.T) {
	resp := testGetRequest(t, "http://localhost:8080/empty_dir/", http.StatusForbidden)
	resp.Body.Close()
}

func testTimeout(t *testing.T) {
	addr := "127.0.0.1:8080" // use 127.0.0.1 to avoid DNS
	// Open raw TCP connection (no http.Client timeouts)
	conn, err := net.Dial("tcp", addr)
	if err != nil {
		t.Fatalf("failed to connect to %s: %v", addr, err)
	}
	defer conn.Close()

	// Wait longer than the server idle timeout (server is expected to close at ~2s)
	wait := 4 * time.Second
	t.Logf("connected -> sleeping %v (expect server to close)...", wait)
	time.Sleep(wait)

	// Now try a non-blocking read to see if server closed the connection.
	// Set a short read deadline only for this read so we don't block forever.
	if err := conn.SetReadDeadline(time.Now().Add(1 * time.Second)); err != nil {
		t.Fatalf("SetReadDeadline failed: %v", err)
	}

	buf := make([]byte, 1)
	n, err := conn.Read(buf)
	if err == nil {
		// If we read bytes with no error, connection is still alive.
		t.Fatalf("expected server to have closed the connection, but read %d bytes and no error", n)
	}

	// If server closed, Read commonly returns io.EOF (or underlying error).
	if err == io.EOF {
		t.Logf("server closed the connection (EOF) as expected")
		return
	}

	// If we get a timeout error on Read, it means server didn't send FIN; connection likely still alive.
	// Check for timeout (net.Error Timeout())
	if ne, ok := err.(net.Error); ok && ne.Timeout() {
		t.Fatalf("read timed out: server did NOT close connection within expected time; err=%v", err)
	}

	// Any other error is treated as server closed (or something else happened).
	t.Fatalf("unexpected read error: %v", err)
}

// Helper functions

func testGetRequest(t *testing.T, url string, expectedStatusCode int) *http.Response {
	t.Helper()
	resp, err := http.Get(url)
	if err != nil {
		t.Fatalf("Failed to send request to %s: %v", url, err)
	}
	if resp.StatusCode != expectedStatusCode {
		t.Errorf("Expected status code %d for %s, got %d", expectedStatusCode, url, resp.StatusCode)
	}
	return resp
}

func testHostRequest(t *testing.T, url string, host string, expectedStatusCode int) *http.Response {
	t.Helper()
	req, err := http.NewRequest("GET", url, nil)
	if err != nil {
		t.Fatalf("Failed to create request for %s: %v", url, err)
	}
	req.Host = host

	client := &http.Client{}
	resp, err := client.Do(req)
	if err != nil {
		t.Fatalf("Failed to send request to %s with host %s: %v", url, host, err)
	}
	if resp.StatusCode != expectedStatusCode {
		t.Errorf("Expected status code %d for %s with host %s, got %d", expectedStatusCode, url, host, resp.StatusCode)
	}
	return resp
}

func readBody(t *testing.T, resp *http.Response) []byte {
	t.Helper()
	body, err := ioutil.ReadAll(resp.Body)
	if err != nil {
		t.Fatalf("Failed to read response body: %v", err)
	}
	return body
}

func assertBodyContains(t *testing.T, body []byte, expected string) {
	t.Helper()
	if !strings.Contains(string(body), expected) {
		t.Errorf("Expected body to contain %q, got %q", expected, string(body))
	}
}
