package server

import (
	"encoding/json"
	"io"
	"log/slog"
	"net/http"
	"net/http/httptest"
	"strings"
	"testing"
)

func newTestHandler() http.Handler {
	logger := slog.New(slog.NewTextHandler(io.Discard, nil))
	return New(logger)
}

func TestHealth(t *testing.T) {
	req := httptest.NewRequest(http.MethodGet, "/healthz", nil)
	rec := httptest.NewRecorder()
	newTestHandler().ServeHTTP(rec, req)
	if rec.Code != http.StatusOK {
		t.Fatalf("status = %d", rec.Code)
	}
	if !strings.Contains(rec.Body.String(), `"status":"ok"`) {
		t.Fatalf("body = %q", rec.Body.String())
	}
}

func TestEchoStampsIdentityFromHeaders(t *testing.T) {
	req := httptest.NewRequest(http.MethodPost, "/api/echo?q=1", strings.NewReader(`{"hello":"world"}`))
	req.Header.Set("Content-Type", "application/json")
	req.Header.Set("X-Request-Id", "req-42")
	req.Header.Set("X-Auth-Subject", "alice")
	req.Header.Set("X-Auth-Tenant", "acme")
	req.Header.Set("X-Auth-Roles", "user,billing")
	req.Header.Set("X-Auth-Country", "GR")
	req.Header.Set("Authorization", "Bearer must-not-be-echoed")

	rec := httptest.NewRecorder()
	newTestHandler().ServeHTTP(rec, req)
	if rec.Code != http.StatusOK {
		t.Fatalf("status = %d body = %s", rec.Code, rec.Body.String())
	}

	var resp EchoResponse
	if err := json.NewDecoder(rec.Body).Decode(&resp); err != nil {
		t.Fatalf("decode: %v", err)
	}
	if resp.Method != "POST" || resp.Path != "/api/echo" || resp.Query != "q=1" {
		t.Errorf("request line wrong: %+v", resp)
	}
	if resp.RequestID != "req-42" {
		t.Errorf("RequestID = %q", resp.RequestID)
	}
	if resp.Identity.Subject != "alice" || resp.Identity.Tenant != "acme" || resp.Identity.Country != "GR" {
		t.Errorf("identity wrong: %+v", resp.Identity)
	}
	if got := resp.Identity.Roles; len(got) != 2 || got[0] != "user" || got[1] != "billing" {
		t.Errorf("roles = %v", got)
	}
	if resp.Body != `{"hello":"world"}` {
		t.Errorf("body = %q", resp.Body)
	}
	// Sensitive headers must not be echoed.
	for _, sensitive := range []string{"Authorization", "Cookie", "Proxy-Authorization"} {
		if _, present := resp.Headers[sensitive]; present {
			t.Errorf("%s header was echoed but should be dropped", sensitive)
		}
	}
	if rec.Header().Get("X-Echoed-By") != "backend-echo" {
		t.Errorf("missing X-Echoed-By marker")
	}
}

func TestEchoWorksWithoutIdentity(t *testing.T) {
	req := httptest.NewRequest(http.MethodGet, "/", nil)
	rec := httptest.NewRecorder()
	newTestHandler().ServeHTTP(rec, req)
	if rec.Code != http.StatusOK {
		t.Fatalf("status = %d", rec.Code)
	}
	var resp EchoResponse
	_ = json.NewDecoder(rec.Body).Decode(&resp)
	if resp.Identity.Subject != "" || resp.Identity.Roles != nil {
		t.Errorf("identity should be empty: %+v", resp.Identity)
	}
}
