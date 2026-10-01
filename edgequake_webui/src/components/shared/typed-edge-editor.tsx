'use client';

/**
 * @module TypedEdgeEditor
 * @description Compact CRUD for typed edges (source — relation → target) + entity lens.
 * @implements SPEC-114b LAW-114-9…13
 */

import { Button } from '@/components/ui/button';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import {
  MAX_RELATION_EDGES,
  edgeKey,
  normalizeRelationEdge,
  type RelationEdge,
} from '@/constants/kg-schema-presets';
import { cn } from '@/lib/utils';
import { Pencil, Plus, Trash2, X } from 'lucide-react';
import { useMemo, useState } from 'react';
import { useTranslation } from 'react-i18next';

export interface TypedEdgeEditorProps {
  entityTypes: string[];
  relationTypes: string[];
  edges: RelationEdge[];
  onChange: (edges: RelationEdge[]) => void;
  /** Optional: when adding an edge whose relation is missing, call with updated list. */
  onRelationTypesChange?: (relationTypes: string[]) => void;
}

export function TypedEdgeEditor({
  entityTypes,
  relationTypes,
  edges,
  onChange,
  onRelationTypesChange,
}: TypedEdgeEditorProps) {
  const { t } = useTranslation();
  const [lens, setLens] = useState<string>('all');
  const [source, setSource] = useState('');
  const [relation, setRelation] = useState('');
  const [target, setTarget] = useState('');
  const [editingKey, setEditingKey] = useState<string | null>(null);

  const entities = useMemo(
    () => [...new Set(entityTypes.filter(Boolean))],
    [entityTypes],
  );
  const relations = useMemo(
    () => [...new Set(relationTypes.filter(Boolean))],
    [relationTypes],
  );

  const filtered = useMemo(() => {
    if (lens === 'all') return edges;
    return edges.filter((e) => e.source === lens || e.target === lens);
  }, [edges, lens]);

  const resetForm = () => {
    setSource('');
    setRelation('');
    setTarget('');
    setEditingKey(null);
  };

  const commitEdge = () => {
    const next = normalizeRelationEdge({ source, relation, target });
    if (!next) return;
    if (!entities.includes(next.source) || !entities.includes(next.target)) return;
    if (!relations.includes(next.relation)) {
      if (onRelationTypesChange && relations.length < 50) {
        onRelationTypesChange([...relations, next.relation]);
      } else if (!relations.includes(next.relation)) {
        return;
      }
    }
    const key = edgeKey(next);
    let updated: RelationEdge[];
    if (editingKey) {
      updated = edges
        .filter((e) => edgeKey(e) !== editingKey)
        .concat(next)
        .filter((e, i, arr) => arr.findIndex((x) => edgeKey(x) === edgeKey(e)) === i);
    } else {
      if (edges.some((e) => edgeKey(e) === key)) {
        resetForm();
        return;
      }
      if (edges.length >= MAX_RELATION_EDGES) return;
      updated = [...edges, next];
    }
    onChange(updated);
    resetForm();
  };

  const startEdit = (edge: RelationEdge) => {
    setSource(edge.source);
    setRelation(edge.relation);
    setTarget(edge.target);
    setEditingKey(edgeKey(edge));
  };

  const removeEdge = (edge: RelationEdge) => {
    const key = edgeKey(edge);
    onChange(edges.filter((e) => edgeKey(e) !== key));
    if (editingKey === key) resetForm();
  };

  const canAdd =
    Boolean(source && relation && target) &&
    entities.length > 0 &&
    (relations.length > 0 || Boolean(onRelationTypesChange));

  return (
    <section
      className="rounded-lg border bg-muted/20 p-3 space-y-2 flex flex-col min-h-0"
      data-testid="typed-edge-editor"
      aria-label={t('kgSchema.typedEdgesLabel', 'Typed edges')}
    >
      <div className="flex items-start justify-between gap-2 shrink-0">
        <div className="min-w-0 space-y-0.5">
          <h4 className="text-sm font-medium">
            {t('kgSchema.typedEdgesHeading', 'Typed edges')}
          </h4>
          <p className="text-xs text-muted-foreground">
            {t(
              'kgSchema.typedEdgesHint',
              'Associate relations with source and target entity types.',
            )}
          </p>
        </div>
        <span className="text-xs text-muted-foreground tabular-nums shrink-0 pt-0.5">
          {edges.length}/{MAX_RELATION_EDGES}
        </span>
      </div>

      {/* Entity lens */}
      <div
        className="flex flex-wrap gap-1 max-h-12 overflow-y-auto shrink-0"
        data-testid="typed-edge-lens"
      >
        <button
          type="button"
          onClick={() => setLens('all')}
          className={cn(
            'rounded-md border px-1.5 py-0.5 text-xs',
            lens === 'all'
              ? 'border-foreground/30 bg-accent'
              : 'border-border bg-background',
          )}
          data-testid="typed-edge-lens-all"
        >
          {t('kgSchema.lensAll', 'All')}
        </button>
        {entities.slice(0, 16).map((ent) => (
          <button
            key={ent}
            type="button"
            onClick={() => setLens(ent)}
            className={cn(
              'rounded-md border px-1.5 py-0.5 text-xs font-mono',
              lens === ent
                ? 'border-foreground/30 bg-accent'
                : 'border-border bg-background',
            )}
            data-testid={`typed-edge-lens-${ent}`}
          >
            {ent}
          </button>
        ))}
      </div>

      <ul
        className="space-y-1 overflow-y-auto min-h-[4.5rem] max-h-40"
        data-testid="typed-edge-list"
      >
        {filtered.length === 0 ? (
          <li className="text-xs text-muted-foreground italic py-2">
            {edges.length === 0
              ? t(
                  'kgSchema.typedEdgesEmpty',
                  'No typed edges. Without edges, any listed entity may use any listed relation.',
                )
              : t('kgSchema.typedEdgesLensEmpty', 'No edges for this entity type.')}
          </li>
        ) : (
          filtered.map((edge) => {
            const key = edgeKey(edge);
            return (
              <li
                key={key}
                className="flex items-center gap-1 rounded-md border bg-background px-2 py-1 text-xs font-mono"
                data-testid={`typed-edge-row-${key}`}
              >
                <span className="truncate flex-1 min-w-0">
                  <span className="text-foreground/80">{edge.source}</span>
                  <span className="mx-0.5 text-muted-foreground">—</span>
                  <span className="text-foreground">{edge.relation}</span>
                  <span className="mx-0.5 text-muted-foreground">→</span>
                  <span className="text-foreground/80">{edge.target}</span>
                </span>
                <Button
                  type="button"
                  size="icon"
                  variant="ghost"
                  className="h-6 w-6 shrink-0"
                  onClick={() => startEdit(edge)}
                  aria-label={t('kgSchema.editEdge', 'Edit edge')}
                >
                  <Pencil className="h-3 w-3" />
                </Button>
                <Button
                  type="button"
                  size="icon"
                  variant="ghost"
                  className="h-6 w-6 shrink-0"
                  onClick={() => removeEdge(edge)}
                  aria-label={t('kgSchema.deleteEdge', 'Delete edge')}
                  data-testid={`typed-edge-delete-${key}`}
                >
                  <Trash2 className="h-3 w-3" />
                </Button>
              </li>
            );
          })
        )}
      </ul>

      <div
        className="grid grid-cols-1 gap-1.5 shrink-0 pt-1 border-t"
        data-testid="typed-edge-form"
      >
        <div className="grid grid-cols-3 gap-1">
          <Select value={source || undefined} onValueChange={setSource}>
            <SelectTrigger className="h-8 text-xs w-full" data-testid="typed-edge-source">
              <SelectValue placeholder={t('kgSchema.source', 'Source')} />
            </SelectTrigger>
            <SelectContent side="top" position="popper" className="max-h-56">
              {entities.map((e) => (
                <SelectItem key={e} value={e} className="text-xs font-mono">
                  {e}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
          <Select value={relation || undefined} onValueChange={setRelation}>
            <SelectTrigger className="h-8 text-xs w-full" data-testid="typed-edge-relation">
              <SelectValue placeholder={t('kgSchema.relation', 'Relation')} />
            </SelectTrigger>
            <SelectContent side="top" position="popper" className="max-h-56">
              {relations.map((r) => (
                <SelectItem key={r} value={r} className="text-xs font-mono">
                  {r}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
          <Select value={target || undefined} onValueChange={setTarget}>
            <SelectTrigger className="h-8 text-xs w-full" data-testid="typed-edge-target">
              <SelectValue placeholder={t('kgSchema.target', 'Target')} />
            </SelectTrigger>
            <SelectContent side="top" position="popper" className="max-h-56">
              {entities.map((e) => (
                <SelectItem key={e} value={e} className="text-xs font-mono">
                  {e}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </div>
        <div className="flex gap-1">
          <Button
            type="button"
            size="sm"
            className="h-7 text-xs flex-1"
            disabled={!canAdd}
            onClick={commitEdge}
            data-testid="typed-edge-add"
          >
            <Plus className="h-3 w-3 mr-1" />
            {editingKey
              ? t('kgSchema.saveEdge', 'Save')
              : t('kgSchema.addEdge', 'Add edge')}
          </Button>
          {editingKey ? (
            <Button
              type="button"
              size="sm"
              variant="ghost"
              className="h-7 text-xs"
              onClick={resetForm}
            >
              <X className="h-3 w-3" />
            </Button>
          ) : null}
          {edges.length > 0 ? (
            <Button
              type="button"
              size="sm"
              variant="ghost"
              className="h-7 text-xs"
              onClick={() => onChange([])}
              data-testid="typed-edge-clear"
            >
              {t('kgSchema.clearEdges', 'Clear')}
            </Button>
          ) : null}
        </div>
      </div>
    </section>
  );
}

export default TypedEdgeEditor;
