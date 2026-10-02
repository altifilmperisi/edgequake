'use client';

import { Badge } from '@/components/ui/badge';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import {
  formatServerDefaultPdfParserLabel,
  getServerDefaultPdfParserBackend,
  pdfParserBackendDisplayName,
} from '@/lib/pdf/resolve-pdf-parser-backend';
import { Eye, Gauge, ScanText, Sparkles } from 'lucide-react';
import { useTranslation } from 'react-i18next';

export type PdfParserBackendChoice = 'none' | 'vision' | 'edgeparse' | 'edgeparse-ocr' | 'auto';

function backendLabel(
  value: PdfParserBackendChoice,
  t: (key: string, defaultValue: string, options?: { value: string }) => string,
) {
  switch (value) {
    case 'edgeparse':
      return t('settings.pdfParser.edgeparse', 'EdgeParse');
    case 'edgeparse-ocr':
      return t('settings.pdfParser.edgeparseOcr', 'EdgeParse + OCR');
    case 'vision':
      return t('settings.pdfParser.vision', 'Vision');
    case 'auto':
      return t('settings.pdfParser.auto', 'Auto');
    default:
      return formatServerDefaultPdfParserLabel(t);
  }
}

interface PdfParserBackendFieldProps {
  value: PdfParserBackendChoice;
  isEditing: boolean;
  onChange: (value: PdfParserBackendChoice) => void;
}

export function PdfParserBackendField({
  value,
  isEditing,
  onChange,
}: PdfParserBackendFieldProps) {
  const { t } = useTranslation();
  const serverBackend = getServerDefaultPdfParserBackend();
  const serverLabel = formatServerDefaultPdfParserLabel(t, serverBackend);
  const resolvedName = pdfParserBackendDisplayName(serverBackend);

  if (isEditing) {
    return (
      <Select value={value} onValueChange={(next) => onChange(next as PdfParserBackendChoice)}>
        <SelectTrigger className="w-full" data-testid="pdf-parser-backend-select">
          <SelectValue />
        </SelectTrigger>
        <SelectContent>
          <SelectItem value="none" data-testid="pdf-parser-option-server-default">
            {serverLabel}
          </SelectItem>
          <SelectItem value="vision">
            {t('settings.pdfParser.vision', 'Vision')}
          </SelectItem>
          <SelectItem value="edgeparse">
            {t('settings.pdfParser.edgeparse', 'EdgeParse')}
          </SelectItem>
          <SelectItem value="edgeparse-ocr" data-testid="pdf-parser-option-edgeparse-ocr">
            {t('settings.pdfParser.edgeparseOcr', 'EdgeParse + OCR')}
          </SelectItem>
          <SelectItem value="auto" data-testid="pdf-parser-option-auto">
            {t('settings.pdfParser.auto', 'Auto')}
          </SelectItem>
        </SelectContent>
      </Select>
    );
  }

  const icon =
    value === 'auto' || (value === 'none' && serverBackend === 'auto') ? (
      <Sparkles className="h-4 w-4 text-sky-600 shrink-0" />
    ) : value === 'vision' || (value === 'none' && serverBackend === 'vision') ? (
      <Eye className="h-4 w-4 text-orange-600 shrink-0" />
    ) : value === 'edgeparse-ocr' ||
      (value === 'none' && serverBackend === 'edgeparse-ocr') ? (
      <ScanText className="h-4 w-4 text-violet-600 shrink-0" />
    ) : (
      <Gauge className="h-4 w-4 text-amber-600 shrink-0" />
    );

  return (
    <div className="flex items-center gap-3 p-3 bg-muted/50 rounded-lg min-h-[3.75rem]">
      {icon}
      <div className="min-w-0 flex-1">
        <div className="font-medium" data-testid="pdf-parser-backend-label">
          {backendLabel(value, t)}
        </div>
        <div className="text-sm text-muted-foreground">
          {value === 'edgeparse'
            ? t(
                'settings.pdfParser.edgeparseHint',
                'Fast, CPU-only, best for digital-native PDFs',
              )
            : value === 'edgeparse-ocr'
              ? t(
                  'settings.pdfParser.edgeparseOcrHint',
                  'EdgeParse with Tesseract OCR for image tables (requires tesseract on PATH)',
                )
            : value === 'vision'
              ? t(
                  'settings.pdfParser.visionHint',
                  'Best for scanned and image-heavy PDFs',
                )
              : value === 'auto'
                ? t(
                    'settings.pdfParser.autoHint',
                    'May use EdgeParse for born-digital PDFs, otherwise Vision',
                  )
                : t(
                    'settings.pdfParser.serverDefaultHintWithValue',
                    'Uses server config when no workspace override is set (currently {{value}}).',
                    { value: resolvedName },
                  )}
        </div>
      </div>
      <Badge variant="outline" className="ml-auto shrink-0">
        {value === 'none'
          ? t('settings.pdfParser.resolvesTo', 'Resolves to {{value}}', {
              value: resolvedName,
            })
          : value === 'auto'
            ? t('settings.pdfParser.resolvesToAuto', 'May use EdgeParse')
            : backendLabel(value, t)}
      </Badge>
    </div>
  );
}
