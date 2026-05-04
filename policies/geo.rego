# Geo-blocking: drop requests whose JWT carries a `country` claim
# in the configured block list. Operators can adjust the list at
# runtime via the `data.zt.config.blocked_countries` document.

package zt.authz

import data.zt.config
import rego.v1

allow_geo if {
	# JWT didn't say where the caller is — trust other layers
	not input.claims.country
}

allow_geo if {
	input.claims.country
	not is_blocked(input.claims.country)
}

is_blocked(country) if {
	country == config.blocked_countries[_]
}
