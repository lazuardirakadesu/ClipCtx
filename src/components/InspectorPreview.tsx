import React, { useState, useEffect } from 'react';
import { ClipItem } from '../types';
import {
  Link,
  FileText,
  Pin,
  Trash2,
  Copy,
  ExternalLink,
  CornerDownLeft,
  Check,
  Image as ImageIcon,
} from 'lucide-react';
import {
  formatAppDisplayName,
  parseHexColor,
  tryFormatJson,
} from '../utils/formatters';
import { openUrl } from '@tauri-apps/plugin-opener';
import { convertFileSrc, invoke } from '@tauri-apps/api/core';

interface InspectorPreviewProps {
  clip: ClipItem | null;
  onPaste: () => void;
  onCopy: () => void;
  onTogglePin: () => void;
  onDelete: () => void;
}

interface ImageMetadata {
  width: number;
  height: number;
  file_size: number;
}

export const InspectorPreview: React.FC<InspectorPreviewProps> = ({
  clip,
  onPaste,
  onCopy,
  onTogglePin,
  onDelete,
}) => {
  const [copied, setCopied] = useState(false);
  const [imgMeta, setImgMeta] = useState<ImageMetadata | null>(null);

  useEffect(() => {
    if (clip && clip.content_type === 'image') {
      invoke<ImageMetadata>('get_image_metadata', { path: clip.content })
        .then(setImgMeta)
        .catch(() => setImgMeta(null));
    } else {
      setImgMeta(null);
    }
  }, [clip]);

  if (!clip) {
    return (
      <div className="flex flex-col items-center justify-center h-full p-6 text-center text-zinc-400 dark:text-zinc-600 select-none">
        <FileText className="w-10 h-10 mb-2 stroke-[1.25] opacity-50" />
        <p className="text-sm font-medium">Select a clip to inspect</p>
        <p className="text-xs text-zinc-400/80 mt-1">Use ↑ and ↓ arrows to navigate</p>
      </div>
    );
  }

  const handleCopy = () => {
    onCopy();
    setCopied(true);
    setTimeout(() => setCopied(false), 1500);
  };

  const handleOpenLink = () => {
    if (clip.content_type === 'url') {
      try {
        openUrl(clip.content.trim());
      } catch {
        window.open(clip.content.trim(), '_blank');
      }
    }
  };

  const handleOpenImage = () => {
    if (clip.content_type === 'image') {
      try {
        openUrl(clip.content);
      } catch (e) {
        console.error('Failed to open image file:', e);
      }
    }
  };

  const lines = clip.content.split('\n');
  const lineCount = lines.length;
  const charCount = clip.content.length;

  const formatFileSize = (bytes: number) => {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    return `${(bytes / (1024 * 1024)).toFixed(2)} MB`;
  };

  return (
    <div className="flex flex-col h-full overflow-hidden bg-white/30 dark:bg-zinc-900/30">
      {/* Top Inspector Bar */}
      <div className="flex items-center justify-between px-3 py-2 border-b border-black/5 dark:border-white/5 bg-black/[0.01] dark:bg-white/[0.01] shrink-0">
        <div className="flex items-center space-x-1.5">
          <span className="px-2 py-0.5 text-[10px] uppercase font-bold tracking-wider rounded-md bg-black/5 dark:bg-white/10 text-zinc-600 dark:text-zinc-300">
            {clip.content_type}
          </span>
          {clip.is_pinned && (
            <span className="flex items-center space-x-1 px-1.5 py-0.5 text-[10px] rounded-md bg-amber-500/10 text-amber-600 dark:text-amber-400 font-medium">
              <Pin className="w-2.5 h-2.5 fill-current" />
              <span>Pinned</span>
            </span>
          )}
        </div>

        {/* Quick Action Icons */}
        <div className="flex items-center space-x-1">
          <button
            onClick={onPaste}
            className="flex items-center space-x-1 px-2.5 py-1 text-xs font-medium rounded-md bg-[#0067C0] text-white dark:bg-[#4CC2FF] dark:text-zinc-950 hover:opacity-90 transition-all cursor-pointer shadow-xs"
            title="Paste into active app (Enter)"
          >
            <CornerDownLeft className="w-3 h-3" />
            <span>Paste</span>
          </button>

          <button
            onClick={handleCopy}
            className="p-1.5 rounded-md text-zinc-600 dark:text-zinc-400 hover:text-zinc-900 dark:hover:text-zinc-100 hover:bg-black/5 dark:hover:bg-white/5 transition-colors cursor-pointer"
            title="Copy to clipboard (Space)"
          >
            {copied ? <Check className="w-3.5 h-3.5 text-emerald-500" /> : <Copy className="w-3.5 h-3.5" />}
          </button>

          <button
            onClick={onTogglePin}
            className={`p-1.5 rounded-md transition-colors cursor-pointer ${
              clip.is_pinned
                ? 'text-amber-500 hover:bg-amber-500/10'
                : 'text-zinc-600 dark:text-zinc-400 hover:bg-black/5 dark:hover:bg-white/5'
            }`}
            title="Toggle pin (Ctrl+P)"
          >
            <Pin className={`w-3.5 h-3.5 ${clip.is_pinned ? 'fill-current' : ''}`} />
          </button>

          <button
            onClick={onDelete}
            className="p-1.5 rounded-md text-zinc-600 dark:text-zinc-400 hover:text-red-500 hover:bg-red-500/10 dark:hover:bg-red-500/20 transition-colors cursor-pointer"
            title="Delete clip (Del)"
          >
            <Trash2 className="w-3.5 h-3.5" />
          </button>
        </div>
      </div>

      {/* Main Preview Content */}
      <div className="flex-1 p-3 overflow-y-auto select-text text-sm">
        {clip.content_type === 'image' ? (
          <ImagePreview
            content={clip.content}
            meta={imgMeta}
            onOpenImage={handleOpenImage}
            onCopy={handleCopy}
          />
        ) : clip.content_type === 'color' ? (
          <ColorPreview content={clip.content} onCopyColor={handleCopy} />
        ) : clip.content_type === 'url' ? (
          <UrlPreview content={clip.content} onOpen={handleOpenLink} />
        ) : clip.content_type === 'json' ? (
          <JsonPreview content={clip.content} />
        ) : clip.content_type === 'code' ? (
          <CodePreview lines={lines} />
        ) : (
          <TextPreview content={clip.content} />
        )}
      </div>

      {/* Footer Info Bar */}
      <div className="px-3 py-2 border-t border-black/5 dark:border-white/5 bg-black/[0.02] dark:bg-white/[0.02] shrink-0 text-[11px] text-zinc-500 dark:text-zinc-400 select-none">
        <div className="flex items-center justify-between mb-1">
          <div className="flex items-center space-x-1.5 min-w-0">
            <span className="font-semibold text-zinc-700 dark:text-zinc-200 truncate">
              {formatAppDisplayName(clip.source_app)}
            </span>
            <span className="text-zinc-400">•</span>
            <span className="truncate max-w-[200px]" title={clip.window_title}>
              {clip.window_title}
            </span>
          </div>

          <span className="shrink-0 text-[10px] text-zinc-400 font-medium">
            {clip.content_type === 'image' && imgMeta ? (
              `${imgMeta.width} × ${imgMeta.height} px • ${formatFileSize(imgMeta.file_size)}`
            ) : (
              `${charCount} chars • ${lineCount} ${lineCount === 1 ? 'line' : 'lines'}`
            )}
          </span>
        </div>

        <div className="flex items-center justify-between text-[10px] text-zinc-400/80 pt-1 border-t border-black/[0.04] dark:border-white/[0.04]">
          <span>↵ Paste</span>
          <span>Space Copy</span>
          <span>Ctrl+P Pin</span>
          <span>Del Delete</span>
          <span>Esc Hide</span>
        </div>
      </div>
    </div>
  );
};

