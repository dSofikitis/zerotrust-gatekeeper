// Package server wires the HTTP routes for backend-echo. The
// service is a deliberate thin upstream: it accepts whatever the
// gateway forwards, echoes the request shape + the identity headers
// the gateway stamped, and lets us verify the full mTLS -> JWT ->
// OPA -> rate-limit -> proxy chain end to end.
package server

import (
	"encoding/json"
	"io"
	"log/slog"
	"net/http"
	"strings"
	"time"
)

// New returns a handler with /healthz and a catch-all echo route.
func New(logger *slog.Logger) http.Handler {
	mux := http.NewServeMux()
	mux.HandleFunc("GET /healthz", handleHealth)
	mux.HandleFunc("/", handleEcho)
	return logging(logger, mux)
}

func handleHealth(w http.ResponseWriter, _ *http.Request) {
	w.Header().Set("Content-Type", "application/json")
	_, _ = w.Write([]byte(`{"status":"ok"}`))
}

// EchoResponse is what the upstream returns. Field names are stable
// so curl + jq examples in the demo script can rely on them.
type EchoResponse struct {
	Method    string            `json:"method"`
	Path      string            `json:"path"`
	Query     string            `json:"query,omitempty"`
	RequestID string            `json:"request_id,omitempty"`
	Identity  Identity          `json:"identity"`
	Headers   map[string]string `json:"headers"`
	Body      string            `json:"body,omitempty"`
}

// Identity is the bag of fields the gateway derives from the JWT and
// stamps onto forwarded requests. Empty fields when no JWT was
// proxied through (e.g. a direct dev-mode hit).
type Identity struct {
	Subject string   `json:"sub,omitempty"`
	Tenant  string   `json:"tenant,omitempty"`
	Roles   []string `json:"roles,omitempty"`
	Country string   `json:"country,omitempty"`
}

func handleEcho(w http.ResponseWriter, r *http.Request) {
	defer r.Body.Close()
	body, _ := io.ReadAll(io.LimitReader(r.Body, 1<<20)) // 1 MiB cap

	resp := EchoResponse{
		Method:    r.Method,
		Path:      r.URL.Path,
		Query:     r.URL.RawQuery,
		RequestID: r.Header.Get("X-Request-Id"),
		Identity: Identity{
			Subject: r.Header.Get("X-Auth-Subject"),
			Tenant:  r.Header.Get("X-Auth-Tenant"),
			Roles:   splitNonEmpty(r.Header.Get("X-Auth-Roles"), ","),
			Country: r.Header.Get("X-Auth-Country"),
		},
		Headers: filterHeaders(r.Header),
		Body:    string(body),
	}
	w.Header().Set("Content-Type", "application/json")
	w.Header().Set("X-Echoed-By", "backend-echo")
	_ = json.NewEncoder(w).Encode(resp)
}

// filterHeaders keeps only headers a typical demo cares about.
// Hop-by-hop fields (Connection, Keep-Alive, ...) and Authorization
// are intentionally dropped — Authorization because we don't want a
// JWT to land in audit logs by accident.
var dropHeaders = map[string]struct{}{
	"Authorization":     {},
	"Cookie":            {},
	"Connection":        {},
	"Keep-Alive":        {},
	"Proxy-Authenticate": {},
	"Proxy-Authorization": {},
	"Te":                {},
	"Trailer":           {},
	"Transfer-Encoding": {},
	"Upgrade":           {},
}

func filterHeaders(h http.Header) map[string]string {
	out := make(map[string]string, len(h))
	for k, vs := range h {
		if _, drop := dropHeaders[http.CanonicalHeaderKey(k)]; drop {
			continue
		}
		if len(vs) == 0 {
			continue
		}
		out[http.CanonicalHeaderKey(k)] = strings.Join(vs, ", ")
	}
	return out
}

func splitNonEmpty(v, sep string) []string {
	if v == "" {
		return nil
	}
	parts := strings.Split(v, sep)
	out := parts[:0]
	for _, p := range parts {
		p = strings.TrimSpace(p)
		if p != "" {
			out = append(out, p)
		}
	}
	return out
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
