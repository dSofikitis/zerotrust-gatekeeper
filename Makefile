.PHONY: help all build test lint clean \
        build-gateway build-auth-issuer build-backend-echo \
        test-gateway test-auth-issuer test-backend-echo test-policies \
        lint-rust lint-go lint-rego \
        compose-up compose-down compose-build compose-logs \
        certs demo

help: ## list targets
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | awk -F':.*?## ' '{printf "  %-22s %s\n", $$1, $$2}'

all: lint test ## lint and test all components

# ---------- build ----------

build: build-gateway build-auth-issuer build-backend-echo ## build all services

build-gateway:
	cd services/gateway && cargo build --release

build-auth-issuer:
	cd services/auth-issuer && go build ./...

build-backend-echo:
	cd services/backend-echo && go build ./...

# ---------- test ----------

test: test-gateway test-auth-issuer test-backend-echo test-policies ## run all tests

test-gateway:
	cd services/gateway && cargo test

test-auth-issuer:
	cd services/auth-issuer && go test ./...

test-backend-echo:
	cd services/backend-echo && go test ./...

test-policies:
	opa test policies/

# ---------- lint ----------

lint: lint-rust lint-go lint-rego ## lint all components

lint-rust:
	cd services/gateway && cargo fmt --check && cargo clippy --all-targets -- -D warnings

lint-go:
	cd services/auth-issuer  && go vet ./...
	cd services/backend-echo && go vet ./...

lint-rego:
	opa fmt --diff policies/

# ---------- compose ----------

compose-up: ## bring up the full stack (gateway + auth-issuer + backend-echo + opa + redis + prometheus + grafana)
	docker compose -f deploy/compose/docker-compose.yml up -d

compose-down: ## tear down (keeps volumes)
	docker compose -f deploy/compose/docker-compose.yml down

compose-build: ## (re)build container images
	docker compose -f deploy/compose/docker-compose.yml build

compose-logs:
	docker compose -f deploy/compose/docker-compose.yml logs -f

# ---------- demo ----------

certs: ## generate dev mTLS certs into certs/
	bash scripts/gen-certs.sh

demo: ## end-to-end walkthrough (token, allow, deny, rate-limit, audit)
	bash scripts/demo.sh

# ---------- clean ----------

clean: ## remove build artifacts
	cd services/gateway && cargo clean || true
