package main

import (
	"context"
	"errors"
	"log/slog"
	"net/http"
	"os"
	"os/signal"
	"strconv"
	"syscall"
	"time"

	"github.com/dSofikitis/zerotrust-gatekeeper/auth-issuer/internal/server"
	"github.com/dSofikitis/zerotrust-gatekeeper/auth-issuer/internal/token"
)

// Version is bumped per release.
const Version = "0.2.0"

func main() {
	logger := slog.New(slog.NewJSONHandler(os.Stdout, &slog.HandlerOptions{Level: slog.LevelInfo}))

	addr := getenv("AUTH_ADDR", ":8081")
	ttl := envDuration("AUTH_TOKEN_TTL", 15*time.Minute)

	signer, err := token.NewSigner()
	if err != nil {
		logger.Error("signer init failed", "error", err)
		os.Exit(1)
	}

	handler := server.New(server.Config{TTL: ttl}, signer, logger)

	srv := &http.Server{
		Addr:              addr,
		Handler:           handler,
		ReadHeaderTimeout: 5 * time.Second,
	}

	logger.Info(
		"auth-issuer starting",
		"version", Version,
		"addr", addr,
		"kid", signer.Kid(),
		"ttl", ttl.String(),
	)

	errCh := make(chan error, 1)
	go func() { errCh <- srv.ListenAndServe() }()

	stop := make(chan os.Signal, 1)
	signal.Notify(stop, syscall.SIGINT, syscall.SIGTERM)

	select {
	case err := <-errCh:
		if err != nil && !errors.Is(err, http.ErrServerClosed) {
			logger.Error("server crashed", "error", err)
			os.Exit(1)
		}
	case sig := <-stop:
		logger.Info("shutting down", "signal", sig.String())
		ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
		defer cancel()
		if err := srv.Shutdown(ctx); err != nil {
			logger.Error("shutdown error", "error", err)
		}
	}
}

func getenv(key, fallback string) string {
	if v := os.Getenv(key); v != "" {
		return v
	}
	return fallback
}

func envDuration(key string, fallback time.Duration) time.Duration {
	v := os.Getenv(key)
	if v == "" {
		return fallback
	}
	if n, err := strconv.Atoi(v); err == nil {
		return time.Duration(n) * time.Second
	}
	if d, err := time.ParseDuration(v); err == nil {
		return d
	}
	return fallback
}
