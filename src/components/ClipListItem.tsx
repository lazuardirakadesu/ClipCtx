import React from 'react';
import { ClipItem } from '../types';
import { Code, Braces, Palette, Link, FileText, Pin } from 'lucide-react';
import { formatRelativeTime, formatAppDisplayName } from '../utils/formatters';

interface ClipListItemProps {
  clip: ClipItem;
  isSelected: boolean;
  onSelect: () => void;
  onTogglePin: (e: React.MouseEvent) => void;
  onDoubleClick: () => void;
}

export const ClipListItem: React.FC<ClipListItemProps> = ({
  clip,
  isSelected,
  onSelect,
  onTogglePin,
  onDoubleClick,
}) => {
  const getIcon = () => {
    switch (clip.content_type) {
      case 'code':
        return <Code className="w-3.5 h-3.5 text-amber-500 shrink-0" />;
      case 'json':
        return <Braces className="w-3.5 h-3.5 text-emerald-500 shrink-0" />;
      case 'color':
        return <Palette className="w-3.5 h-3.5 text-fuchsia-500 shrink-0" />;
      case 'url':
        return <Link className="w-3.5 h-3.5 text-blue-500 shrink-0" />;
      default:
        return <FileText className="w-3.5 h-3.5 text-zinc-400 dark:text-zinc-500 shrink-0" />;
    }
  };

  const isColor = clip.content_type === 'color';
  const cleanApp = formatAppDisplayName(clip.source_app);

  return (
    <div
      onClick={onSelect}
      onDoubleClick={onDoubleClick}
      className={`group relative flex flex-col px-3 py-2.5 my-1 mx-2 rounded-lg cursor-pointer transition-all duration-150 select-none ${
        isSelected
          ? 'bg-black/[0.06] dark:bg-white/[0.08] shadow-xs'
          : 'hover:bg-black/[0.03] dark:hover:bg-white/[0.04]'
      }`}
    >
      {/* Active Accent Pill */}
      {isSelected && (
        <div className="absolute left-0 top-1/2 -translate-y-1/2 w-1 h-5 rounded-full bg-[#0067C0] dark:bg-[#4CC2FF]" />
      )}

      {/* Header Row: Icon + Source App + Pin + Time */}
      <div className="flex items-center justify-between text-[11px] text-zinc-500 dark:text-zinc-400 mb-1">
        <div className="flex items-center space-x-1.5 min-w-0">
          {getIcon()}
          <span className="font-medium truncate max-w-[110px] text-zinc-700 dark:text-zinc-300">
            {cleanApp}
          </span>
        </div>

        <div className="flex items-center space-x-1 shrink-0">
          <button
            onClick={onTogglePin}
            className={`p-0.5 rounded transition-colors ${
              clip.is_pinned
                ? 'text-amber-500 dark:text-amber-400'
                : 'text-zinc-300 dark:text-zinc-600 opacity-0 group-hover:opacity-100 hover:text-zinc-500 dark:hover:text-zinc-400'
            }`}
            title={clip.is_pinned ? 'Unpin clip' : 'Pin clip'}
          >
            <Pin className={`w-3 h-3 ${clip.is_pinned ? 'fill-amber-500 dark:fill-amber-400' : ''}`} />
          </button>
          <span className="text-[10px] text-zinc-400 dark:text-zinc-500">
            {formatRelativeTime(clip.last_used_at)}
          </span>
        </div>
      </div>

      {/* Content Preview */}
      <div className="flex items-center space-x-2">
        {isColor && (
          <div
            className="w-3.5 h-3.5 rounded-full border border-black/10 dark:border-white/20 shrink-0 shadow-xs"
            style={{ backgroundColor: clip.content.trim() }}
          />
        )}
        <p
          className={`text-xs text-zinc-800 dark:text-zinc-200 line-clamp-2 break-all ${
            clip.content_type === 'code' || clip.content_type === 'json' ? 'font-code text-[11px]' : ''
          }`}
        >
          {clip.content}
        </p>
      </div>
    </div>
  );
};
