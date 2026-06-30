#!/usr/bin/env bash
# Ensure AGE graph labels (Node, EDGE) exist eagerly.
set -euo pipefail

DB_URL="${DATABASE_URL:-postgresql://edgequake:edgequake_secret@localhost:5432/edgequake}"
GRAPH_NAME="${EDGEQUAKE_GRAPH_NAME:-eq_eq_default_graph}"

TMP="${DB_URL#*://}" ; USER="${TMP%%:*}"; TMP="${TMP#*:}"
PASS="${TMP%%@*}"; TMP="${TMP#*@}"
HOST="${TMP%%:*}"; TMP="${TMP#*:}"
PORT="${TMP%%/*}"; DB="${TMP#*/}"; DB="${DB%%\?*}"

export PGPASSWORD="$PASS"
PSQL="psql -h $HOST -p $PORT -U $USER -d $DB -t -A"

# Check current labels
LABELS=$($PSQL -c "LOAD 'age'; SET search_path = ag_catalog, \"\$user\", public; 
  SELECT l.name FROM ag_label l JOIN ag_graph g ON l.graph = g.graphid
  WHERE g.name = '${GRAPH_NAME}' AND l.name IN ('Node','EDGE') ORDER BY l.name;")

HAS_NODE=$(echo "$LABELS" | grep -c "Node" || true)
HAS_EDGE=$(echo "$LABELS" | grep -c "EDGE" || true)

if [ "$HAS_NODE" -eq 0 ]; then
  echo "Creating Node label..."
  $PSQL -c "LOAD 'age'; SET search_path = ag_catalog, \"\$user\", public;
    SELECT * FROM cypher('${GRAPH_NAME}', \$\$ CREATE (n:Node {node_id: '__seed__'}) \$\$) AS (n agtype);
    SELECT * FROM cypher('${GRAPH_NAME}', \$\$ MATCH (n:Node {node_id: '__seed__'}) DETACH DELETE n \$\$) AS (n agtype);"
  echo "  Node label created"
fi

if [ "$HAS_EDGE" -eq 0 ]; then
  echo "Creating EDGE label..."
  $PSQL -c "LOAD 'age'; SET search_path = ag_catalog, \"\$user\", public;
    SELECT * FROM cypher('${GRAPH_NAME}', \$\$ CREATE (n:Node {node_id: '__seed_a__'}) \$\$) AS (n agtype);
    SELECT * FROM cypher('${GRAPH_NAME}', \$\$ CREATE (n:Node {node_id: '__seed_b__'}) \$\$) AS (n agtype);
    SELECT * FROM cypher('${GRAPH_NAME}', \$\$ MATCH (a:Node {node_id: '__seed_a__'}), (b:Node {node_id: '__seed_b__'}) CREATE (a)-[r:EDGE {}]->(b) \$\$) AS (r agtype);
    SELECT * FROM cypher('${GRAPH_NAME}', \$\$ MATCH (n:Node {node_id: '__seed_a__'}) DETACH DELETE n \$\$) AS (n agtype);
    SELECT * FROM cypher('${GRAPH_NAME}', \$\$ MATCH (n:Node {node_id: '__seed_b__'}) DETACH DELETE n \$\$) AS (n agtype);"
  echo "  EDGE label created"
fi

echo "✅ Graph labels OK (Node=$([ $HAS_NODE -eq 0 ] && echo NEW || echo EXISTS), EDGE=$([ $HAS_EDGE -eq 0 ] && echo NEW || echo EXISTS))"
