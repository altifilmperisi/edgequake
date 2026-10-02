"use client";

import { Button } from "@/components/ui/button";
import { X } from "lucide-react";
import { useTranslation } from "react-i18next";
import type { AttachedImage } from "@/lib/query/query-interface-types";

interface AttachmentTrayProps {
  images: AttachedImage[];
  onRemove: (index: number) => void;
}

export function AttachmentTray({ images, onRemove }: AttachmentTrayProps) {
  const { t } = useTranslation();
  if (images.length === 0) return null;

  return (
    <div
      className="flex flex-wrap gap-2 mb-2 min-h-16"
      role="list"
      aria-label={t("query.attachedImages", "Attached images")}
      data-testid="spec100-query-attachments-slot"
    >
      {images.map((img, idx) => (
        <div
          key={idx}
          role="listitem"
          className="relative group w-16 h-16 rounded border overflow-hidden flex-shrink-0"
        >
          {/* eslint-disable-next-line @next/next/no-img-element */}
          <img
            src={img.preview}
            alt={t("query.attachmentAlt", "Attachment {{n}}", { n: idx + 1 })}
            className="w-full h-full object-cover"
          />
          <button
            type="button"
            onClick={() => onRemove(idx)}
            className="absolute top-0.5 right-0.5 bg-black/60 rounded-full p-0.5 opacity-0 group-hover:opacity-100 focus:opacity-100 transition-opacity"
            aria-label={t("query.removeImage", "Remove image {{n}}", {
              n: idx + 1,
            })}
          >
            <X className="h-3 w-3 text-white" aria-hidden="true" />
          </button>
        </div>
      ))}
    </div>
  );
}
