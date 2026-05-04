package server

import (
	"bytes"
	"encoding/json"
	"io"
	"log/slog"
	"net/http"
	"net/http/httptest"
	"strings"
	"testing"

	"github.com/dSofikitis/zerotrust-gatekeeper/auth-issuer/internal/token"
	"github.com/golang-jwt/jwt/v5"
)

func newTestHandler(t *testing.T) (http.Handler, *token.Signer) {
	t.Helper()
	s, err := token.NewSigner()
	if err != nil {
		t.Fatalf("NewSigner: %v", err)
	}
	logger := slog.New(slog.NewTextHandler(io.Discard, nil))
	return New(Config{}, s, logger), s
}

func TestHealth(t *testing.T) {
	h, _ := newTestHandler(t)
	req := httptest.NewRequest(http.MethodGet, "/healthz", nil)
	rec := httptest.NewRecorder()
	h.ServeHTTP(rec, req)
	if rec.Code != http.StatusOK {
		t.Fatalf("status = %d", rec.Code)
	}
}

func TestJWKS(t *testing.T) {
	h, signer := newTestHandler(t)
	req := httptest.NewRequest(http.MethodGet, "/.well-known/jwks.json", nil)
	rec := httptest.NewRecorder()
	h.ServeHTTP(rec, req)
	if rec.Code != http.StatusOK {
		t.Fatalf("status = %d body = %s", rec.Code, rec.Body.String())
	}
	var doc struct {
		Keys []map[string]string `json:"keys"`
	}
	if err := json.Unmarshal(rec.Body.Bytes(), &doc); err != nil {
		t.Fatalf("unmarshal: %v", err)
	}
	if len(doc.Keys) != 1 {
		t.Fatalf("len(keys) = %d", len(doc.Keys))
	}
	if doc.Keys[0]["kid"] != signer.Kid() {
		t.Errorf("kid mismatch: %s vs %s", doc.Keys[0]["kid"], signer.Kid())
	}
}

func TestTokenIssued(t *testing.T) {
	h, signer := newTestHandler(t)
	body := bytes.NewBufferString(`{"username":"alice"}`)
	req := httptest.NewRequest(http.MethodPost, "/auth/token", body)
	rec := httptest.NewRecorder()
	h.ServeHTTP(rec, req)
	if rec.Code != http.StatusOK {
		t.Fatalf("status = %d body = %s", rec.Code, rec.Body.String())
	}
	var resp TokenResponse
	if err := json.Unmarshal(rec.Body.Bytes(), &resp); err != nil {
		t.Fatalf("unmarshal: %v", err)
	}
	if resp.TokenType != "Bearer" {
		t.Errorf("token_type = %q", resp.TokenType)
	}
	if resp.AccessToken == "" {
		t.Fatal("access_token empty")
	}
	if resp.Kid != signer.Kid() {
		t.Errorf("kid mismatch")
	}

	// Verify the issued token round-trips against the signer's public key.
	parsed, err := jwt.ParseWithClaims(resp.AccessToken, &token.Claims{}, func(*jwt.Token) (any, error) {
		return signer.PublicKey(), nil
	}, jwt.WithAudience(token.Audience), jwt.WithIssuer(token.Issuer))
	if err != nil {
		t.Fatalf("Parse: %v", err)
	}
	c := parsed.Claims.(*token.Claims)
	if c.Subject != "alice" {
		t.Errorf("Subject = %q", c.Subject)
	}
	if c.Tenant != "acme" {
		t.Errorf("Tenant = %q, want acme (dev derivation for 'alice')", c.Tenant)
	}
	if len(c.Roles) != 1 || c.Roles[0] != "user" {
		t.Errorf("Roles = %v, want [user]", c.Roles)
	}
}

func TestTokenAdminRole(t *testing.T) {
	h, signer := newTestHandler(t)
	req := httptest.NewRequest(http.MethodPost, "/auth/token", strings.NewReader(`{"username":"admin"}`))
	rec := httptest.NewRecorder()
	h.ServeHTTP(rec, req)
	if rec.Code != http.StatusOK {
		t.Fatalf("status = %d", rec.Code)
	}
	var resp TokenResponse
	_ = json.Unmarshal(rec.Body.Bytes(), &resp)
	parsed, err := jwt.ParseWithClaims(resp.AccessToken, &token.Claims{}, func(*jwt.Token) (any, error) {
		return signer.PublicKey(), nil
	})
	if err != nil {
		t.Fatalf("Parse: %v", err)
	}
	c := parsed.Claims.(*token.Claims)
	if len(c.Roles) != 1 || c.Roles[0] != "admin" {
		t.Errorf("Roles = %v, want [admin]", c.Roles)
	}
}

func TestTokenExplicitOverrides(t *testing.T) {
	h, signer := newTestHandler(t)
	body := strings.NewReader(`{"username":"alice","tenant":"globex","roles":["read-only"],"country":"FR"}`)
	req := httptest.NewRequest(http.MethodPost, "/auth/token", body)
	rec := httptest.NewRecorder()
	h.ServeHTTP(rec, req)
	if rec.Code != http.StatusOK {
		t.Fatalf("status = %d", rec.Code)
	}
	var resp TokenResponse
	_ = json.Unmarshal(rec.Body.Bytes(), &resp)
	parsed, err := jwt.ParseWithClaims(resp.AccessToken, &token.Claims{}, func(*jwt.Token) (any, error) {
		return signer.PublicKey(), nil
	})
	if err != nil {
		t.Fatalf("Parse: %v", err)
	}
	c := parsed.Claims.(*token.Claims)
	if c.Tenant != "globex" {
		t.Errorf("Tenant = %q", c.Tenant)
	}
	if len(c.Roles) != 1 || c.Roles[0] != "read-only" {
		t.Errorf("Roles = %v", c.Roles)
	}
	if c.Country != "FR" {
		t.Errorf("Country = %q", c.Country)
	}
}

func TestTokenMissingUsername(t *testing.T) {
	h, _ := newTestHandler(t)
	req := httptest.NewRequest(http.MethodPost, "/auth/token", strings.NewReader(`{}`))
	rec := httptest.NewRecorder()
	h.ServeHTTP(rec, req)
	if rec.Code != http.StatusBadRequest {
		t.Fatalf("status = %d, want 400", rec.Code)
	}
}

func TestTokenBadJSON(t *testing.T) {
	h, _ := newTestHandler(t)
	req := httptest.NewRequest(http.MethodPost, "/auth/token", strings.NewReader(`{not-json`))
	rec := httptest.NewRecorder()
	h.ServeHTTP(rec, req)
	if rec.Code != http.StatusBadRequest {
		t.Fatalf("status = %d, want 400", rec.Code)
	}
}
