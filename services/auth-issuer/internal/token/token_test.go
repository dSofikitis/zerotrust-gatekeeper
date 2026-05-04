package token

import (
	"crypto/rsa"
	"encoding/base64"
	"encoding/json"
	"math/big"
	"testing"
	"time"

	"github.com/golang-jwt/jwt/v5"
)

var fixedNow = func() time.Time { return time.Date(2026, 5, 3, 12, 0, 0, 0, time.UTC) }

func newTestSigner(t *testing.T) *Signer {
	t.Helper()
	s, err := NewSigner()
	if err != nil {
		t.Fatalf("NewSigner: %v", err)
	}
	s.SetClock(fixedNow)
	return s
}

func TestMintRoundTrip(t *testing.T) {
	s := newTestSigner(t)
	tok, err := s.Mint("alice", Claims{Tenant: "acme", Roles: []string{"user"}, Country: "GR"}, 0)
	if err != nil {
		t.Fatalf("Mint: %v", err)
	}

	parsed, err := jwt.ParseWithClaims(tok, &Claims{}, func(t *jwt.Token) (any, error) {
		return s.PublicKey(), nil
	}, jwt.WithIssuer(Issuer), jwt.WithAudience(Audience), jwt.WithLeeway(time.Second), jwt.WithTimeFunc(fixedNow))
	if err != nil {
		t.Fatalf("Parse: %v", err)
	}
	if !parsed.Valid {
		t.Fatal("token was not valid")
	}
	c, ok := parsed.Claims.(*Claims)
	if !ok {
		t.Fatalf("unexpected claims type: %T", parsed.Claims)
	}
	if c.Subject != "alice" {
		t.Errorf("Subject = %q, want alice", c.Subject)
	}
	if c.Tenant != "acme" {
		t.Errorf("Tenant = %q, want acme", c.Tenant)
	}
	if got, want := c.Roles, []string{"user"}; !equalStrings(got, want) {
		t.Errorf("Roles = %v, want %v", got, want)
	}
	if c.Country != "GR" {
		t.Errorf("Country = %q, want GR", c.Country)
	}
	if parsed.Header["kid"] != s.Kid() {
		t.Errorf("kid header = %v, want %s", parsed.Header["kid"], s.Kid())
	}
}

func TestJWKSExposesPublicKey(t *testing.T) {
	s := newTestSigner(t)
	bs, err := s.JWKS()
	if err != nil {
		t.Fatalf("JWKS: %v", err)
	}
	var doc struct {
		Keys []map[string]string `json:"keys"`
	}
	if err := json.Unmarshal(bs, &doc); err != nil {
		t.Fatalf("unmarshal: %v", err)
	}
	if len(doc.Keys) != 1 {
		t.Fatalf("len(keys) = %d, want 1", len(doc.Keys))
	}
	jwk := doc.Keys[0]
	for _, want := range []string{"kty", "alg", "use", "kid", "n", "e"} {
		if jwk[want] == "" {
			t.Errorf("jwk missing %q", want)
		}
	}
	if jwk["kty"] != "RSA" || jwk["alg"] != "RS256" || jwk["use"] != "sig" {
		t.Errorf("unexpected fixed fields: kty=%s alg=%s use=%s", jwk["kty"], jwk["alg"], jwk["use"])
	}
	if jwk["kid"] != s.Kid() {
		t.Errorf("jwk kid = %s, want %s", jwk["kid"], s.Kid())
	}

	// The (n, e) pair should reconstruct the actual public key.
	rebuilt := &rsa.PublicKey{}
	nBytes, err := base64.RawURLEncoding.DecodeString(jwk["n"])
	if err != nil {
		t.Fatalf("decode n: %v", err)
	}
	rebuilt.N = new(big.Int).SetBytes(nBytes)
	eBytes, err := base64.RawURLEncoding.DecodeString(jwk["e"])
	if err != nil {
		t.Fatalf("decode e: %v", err)
	}
	rebuilt.E = int(new(big.Int).SetBytes(eBytes).Int64())
	pub := s.PublicKey()
	if rebuilt.N.Cmp(pub.N) != 0 || rebuilt.E != pub.E {
		t.Fatal("rebuilt key does not match signer's public key")
	}
}

func TestExpiryEnforced(t *testing.T) {
	s := newTestSigner(t)
	// Mint with a tiny TTL pinned to the signer's frozen clock.
	tok, err := s.Mint("alice", Claims{Tenant: "acme"}, time.Millisecond)
	if err != nil {
		t.Fatalf("Mint: %v", err)
	}
	// Parse from a wall clock one hour after the signer's clock —
	// the token must be rejected as expired.
	future := func() time.Time { return time.Date(2026, 5, 3, 13, 0, 0, 0, time.UTC) }
	parser := jwt.NewParser(
		jwt.WithIssuer(Issuer),
		jwt.WithAudience(Audience),
		jwt.WithLeeway(0),
		jwt.WithTimeFunc(future),
	)
	parsed, err := parser.ParseWithClaims(tok, &Claims{}, func(*jwt.Token) (any, error) {
		return s.PublicKey(), nil
	})
	if err == nil && parsed.Valid {
		t.Fatal("expected expiry rejection, got valid token")
	}
}

func equalStrings(a, b []string) bool {
	if len(a) != len(b) {
		return false
	}
	for i := range a {
		if a[i] != b[i] {
			return false
		}
	}
	return true
}
