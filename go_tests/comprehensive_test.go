package go_tests

import (
	"io"
	"io/ioutil"
	"net"
	"net/http"
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
	t.Run("Timeout", testTimeout)
	t.Run("ChunkedRequest", testChunkedRequest)
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
	resp, err := http.Post("http://localhost:8083/", "text/plain", largeBody)
	if err != nil {
		t.Fatalf("Failed to send request: %v", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusRequestEntityTooLarge {
		t.Errorf("Expected status code %d, got %d", http.StatusRequestEntityTooLarge, resp.StatusCode)
	}

	// Test body smaller than limit
	smallBody := strings.NewReader("12345")
	resp, err = http.Post("http://localhost:8083/", "text/plain", smallBody)
	if err != nil {
		t.Fatalf("Failed to send request: %v", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusCreated {
		t.Errorf("Expected status code %d, got %d", http.StatusCreated, resp.StatusCode)
	}
}

func testFileUploadAndDownload(t *testing.T) {
	fileContent := "This is a test file for upload and download."
	resp, err := http.Post("http://localhost:8080/post_test", "text/plain", strings.NewReader(fileContent))
	if err != nil {
		t.Fatalf("Failed to send POST request: %v", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusCreated {
		t.Fatalf("Expected status code %d, got %d", http.StatusCreated, resp.StatusCode)
	}

	body, err := ioutil.ReadAll(resp.Body)
	if err != nil {
		t.Fatalf("Failed to read response body: %v", err)
	}

	fileURL := string(body)

	resp = testGetRequest(t, "http://localhost:8080"+fileURL, http.StatusOK)
	defer resp.Body.Close()

	body = readBody(t, resp)

	if string(body) != fileContent {
		t.Errorf("Expected file content %q, got %q", fileContent, string(body))
	}
}

func testDeleteRequest(t *testing.T) {
	fileName := "test_file_for_delete.txt"
	filePath := "../www/uploads/" + fileName
	fileContent := "This is a test file for deletion."

	if err := ioutil.WriteFile(filePath, []byte(fileContent), 0644); err != nil {
		t.Fatalf("Failed to create test file: %v", err)
	}

	req, err := http.NewRequest("DELETE", "http://localhost:8080/delete_test/"+fileName, nil)
	if err != nil {
		t.Fatalf("Failed to create request: %v", err)
	}

	client := &http.Client{}
	resp, err := client.Do(req)
	if err != nil {
		t.Fatalf("Failed to send request: %v", err)
	}
	defer resp.Body.Close()

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
	conn, err := net.Dial("tcp", "localhost:8080")
	if err != nil {
		t.Fatalf("Failed to connect to server: %v", err)
	}
	defer conn.Close()

	time.Sleep(3 * time.Second)

	_, err = conn.Write([]byte("GET / HTTP/1.1\r\nHost: localhost\r\n\r\n"))
	if err == nil {
		t.Errorf("Expected a write error after timeout, but got none")
	}
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