/* --- Sub-renderers for different content types --- */

const ImagePreview: React.FC<{
  content: string;
  meta: ImageMetadata | null;
  onOpenImage: () => void;
  onCopy: () => void;
}> = ({ content, meta, onOpenImage, onCopy }) => {
  const imgSrc = convertFileSrc(content);
  const fileName = content.split('\\').pop()?.split('/').pop() ?? 'screenshot.png';

  const formatFileSize = (bytes: number) => {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    return `${(bytes / (1024 * 1024)).toFixed(2)} MB`;
  };

  return (
    <div className="flex flex-col space-y-3">
      {/* Header Info Banner */}
      <div className="flex items-center justify-between p-2.5 rounded-lg border border-black/10 dark:border-white/10 bg-white/50 dark:bg-black/20 text-xs">
        <div className="flex items-center space-x-2 min-w-0">
          <ImageIcon className="w-4 h-4 text-cyan-500 shrink-0" />
          <span className="font-medium text-zinc-800 dark:text-zinc-200 truncate" title={fileName}>
            {fileName}
          </span>
          {meta && (
            <span className="px-1.5 py-0.5 rounded text-[10px] font-code bg-black/5 dark:bg-white/10 text-zinc-500 dark:text-zinc-400 shrink-0">
              {meta.width} × {meta.height}
            </span>
          )}
        </div>

        <div className="flex items-center space-x-2 shrink-0">
          {meta && (
            <span className="text-[11px] text-zinc-400 dark:text-zinc-500">
              {formatFileSize(meta.file_size)}
            </span>
          )}
          <button
            onClick={onOpenImage}
            className="flex items-center space-x-1 text-xs text-[#0067C0] dark:text-[#4CC2FF] hover:underline cursor-pointer"
            title="Open in Default Photo Viewer"
          >
            <span>Open</span>
            <ExternalLink className="w-3 h-3" />
          </button>
        </div>
      </div>

      {/* Main Image Frame */}
      <div
        onClick={onCopy}
        className="relative group rounded-xl border border-black/10 dark:border-white/10 overflow-hidden bg-black/5 dark:bg-black/40 flex items-center justify-center p-2 cursor-pointer shadow-inner min-h-[180px] max-h-[300px]"
        title="Click to copy image"
      >
        <img
          src={imgSrc}
          alt="Clipboard screenshot"
          className="max-h-[280px] max-w-full rounded-lg object-contain shadow-md transition-transform group-hover:scale-[1.01]"
        />
        <div className="absolute inset-0 bg-black/30 opacity-0 group-hover:opacity-100 transition-opacity flex items-center justify-center rounded-xl pointer-events-none">
          <span className="px-3 py-1.5 rounded-lg bg-black/75 text-white text-xs font-medium shadow-lg flex items-center space-x-1.5">
            <Copy className="w-3.5 h-3.5" />
            <span>Click to Copy Image</span>
          </span>
        </div>
      </div>
    </div>
  );
};

