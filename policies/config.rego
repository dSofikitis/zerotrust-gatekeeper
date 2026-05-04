# Operator-tunable data. In a real deployment this lives in a
# bundle that operations rolls out independently of the rule code.

package zt.config

# Country codes that the geo rule rejects outright.
blocked_countries := ["KP", "IR"]
