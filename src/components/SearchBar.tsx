import React, { useRef, useEffect } from 'react';
import { Search, X } from 'lucide-react';

interface SearchBarProps {
  query: string;
  onChange: (val: string) => void;
  onClear: () => void;
}

export const SearchBar: React.FC<SearchBarProps> = ({ query, onChange, onClear }) => {
  const inputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    // Focus search bar on mount
    inputRef.current?.focus();
  }, []);

  return (
    <div className="relative flex items-center px-4 py-3 border-b border-black/10 dark:border-white/10 bg-white/40 dark:bg-zinc-900/40">
      <Search className="w-4 h-4 text-zinc-400 dark:text-zinc-500 mr-2.5 shrink-0" />
      <input
        ref={inputRef}
        type="text"
        value={query}
        onChange={(e) => onChange(e.target.value)}
        placeholder="Search clipboard, app name, or window title..."
        className="w-full bg-transparent text-sm text-zinc-900 dark:text-zinc-100 placeholder-zinc-400 dark:placeholder-zinc-500 outline-none font-normal"
      />
      {query ? (
        <button
          onClick={() => {
            onClear();
            inputRef.current?.focus();
          }}
          className="p-1 rounded-md text-zinc-400 hover:text-zinc-600 dark:hover:text-zinc-200 hover:bg-black/5 dark:hover:bg-white/5 transition-colors cursor-pointer mr-2 shrink-0"
          title="Clear search"
        >
          <X className="w-3.5 h-3.5" />
        </button>
      ) : null}
      <kbd className="hidden sm:inline-flex items-center px-1.5 py-0.5 text-[10px] font-medium text-zinc-400 dark:text-zinc-500 bg-black/5 dark:bg-white/5 border border-black/5 dark:border-white/10 rounded-md shrink-0">
        Esc
      </kbd>
    </div>
  );
};
