package go_tests

import (
	"io/ioutil"
	"net/http"
	"os"
	"strings"
	"testing"
	"time"
)

func TestComprehensiveServer(t *testing.T) {
	time.Sleep(2 * time.Second) // Wait for the server to start

	// Test case 1: Port 8080, localhost -> ./www/site
	t.Run("Port8080_Localhost", func(t *testing.T) {
		resp, err := http.Get("http://localhost:8080/")
		if err != nil {
			t.Fatalf("Failed to send request: %v", err)
		}
		defer resp.Body.Close()

		if resp.StatusCode != http.StatusOK {
			t.Errorf("Expected status code %d, got %d", http.StatusOK, resp.StatusCode)
		}

		body, err := ioutil.ReadAll(resp.Body)
		if err != nil {
			t.Fatalf("Failed to read response body: %v", err)
		}

		if !strings.Contains(string(body), "<h1>Welcome to the test website!</h1>") {
			t.Errorf("Expected body to contain %q, got %q", "<h1>Welcome to the test website!</h1>", string(body))
		}
	})

	// Test case 2: Port 8081, Host: site1.com -> ./www/site1
	t.Run("Port8081_Site1", func(t *testing.T) {
		req, err := http.NewRequest("GET", "http://127.0.0.1:8081/", nil)
		if err != nil {
			t.Fatalf("Failed to create request: %v", err)
		}
		req.Host = "site1.com"

		client := &http.Client{}
		resp, err := client.Do(req)
		if err != nil {
			t.Fatalf("Failed to send request: %v", err)
		}
		defer resp.Body.Close()

		if resp.StatusCode != http.StatusOK {
			t.Errorf("Expected status code %d, got %d", http.StatusOK, resp.StatusCode)
		}

		body, err := ioutil.ReadAll(resp.Body)
		if err != nil {
			t.Fatalf("Failed to read response body: %v", err)
		}

		if !strings.Contains(string(body), "<h1>This is site 1</h1>") {
			t.Errorf("Expected body to contain %q, got %q", "<h1>This is site 1</h1>", string(body))
		}
	})

	// Test case 3: Port 8081, Host: site2.com -> ./www/site2
	t.Run("Port8081_Site2", func(t *testing.T) {
		req, err := http.NewRequest("GET", "http://127.0.0.1:8081/", nil)
		if err != nil {
			t.Fatalf("Failed to create request: %v", err)
		}
		req.Host = "site2.com"

		client := &http.Client{}
		resp, err := client.Do(req)
		if err != nil {
			t.Fatalf("Failed to send request: %v", err)
		}
		defer resp.Body.Close()

		if resp.StatusCode != http.StatusOK {
			t.Errorf("Expected status code %d, got %d", http.StatusOK, resp.StatusCode)
		}

		body, err := ioutil.ReadAll(resp.Body)
		if err != nil {
			t.Fatalf("Failed to read response body: %v", err)
		}

		if !strings.Contains(string(body), "<h1>This is site 2</h1>") {
			t.Errorf("Expected body to contain %q, got %q", "<h1>This is site 2</h1>", string(body))
		}
	})

	// Test case 4: Port 8082, site1.com -> ./www/site1
	t.Run("Port8082_Site1", func(t *testing.T) {
		resp, err := http.Get("http://127.0.0.1:8082/")
		if err != nil {
			t.Fatalf("Failed to send request: %v", err)
		}
		defer resp.Body.Close()

		if resp.StatusCode != http.StatusOK {
			t.Errorf("Expected status code %d, got %d", http.StatusOK, resp.StatusCode)
		}

		body, err := ioutil.ReadAll(resp.Body)
		if err != nil {
			t.Fatalf("Failed to read response body: %v", err)
		}

		if !strings.Contains(string(body), "<h1>This is site 1</h1>") {
			t.Errorf("Expected body to contain %q, got %q", "<h1>This is site 1</h1>", string(body))
		}
	})

	// Test case 5: 404 Not Found
	t.Run("NotFound", func(t *testing.T) {
		resp, err := http.Get("http://localhost:8080/non-existent-file")
		if err != nil {
			t.Fatalf("Failed to send request: %v", err)
		}
		defer resp.Body.Close()

		if resp.StatusCode != http.StatusNotFound {
			t.Errorf("Expected status code %d, got %d", http.StatusNotFound, resp.StatusCode)
		}
	})

	// Test case 6: ServeStaticWebsite
	t.Run("ServeStaticWebsite", func(t *testing.T) {
		// Test the style.css file
		resp, err := http.Get("http://127.0.0.1:8080/style.css")
		if err != nil {
			t.Fatalf("Failed to send request: %v", err)
		}
		defer resp.Body.Close()
		if resp.StatusCode != http.StatusOK {
			t.Errorf("Expected status code %d, got %d", http.StatusOK, resp.StatusCode)
		}
		body, err := ioutil.ReadAll(resp.Body)
		if err != nil {
			t.Fatalf("Failed to read response body: %v", err)
		}
		if !strings.Contains(string(body), "background-color: #f0f0f0;") {
			t.Errorf("Expected body to contain %q, got %q", "background-color: #f0f0f0;", string(body))
		}

		// Test the script.js file
		resp, err = http.Get("http://127.0.0.1:8080/script.js")
		if err != nil {
			t.Fatalf("Failed to send request: %v", err)
		}
		defer resp.Body.Close()
		if resp.StatusCode != http.StatusOK {
			t.Errorf("Expected status code %d, got %d", http.StatusOK, resp.StatusCode)
		}
		body, err = ioutil.ReadAll(resp.Body)
		if err != nil {
			t.Fatalf("Failed to read response body: %v", err)
		}
		if !strings.Contains(string(body), "console.log(\"Hello from script.js!\");") {
			t.Errorf("Expected body to contain %q, got %q", "console.log(\"Hello from script.js!\");", string(body))
		}

		// Test the image.png file
		resp, err = http.Get("http://127.0.0.1:8080/image.png")
		if err != nil {
			t.Fatalf("Failed to send request: %v", err)
		}
		defer resp.Body.Close()
		if resp.StatusCode != http.StatusOK {
			t.Errorf("Expected status code %d, got %d", http.StatusOK, resp.StatusCode)
		}
		if resp.Header.Get("Content-Type") != "image/png" {
			t.Errorf("Expected Content-Type %q, got %q", "image/png", resp.Header.Get("Content-Type"))
		}
	})

	// Test case 7: MethodNotAllowed
	t.Run("MethodNotAllowed", func(t *testing.T) {
		resp, err := http.Post("http://127.0.0.1:8080/", "text/plain", nil)
		if err != nil {
			t.Fatalf("Failed to send request: %v", err)
		}
		defer resp.Body.Close()

		if resp.StatusCode != http.StatusMethodNotAllowed {
			t.Errorf("Expected status code %d, got %d", http.StatusMethodNotAllowed, resp.StatusCode)
		}
	})

	// Test case 8: CustomErrorPage
	t.Run("CustomErrorPage", func(t *testing.T) {
		resp, err := http.Get("http://localhost:8080/non-existent-file")
		if err != nil {
			t.Fatalf("Failed to send request: %v", err)
		}
		defer resp.Body.Close()

		if resp.StatusCode != http.StatusNotFound {
			t.Errorf("Expected status code %d, got %d", http.StatusNotFound, resp.StatusCode)
		}

		body, err := ioutil.ReadAll(resp.Body)
		if err != nil {
			t.Fatalf("Failed to read response body: %v", err)
		}

		if !strings.Contains(string(body), "<p>The page you requested could not be found.</p>") {
			t.Errorf("Expected body to contain %q, got %q", "<p>The page you requested could not be found.</p>", string(body))
		}
	})

	// Test case 9: Client body size limit
	t.Run("ClientBodySizeLimit", func(t *testing.T) {
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
	})

	// Test case 10: File upload and download
	t.Run("FileUploadAndDownload", func(t *testing.T) {
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

		resp, err = http.Get("http://localhost:8080" + fileURL)
		if err != nil {
			t.Fatalf("Failed to send GET request: %v", err)
		}
		defer resp.Body.Close()

		if resp.StatusCode != http.StatusOK {
			t.Fatalf("Expected status code %d, got %d", http.StatusOK, resp.StatusCode)
		}

		body, err = ioutil.ReadAll(resp.Body)
		if err != nil {
			t.Fatalf("Failed to read response body: %v", err)
		}

		if string(body) != fileContent {
			t.Errorf("Expected file content %q, got %q", fileContent, string(body))
		}
	})

	// Test case 11: DELETE request
	t.Run("DeleteRequest", func(t *testing.T) {
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
	})

	// Test case 12: Directory listing
	t.Run("DirectoryListing", func(t *testing.T) {
		resp, err := http.Get("http://localhost:8080/empty_dir/")
		if err != nil {
			t.Fatalf("Failed to send request: %v", err)
		}
		defer resp.Body.Close()

		if resp.StatusCode != http.StatusForbidden {
			t.Errorf("Expected status code %d, got %d", http.StatusForbidden, resp.StatusCode)
		}
	})
}