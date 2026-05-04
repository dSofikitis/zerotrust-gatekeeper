package main

import "fmt"

// Version is bumped per release. The protected upstream lands in
// phase 8.
const Version = "0.1.0"

func main() {
	fmt.Printf("backend-echo %s: scaffolded; implementation lands in phase 8.\n", Version)
}
