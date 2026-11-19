import React from 'react';
import { AppFilterItem } from '../types';
import { Pin, Layers } from 'lucide-react';
import { formatAppDisplayName } from '../utils/formatters';

interface AppFilterBarProps {
  apps: AppFilterItem[];
  selectedFilter: string; // 'All', 'Pinned', or app name
  totalCount: number;
  pinnedCount: number;
  onSelectFilter: (filter: string) => void;
}

export const AppFilterBar: React.FC<AppFilterBarProps> = ({
  apps,
  selectedFilter,
  totalCount,
  pinnedCount,
  onSelectFilter,
}) => {
  return (
    <div className="flex items-center space-x-1.5 px-3 py-2 border-b border-black/5 dark:border-white/5 bg-black/[0.02] dark:bg-white/[0.02] overflow-x-auto text-xs no-scrollbar select-none">
      {/* All Tab */}
      <button
        onClick={() => onSelectFilter('All')}
        className={`flex items-center space-x-1.5 px-2.5 py-1 rounded-full transition-all cursor-pointer whitespace-nowrap shrink-0 ${
          selectedFilter === 'All'
            ? 'bg-[#0067C0] text-white dark:bg-[#4CC2FF] dark:text-zinc-950 font-medium shadow-xs'
            : 'text-zinc-600 dark:text-zinc-400 hover:bg-black/5 dark:hover:bg-white/5'
        }`}
      >
        <Layers className="w-3 h-3" />
        <span>All</span>
        <span
          className={`px-1 py-0.2 rounded-full text-[10px] ${
            selectedFilter === 'All'
              ? 'bg-white/20 dark:bg-black/20 text-white dark:text-zinc-950 font-bold'
              : 'bg-black/5 dark:bg-white/10 text-zinc-500 dark:text-zinc-400'
          }`}
        >
          {totalCount}
        </span>
      </button>

      {/* Pinned Tab */}
      <button
        onClick={() => onSelectFilter('Pinned')}
        className={`flex items-center space-x-1.5 px-2.5 py-1 rounded-full transition-all cursor-pointer whitespace-nowrap shrink-0 ${
          selectedFilter === 'Pinned'
            ? 'bg-[#0067C0] text-white dark:bg-[#4CC2FF] dark:text-zinc-950 font-medium shadow-xs'
            : 'text-zinc-600 dark:text-zinc-400 hover:bg-black/5 dark:hover:bg-white/5'
        }`}
      >
        <Pin className="w-3 h-3" />
        <span>Pinned</span>
        {pinnedCount > 0 && (
          <span
            className={`px-1 py-0.2 rounded-full text-[10px] ${
              selectedFilter === 'Pinned'
                ? 'bg-white/20 dark:bg-black/20 text-white dark:text-zinc-950 font-bold'
                : 'bg-black/5 dark:bg-white/10 text-zinc-500 dark:text-zinc-400'
            }`}
          >
            {pinnedCount}
          </span>
        )}
      </button>

      {/* Dynamic App Filter Pills */}
      {apps.map((item) => {
        const isSelected = selectedFilter === item.source_app;
        const displayName = formatAppDisplayName(item.source_app);

        return (
          <button
            key={item.source_app}
            onClick={() => onSelectFilter(item.source_app)}
            className={`flex items-center space-x-1.5 px-2.5 py-1 rounded-full transition-all cursor-pointer whitespace-nowrap shrink-0 ${
              isSelected
                ? 'bg-[#0067C0] text-white dark:bg-[#4CC2FF] dark:text-zinc-950 font-medium shadow-xs'
                : 'text-zinc-600 dark:text-zinc-400 hover:bg-black/5 dark:hover:bg-white/5'
            }`}
            title={item.source_app}
          >
            <span className="truncate max-w-[120px]">{displayName}</span>
            <span
              className={`px-1 py-0.2 rounded-full text-[10px] ${
                isSelected
                  ? 'bg-white/20 dark:bg-black/20 text-white dark:text-zinc-950 font-bold'
                  : 'bg-black/5 dark:bg-white/10 text-zinc-500 dark:text-zinc-400'
              }`}
            >
              {item.count}
            </span>
          </button>
        );
      })}
    </div>
  );
};
