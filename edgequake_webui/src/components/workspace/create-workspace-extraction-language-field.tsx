/**
 * Optional extraction language select for create/reconfigure wizards (SPEC-096).
 */
'use client';

import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import {
  EXTRACTION_LANGUAGES,
  EXTRACTION_LANGUAGE_SERVER_DEFAULT,
  formatServerDefaultExtractionLanguageLabel,
} from '@/constants/extraction-languages';
import { useTranslation } from 'react-i18next';

export interface CreateWorkspaceExtractionLanguageFieldProps {
  value: string | null;
  onChange: (language: string | null) => void;
  /** Hide long description; tighten vertical rhythm for wizard steps. */
  compact?: boolean;
}

export function CreateWorkspaceExtractionLanguageField({
  value,
  onChange,
  compact = false,
}: CreateWorkspaceExtractionLanguageFieldProps) {
  const { t } = useTranslation();
  const selectValue =
    value && value !== EXTRACTION_LANGUAGE_SERVER_DEFAULT
      ? value
      : EXTRACTION_LANGUAGE_SERVER_DEFAULT;
  const serverDefaultLabel = formatServerDefaultExtractionLanguageLabel(t);

  return (
    <div className={compact ? 'space-y-1.5' : 'space-y-2'}>
      {compact ? (
        <div className="flex flex-wrap items-baseline justify-between gap-x-3 gap-y-0.5">
          <label className="text-sm font-medium">
            {t('workspace.extractionLanguage.title', 'Extraction Language')}
          </label>
          <p className="text-xs text-muted-foreground">
            {t(
              'workspace.extractionLanguage.descriptionShort',
              'Names and relationship text language.',
            )}
          </p>
        </div>
      ) : (
        <>
          <label className="text-sm font-medium">
            {t('workspace.extractionLanguage.title', 'Extraction Language')}
          </label>
          <p className="text-xs text-muted-foreground">
            {t(
              'workspace.extractionLanguage.description',
              'Language used for entity names, descriptions, and relationship text during extraction.',
            )}
          </p>
        </>
      )}
      <Select
        value={selectValue}
        onValueChange={(v) => {
          onChange(v === EXTRACTION_LANGUAGE_SERVER_DEFAULT ? null : v);
        }}
      >
        <SelectTrigger
          className={compact ? 'w-full h-9' : 'w-full'}
          data-testid="create-workspace-extraction-language"
        >
          <SelectValue
            placeholder={serverDefaultLabel}
          />
        </SelectTrigger>
        <SelectContent>
          <SelectItem value={EXTRACTION_LANGUAGE_SERVER_DEFAULT}>
            {serverDefaultLabel}
          </SelectItem>
          {EXTRACTION_LANGUAGES.map((lang) => (
            <SelectItem key={lang} value={lang}>
              {lang}
            </SelectItem>
          ))}
        </SelectContent>
      </Select>
    </div>
  );
}
