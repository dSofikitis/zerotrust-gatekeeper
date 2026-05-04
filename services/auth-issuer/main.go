package main

import "fmt"

// Version is bumped per release. The real auth-issuer (token mint
// + JWKS endpoint) lands in phase 3.
const Version = "0.1.0"

func main() {
	fmt.Printf("auth-issuer %s: scaffolded; implementation lands in phase 3.\n", Version)
}
