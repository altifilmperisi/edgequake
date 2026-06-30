-- EdgeQuake Migration 039: Verify AGE Graph Labels (Node, EDGE)
--
-- These labels are created by the `init-graph-labels.sh` startup script
-- which runs LOAD 'age' + cypher CREATE directly (outside sqlx).
--
-- This migration only verifies the labels exist via ag_catalog tables,
-- which do not require LOAD 'age'. If labels are missing, it logs a
-- NOTICE; the startup script will handle creation on next restart.

DO $$
DECLARE
    graph_name TEXT := 'eq_eq_default_graph';
    graph_id oid;
    node_ok BOOLEAN := false;
    edge_ok BOOLEAN := false;
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_extension WHERE extname = 'age') THEN
        RAISE NOTICE 'AGE extension not available, skipping label check';
        RETURN;
    END IF;

    SELECT graphid INTO graph_id FROM ag_catalog.ag_graph WHERE name = graph_name;
    IF graph_id IS NULL THEN
        RAISE NOTICE 'Graph % does not exist yet, skipping label check', graph_name;
        RETURN;
    END IF;

    SELECT EXISTS(
        SELECT 1 FROM ag_catalog.ag_label
        WHERE graph = graph_id AND name = 'Node'
    ) INTO node_ok;

    SELECT EXISTS(
        SELECT 1 FROM ag_catalog.ag_label
        WHERE graph = graph_id AND name = 'EDGE'
    ) INTO edge_ok;

    IF node_ok AND edge_ok THEN
        RAISE NOTICE 'Migration 039 OK: Node(ready) EDGE(ready)';
    ELSE
        RAISE WARNING 'Migration 039: labels not found (Node=%, EDGE=%); run init-graph-labels.sh',
            node_ok, edge_ok;
    END IF;
END $$;
