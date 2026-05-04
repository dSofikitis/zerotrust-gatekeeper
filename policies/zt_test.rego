package zt.authz_test

import data.zt.authz

test_default_deny if {
	not authz.allow with input as {}
}

test_scaffold_allow if {
	authz.allow with input as {"scaffold": true}
}
