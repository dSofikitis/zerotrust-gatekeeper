# Tenant scoping: a request can only touch resources belonging to
# the tenant claim in its JWT.
#
# Path shape used by the gateway is /tenants/<tenant_id>/<...>;
# any other path is allowed past this rule (other rules will judge it).

package zt.authz

import rego.v1

allow_tenant_scope if {
	# request isn't tenant-scoped — let other rules handle it
	not is_tenant_scoped(input.path)
}

allow_tenant_scope if {
	# request is tenant-scoped and matches the JWT's tenant claim
	is_tenant_scoped(input.path)
	input.path[1] == input.claims.tenant
}

is_tenant_scoped(path) if {
	count(path) >= 2
	path[0] == "tenants"
}
