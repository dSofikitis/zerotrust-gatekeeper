# Method-based authorization: GET is open to any valid identity;
# POST / PUT / DELETE / PATCH require the `admin` role.

package zt.authz

import rego.v1

read_methods := {"GET", "HEAD", "OPTIONS"}

allow_method if {
	read_methods[input.method]
}

allow_method if {
	not read_methods[input.method]
	"admin" in input.claims.roles
}
