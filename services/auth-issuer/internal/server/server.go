// Package server wires the HTTP routes for auth-issuer.
package server

import (
	"encoding/json"
	"log/slog"
	"net/http"
	"strings"
	"time"

	"github.com/dSofikitis/zerotrust-gatekeeper/auth-issuer/internal/token"
)

// Config holds runtime knobs for the HTTP server.
type Config struct {
	TTL time.Duration
}

// New builds the auth-issuer HTTP handler:
//
//	GET  /healthz                   liveness
//	GET  /.well-known/jwks.json     verification keys
//	POST /auth/token                mint a JWT
func New(cfg Config, signer *token.Signer, logger *slog.Logger) http.Handler {
	if cfg.TTL == 0 {
		cfg.TTL = token.DefaultTTL
	}
	mux := http.NewServeMux()
	mux.HandleFunc("GET /healthz", handleHealth)
	mux.HandleFunc("GET /.well-known/jwks.json", handleJWKS(signer))
	mux.HandleFunc("POST /auth/token", handleToken(cfg, signer, logger))
	return logging(logger, mux)
}

// ---------- handlers ----------

func handleHealth(w http.ResponseWriter, _ *http.Request) {
	w.Header().Set("Content-Type", "application/json")
	_, _ = w.Write([]byte(`{"status":"ok"}`))
}

func handleJWKS(signer *token.Signer) http.HandlerFunc {
	return func(w http.ResponseWriter, _ *http.Request) {
		bs, err := signer.JWKS()
		if err != nil {
			writeJSONError(w, http.StatusInternalServerError, "jwks unavailable")
			return
		}
		w.Header().Set("Content-Type", "application/json")
		w.Header().Set("Cache-Control", "public, max-age=300")
		_, _ = w.Write(bs)
	}
}

// TokenRequest is the JSON body for POST /auth/token. The dev IdP
// accepts any non-empty username; password (when provided) is
// deliberately not validated here so the demo can stand on its own
// without a user store.
type TokenRequest struct {
	Username string `json:"username"`
	Password string `json:"password,omitempty"`
	Tenant   string `json:"tenant,omitempty"`
	Roles    []string `json:"roles,omitempty"`
	Country  string `json:"country,omitempty"`
}

// TokenResponse mirrors RFC 6749 §5.1 (minus refresh tokens).
type TokenResponse struct {
	AccessToken string `json:"access_token"`
	TokenType   string `json:"token_type"`
	ExpiresIn   int    `json:"expires_in"`
	Kid         string `json:"kid"`
}

func handleToken(cfg Config, signer *token.Signer, logger *slog.Logger) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		defer r.Body.Close()
		var req TokenRequest
		if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
			writeJSONError(w, http.StatusBadRequest, "invalid json: "+err.Error())
			return
		}
		req.Username = strings.TrimSpace(req.Username)
		if req.Username == "" {
			writeJSONError(w, http.StatusBadRequest, "username required")
			return
		}
		c := token.Claims{
			Tenant:  defaultStr(req.Tenant, deriveTenant(req.Username)),
			Roles:   defaultRoles(req.Roles, req.Username),
			Country: defaultStr(req.Country, "GR"),
		}
		signed, err := signer.Mint(req.Username, c, cfg.TTL)
		if err != nil {
			logger.Error("mint failed", "error", err, "user", req.Username)
			writeJSONError(w, http.StatusInternalServerError, "mint failed")
			return
		}
		resp := TokenResponse{
			AccessToken: signed,
			TokenType:   "Bearer",
			ExpiresIn:   int(cfg.TTL.Seconds()),
			Kid:         signer.Kid(),
		}
		w.Header().Set("Content-Type", "application/json")
		_ = json.NewEncoder(w).Encode(resp)
	}
}

// ---------- helpers ----------

func defaultStr(v, fallback string) string {
	if strings.TrimSpace(v) != "" {
		return v
	}
	return fallback
}

func defaultRoles(roles []string, username string) []string {
	if len(roles) > 0 {
		return roles
	}
	if username == "admin" {
		return []string{"admin"}
	}
	return []string{"user"}
}

// deriveTenant is dev sugar so curl examples look natural:
//
//	alice / bob -> acme
//	carl  / dan -> globex
//	anything else -> acme
//
// Real prod hands the tenant via the IdP's user record.
func deriveTenant(username string) string {
	switch username {
	case "carl", "dan":
		return "globex"
	default:
		return "acme"
	}
}

func writeJSONError(w http.ResponseWriter, status int, message string) {
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(status)
	_ = json.NewEncoder(w).Encode(map[string]string{"error": message})
}

// ---------- middleware ----------

func logging(logger *slog.Logger, next http.Handler) http.Handler {
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		start := time.Now()
		rec := &statusRecorder{ResponseWriter: w, status: http.StatusOK}
		next.ServeHTTP(rec, r)
		logger.Info(
			"http",
			"method", r.Method,
			"path", r.URL.Path,
			"status", rec.status,
			"duration_ms", time.Since(start).Milliseconds(),
		)
	})
}

type statusRecorder struct {
	http.ResponseWriter
	status int
}

func (r *statusRecorder) WriteHeader(code int) {
	r.status = code
	r.ResponseWriter.WriteHeader(code)
}
