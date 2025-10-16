package go_tests

import (
	"io/ioutil"
	"net/http"
	"strings"
	"testing"
	"time"
)

func TestGetIndex(t *testing.T) {
	time.Sleep(2 * time.Second) // Wait for the server to start

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

	expectedBody := "<html>\n\n<body>\n    <h1>Hello from index.html!</h1>\n</body>\n\n</html>"
	if string(body) != expectedBody {
		t.Errorf("Expected body %q, got %q", expectedBody, string(body))
	}
}

func Test404NotFound(t *testing.T) {
	resp, err := http.Get("http://127.0.0.1:8082/non-existent-file")
	if err != nil {
		t.Fatalf("Failed to send request: %v", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusNotFound {
		t.Errorf("Expected status code %d, got %d", http.StatusNotFound, resp.StatusCode)
	}
}

func TestServeStaticWebsite(t *testing.T) {
	resp, err := http.Get("http://127.0.0.1:8082/site/")
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

	// Test the style.css file
	resp, err = http.Get("http://127.0.0.1:8082/site/style.css")
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
	if !strings.Contains(string(body), "background-color: #f0f0f0;") {
		t.Errorf("Expected body to contain %q, got %q", "background-color: #f0f0f0;", string(body))
	}

	// Test the script.js file
	resp, err = http.Get("http://127.0.0.1:8082/site/script.js")
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
	resp, err = http.Get("http://127.0.0.1:8082/site/image.png")
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
}

func TestRouting(t *testing.T) {
	// Test the /site/ route
	resp, err := http.Get("http://127.0.0.1:8082/site/")
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

	// Test the /cgi-bin/ route
	resp, err = http.Get("http://127.0.0.1:8082/cgi-bin/test.py")
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
	if !strings.Contains(string(body), "Hello from Python CGI!") {
		t.Errorf("Expected body to contain %q, got %q", "Hello from Python CGI!", string(body))
	}
}

func TestMethodNotAllowed(t *testing.T) {
	resp, err := http.Post("http://127.0.0.1:8082/site/", "text/plain", nil)
	if err != nil {
		t.Fatalf("Failed to send request: %v", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusMethodNotAllowed {
		t.Errorf("Expected status code %d, got %d", http.StatusMethodNotAllowed, resp.StatusCode)
	}
}

func TestCustomErrorPage(t *testing.T) {
	resp, err := http.Get("http://127.0.0.1:8082/non-existent-file")
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
}

func TestMultiPort(t *testing.T) {
	// Make a request to the server on the first port
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

	expectedBody := "<html>\n\n<body>\n    <h1>Hello from index.html!</h1>\n</body>\n\n</html>"
	if string(body) != expectedBody {
		t.Errorf("Expected body %q, got %q", expectedBody, string(body))
	}

	// Make a request to the server on the second port
	resp, err = http.Get("http://127.0.0.1:8081/")
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

	if string(body) != expectedBody {
		t.Errorf("Expected body %q, got %q", expectedBody, string(body))
	}
}
