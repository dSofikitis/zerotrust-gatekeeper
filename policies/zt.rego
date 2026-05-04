# Placeholder package so `opa fmt` / `opa test` have something to
# evaluate from commit 1. Real rules (tenants, methods, geo) land in
# phase 7 alongside the gateway's OPA-integration layer.

package zt.authz

default allow := false

allow if {
	input.scaffold == true
}