const ColorPreview: React.FC<{ content: string; onCopyColor: () => void }> = ({ content, onCopyColor }) => {
  const trimmed = content.trim();
  const hexInfo = parseHexColor(trimmed);

  return (
    <div className="flex flex-col space-y-4">
      {/* Color Swatch Card */}
      <div className="flex items-center space-x-4 p-4 rounded-xl border border-black/10 dark:border-white/10 bg-white/50 dark:bg-black/20 shadow-xs">
        <div
          className="w-16 h-16 rounded-xl border border-black/15 dark:border-white/20 shadow-md shrink-0 transition-transform hover:scale-105"
          style={{ backgroundColor: trimmed }}
        />
        <div className="flex flex-col space-y-1 flex-1">
          <div className="flex items-center justify-between">
            <span className="font-code text-base font-bold text-zinc-900 dark:text-zinc-100">
              {trimmed}
            </span>
            <button
              onClick={onCopyColor}
              className="flex items-center space-x-1 px-2 py-0.5 text-xs rounded-md bg-black/5 dark:bg-white/10 hover:bg-black/10 dark:hover:bg-white/20 text-zinc-700 dark:text-zinc-300 transition-colors cursor-pointer"
            >
              <Copy className="w-3 h-3" />
              <span>Copy</span>
            </button>
          </div>
          {hexInfo && (
            <>
              <span className="font-code text-xs text-zinc-500 dark:text-zinc-400">{hexInfo.rgb}</span>
              <span className="font-code text-xs text-zinc-500 dark:text-zinc-400">{hexInfo.hsl}</span>
            </>
          )}
        </div>
      </div>
    </div>
  );
};

