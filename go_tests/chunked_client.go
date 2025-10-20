package main

import (
	"fmt"
	"io"
	"net/http"
	"time"
)

func main() {
	// Create a pipe. The writer will be used to generate the body.
	pr, pw := io.Pipe()

	// Start a goroutine to write the data to the pipe writer.
	go func() {
		defer pw.Close()

		chunks := []string{"Hello, ", "world!\n", "This is ", "a chunked ", "request."}

		for _, chunk := range chunks {
			_, err := pw.Write([]byte(chunk))
			if err != nil {
				fmt.Println("Error writing chunk data:", err)
				return
			}
			time.Sleep(200 * time.Millisecond) // Sleep to simulate network delay
		}
	}()

	// Create a new POST request with the pipe reader as the body.
	// The Go http client will automatically use chunked transfer encoding
	// because the body is a reader of unknown length.
	req, err := http.NewRequest("POST", "http://127.0.0.1:8081/cgi-bin/echo.py", pr)
	if err != nil {
		fmt.Println("Error creating request:", err)
		return
	}

	// Create a client and send the request.
	client := &http.Client{}
	resp, err := client.Do(req)
	if err != nil {
		fmt.Println("Error sending request:", err)
		return
	}
	defer resp.Body.Close()

	// Read and print the response.
	fmt.Println("Response Status:", resp.Status)
	body, err := io.ReadAll(resp.Body)
	if err != nil {
		fmt.Println("Error reading response body:", err)
		return
	}
	fmt.Println("Response Body:")
	fmt.Println(string(body))
}