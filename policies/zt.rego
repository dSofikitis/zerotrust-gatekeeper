# Top-level entry point. The gateway calls
#   POST /v1/data/zt/authz/allow
# per request and refuses to proceed unless `allow` is true.
#
# `allow` is the AND of three independently-tested sub-rules:
#   - allow_tenant_scope (tenants.rego)
#   - allow_method       (methods.rego)
#   - allow_geo          (geo.rego)

package zt.authz

import rego.v1

default allow := false

allow if {
	allow_tenant_scope
	allow_method
	allow_geo
}

# `reasons` returns the list of sub-rules that vetoed the request.
# Useful for the audit log so a 403 includes a precise cause.
reasons := r if {
	r := array.concat(
		array.concat(
			[m | not allow_tenant_scope; m := "tenant_scope"],
			[m | not allow_method; m := "method"],
		),
		[m | not allow_geo; m := "geo"],
	)
}
