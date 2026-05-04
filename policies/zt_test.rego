# Tests for the three sub-rules and the composed allow.
# Run via `opa test policies/`.

package zt.authz_test

import data.zt.authz
import rego.v1

# ---------- tenant scoping ----------

test_tenant_scope_passes_for_matching_tenant if {
	authz.allow_tenant_scope with input as {
		"path": ["tenants", "acme", "users"],
		"claims": {"tenant": "acme", "roles": ["user"]},
	}
}

test_tenant_scope_blocks_cross_tenant if {
	not authz.allow_tenant_scope with input as {
		"path": ["tenants", "acme", "users"],
		"claims": {"tenant": "globex", "roles": ["user"]},
	}
}

test_tenant_scope_passes_for_unscoped_path if {
	# /api/echo is not /tenants/<x>/... so the rule abstains.
	authz.allow_tenant_scope with input as {
		"path": ["api", "echo"],
		"claims": {"tenant": "acme", "roles": ["user"]},
	}
}

# ---------- methods ----------

test_method_get_open_to_users if {
	authz.allow_method with input as {
		"method": "GET",
		"claims": {"roles": ["user"]},
	}
}

test_method_post_blocked_for_users if {
	not authz.allow_method with input as {
		"method": "POST",
		"claims": {"roles": ["user"]},
	}
}

test_method_post_allowed_for_admins if {
	authz.allow_method with input as {
		"method": "POST",
		"claims": {"roles": ["admin"]},
	}
}

test_method_delete_blocked_for_users if {
	not authz.allow_method with input as {
		"method": "DELETE",
		"claims": {"roles": ["user"]},
	}
}

# ---------- geo ----------

test_geo_passes_when_country_absent if {
	authz.allow_geo with input as {"claims": {"roles": ["user"]}}
}

test_geo_passes_for_safe_country if {
	authz.allow_geo with input as {"claims": {"roles": ["user"], "country": "GR"}}
}

test_geo_blocks_listed_country if {
	not authz.allow_geo with input as {"claims": {"roles": ["user"], "country": "KP"}}
}

# ---------- composite allow ----------

test_allow_for_in_tenant_get_user if {
	authz.allow with input as {
		"method": "GET",
		"path": ["tenants", "acme", "users"],
		"claims": {"tenant": "acme", "roles": ["user"], "country": "GR"},
	}
}

test_allow_blocks_admin_on_wrong_tenant if {
	not authz.allow with input as {
		"method": "DELETE",
		"path": ["tenants", "globex", "users", "alice"],
		"claims": {"tenant": "acme", "roles": ["admin"], "country": "GR"},
	}
}

test_default_deny if {
	not authz.allow with input as {}
}

# ---------- reasons ----------

test_reasons_lists_failed_rules if {
	got := authz.reasons with input as {
		"method": "DELETE",
		"path": ["tenants", "globex", "users"],
		"claims": {"tenant": "acme", "roles": ["user"], "country": "KP"},
	}
	"tenant_scope" in got
	"method" in got
	"geo" in got
}