const UrlPreview: React.FC<{ content: string; onOpen: () => void }> = ({ content, onOpen }) => {
  const trimmed = content.trim();
  let domain = '';
  try {
    const url = new URL(trimmed);
    domain = url.hostname;
  } catch {
    domain = trimmed;
  }

  return (
    <div className="flex flex-col space-y-3">
      <div className="p-4 rounded-xl border border-black/10 dark:border-white/10 bg-white/50 dark:bg-black/20 shadow-xs flex flex-col space-y-2">
        <div className="flex items-center justify-between">
          <span className="text-xs font-semibold uppercase tracking-wider text-blue-600 dark:text-blue-400 flex items-center space-x-1">
            <Link className="w-3 h-3" />
            <span>{domain}</span>
          </span>
          <button
            onClick={onOpen}
            className="flex items-center space-x-1 text-xs text-[#0067C0] dark:text-[#4CC2FF] hover:underline cursor-pointer"
          >
            <span>Open Link</span>
            <ExternalLink className="w-3 h-3" />
          </button>
        </div>
        <p className="font-code text-xs text-zinc-700 dark:text-zinc-300 break-all select-text">
          {trimmed}
        </p>
      </div>
    </div>
  );
};

const JsonPreview: React.FC<{ content: string }> = ({ content }) => {
  const formatted = tryFormatJson(content) ?? content;
  const lines = formatted.split('\n');

  return (
    <div className="rounded-lg p-3 bg-black/5 dark:bg-black/30 border border-black/5 dark:border-white/5 font-code text-[11px] leading-relaxed overflow-x-auto select-text">
      <table className="w-full border-collapse">
        <tbody>
          {lines.map((line, idx) => (
            <tr key={idx} className="hover:bg-black/5 dark:hover:bg-white/5">
              <td className="pr-3 text-right text-zinc-400 dark:text-zinc-600 select-none w-8 align-top text-[10px]">
                {idx + 1}
              </td>
              <td className="text-zinc-800 dark:text-zinc-200 whitespace-pre">{line}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
};

const CodePreview: React.FC<{ lines: string[] }> = ({ lines }) => {
  return (
    <div className="rounded-lg p-3 bg-black/5 dark:bg-black/30 border border-black/5 dark:border-white/5 font-code text-[11px] leading-relaxed overflow-x-auto select-text">
      <table className="w-full border-collapse">
        <tbody>
          {lines.map((line, idx) => (
            <tr key={idx} className="hover:bg-black/5 dark:hover:bg-white/5">
              <td className="pr-3 text-right text-zinc-400 dark:text-zinc-600 select-none w-8 align-top text-[10px]">
                {idx + 1}
              </td>
              <td className="text-zinc-800 dark:text-zinc-200 whitespace-pre">{line}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
};

const TextPreview: React.FC<{ content: string }> = ({ content }) => {
  return (
    <div className="text-xs text-zinc-800 dark:text-zinc-200 leading-relaxed whitespace-pre-wrap select-text p-1">
      {content}
    </div>
  );
};
