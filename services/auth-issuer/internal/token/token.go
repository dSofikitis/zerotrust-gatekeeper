// Package token mints + signs JWTs for the dev IdP and exports the
// public verification key as a JWKS document. RSA-SHA256 (RS256) is
// chosen because it's what every OIDC client expects and lets the
// gateway verify against the public key alone.
package token

import (
	"crypto/rand"
	"crypto/rsa"
	"crypto/sha256"
	"encoding/base64"
	"encoding/json"
	"fmt"
	"math/big"
	"time"

	"github.com/golang-jwt/jwt/v5"
)

const (
	// DefaultTTL is how long a freshly minted token is valid for.
	DefaultTTL = 15 * time.Minute
	// Issuer is stamped into the iss claim and JWKS issuer field.
	Issuer = "zt-auth-issuer"
	// Audience is what the gateway will check on incoming tokens.
	Audience = "zt-gateway"
)

// Claims carries the dev-IdP's JWT payload. Anything new the
// gateway / OPA needs goes through here so the wire format and the
// Rego input contract stay in lock-step.
type Claims struct {
	Tenant  string   `json:"tenant"`
	Roles   []string `json:"roles"`
	Country string   `json:"country,omitempty"`
	jwt.RegisteredClaims
}

// Signer holds the RSA keypair + key id used to sign tokens. The
// public side is what JWKS exposes.
type Signer struct {
	key *rsa.PrivateKey
	kid string
	now func() time.Time
}

// NewSigner generates a fresh 2048-bit RSA key on startup. Real
// deploys would either load from a managed key store or rotate via
// scheduled re-generation; the dev IdP keeps it simple.
func NewSigner() (*Signer, error) {
	key, err := rsa.GenerateKey(rand.Reader, 2048)
	if err != nil {
		return nil, fmt.Errorf("generate key: %w", err)
	}
	kid, err := computeKid(&key.PublicKey)
	if err != nil {
		return nil, err
	}
	return &Signer{key: key, kid: kid, now: time.Now}, nil
}

// SetClock swaps the time source for deterministic tests.
func (s *Signer) SetClock(now func() time.Time) { s.now = now }

// Kid returns the key id stamped into both the JWS header and the
// matching JWK on the JWKS endpoint.
func (s *Signer) Kid() string { return s.kid }

// Mint produces a signed compact JWS for the given subject + claims.
// ttl=0 falls back to DefaultTTL.
func (s *Signer) Mint(subject string, c Claims, ttl time.Duration) (string, error) {
	if ttl <= 0 {
		ttl = DefaultTTL
	}
	now := s.now()
	c.RegisteredClaims = jwt.RegisteredClaims{
		Issuer:    Issuer,
		Subject:   subject,
		Audience:  jwt.ClaimStrings{Audience},
		IssuedAt:  jwt.NewNumericDate(now),
		NotBefore: jwt.NewNumericDate(now),
		ExpiresAt: jwt.NewNumericDate(now.Add(ttl)),
	}
	tok := jwt.NewWithClaims(jwt.SigningMethodRS256, c)
	tok.Header["kid"] = s.kid
	return tok.SignedString(s.key)
}

// JWKS returns the bytes of a JWKS document with the single signing
// key. Cache-Control headers belong on the HTTP layer, not here.
func (s *Signer) JWKS() ([]byte, error) {
	pub := s.key.PublicKey
	jwk := map[string]string{
		"kty": "RSA",
		"alg": "RS256",
		"use": "sig",
		"kid": s.kid,
		"n":   base64.RawURLEncoding.EncodeToString(pub.N.Bytes()),
		"e":   base64.RawURLEncoding.EncodeToString(big.NewInt(int64(pub.E)).Bytes()),
	}
	return json.Marshal(map[string]any{"keys": []any{jwk}})
}

// PublicKey is exposed so tests (and a future in-process gateway
// integration) can verify without round-tripping through JWKS.
func (s *Signer) PublicKey() *rsa.PublicKey { return &s.key.PublicKey }

// computeKid derives a stable RFC 7638 thumbprint over the public
// key. Stable across restarts of the same key, fresh every restart
// of NewSigner because the key itself is fresh.
func computeKid(pub *rsa.PublicKey) (string, error) {
	canonical := map[string]string{
		"e":   base64.RawURLEncoding.EncodeToString(big.NewInt(int64(pub.E)).Bytes()),
		"kty": "RSA",
		"n":   base64.RawURLEncoding.EncodeToString(pub.N.Bytes()),
	}
	bs, err := json.Marshal(canonical)
	if err != nil {
		return "", err
	}
	sum := sha256.Sum256(bs)
	return base64.RawURLEncoding.EncodeToString(sum[:]), nil
}
