import { useState, useEffect, useRef, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { ClipItem, AppFilterItem } from './types';
import { SearchBar } from './components/SearchBar';
import { AppFilterBar } from './components/AppFilterBar';
import { ClipListItem } from './components/ClipListItem';
import { InspectorPreview } from './components/InspectorPreview';
import { ClipboardList } from 'lucide-react';
import './App.css';

export default function App() {
  const [clips, setClips] = useState<ClipItem[]>([]);
  const [appSummaries, setAppSummaries] = useState<AppFilterItem[]>([]);
  const [searchQuery, setSearchQuery] = useState('');
  const [selectedFilter, setSelectedFilter] = useState('All');
  const [selectedIndex, setSelectedIndex] = useState(0);

  const listContainerRef = useRef<HTMLDivElement>(null);
  const itemRefs = useRef<(HTMLDivElement | null)[]>([]);

  // Deduplicate clips helper by id and content_hash
  const deduplicateClips = useCallback((items: ClipItem[]): ClipItem[] => {
    const seenIds = new Set<number>();
    const seenHashes = new Set<string>();
    const unique: ClipItem[] = [];
    for (const item of items) {
      if (!seenIds.has(item.id) && !seenHashes.has(item.content_hash)) {
        seenIds.add(item.id);
        seenHashes.add(item.content_hash);
        unique.push(item);
      }
    }
    return unique;
  }, []);

  // Fetch clips from SQLite backend
  const loadClips = useCallback(async () => {
    try {
      const isPinned = selectedFilter === 'Pinned';
      const appFilter = selectedFilter !== 'All' && selectedFilter !== 'Pinned' ? selectedFilter : null;

      const results = await invoke<ClipItem[]>('get_clips', {
        query: searchQuery ? searchQuery : null,
        filterApp: appFilter,
        filterType: null,
        pinnedOnly: isPinned,
        limit: 100,
      });

      const uniqueResults = deduplicateClips(results);
      setClips(uniqueResults);
      setSelectedIndex((prev) => (uniqueResults.length === 0 ? 0 : Math.min(prev, uniqueResults.length - 1)));
    } catch (err) {
      console.error('Failed to load clips:', err);
    }
  }, [searchQuery, selectedFilter, deduplicateClips]);

  // Fetch app statistics for the filter pills
  const loadAppSummaries = useCallback(async () => {
    try {
      const summaries = await invoke<AppFilterItem[]>('get_apps_summary');
      setAppSummaries(summaries);
    } catch (err) {
      console.error('Failed to load app summaries:', err);
    }
  }, []);

  // Initial load
  useEffect(() => {
    loadClips();
    loadAppSummaries();
  }, [loadClips, loadAppSummaries]);

  // Listen to background clipboard update events
  useEffect(() => {
    let unlistenClipUpdate: (() => void) | undefined;
    let unlistenClipsCleared: (() => void) | undefined;

    const setupListeners = async () => {
      unlistenClipUpdate = await listen<ClipItem>('clipboard-updated', (event) => {
        const updatedClip = event.payload;
        if (updatedClip) {
          setClips((prev) => {
            const remaining = prev.filter(
              (c) => c.id !== updatedClip.id && c.content_hash !== updatedClip.content_hash
            );
            const matchesFilter =
              selectedFilter === 'All' ||
              (selectedFilter === 'Pinned' && updatedClip.is_pinned) ||
              selectedFilter === updatedClip.source_app;

            const nextList = matchesFilter ? [updatedClip, ...remaining] : remaining;
            return deduplicateClips(nextList);
          });
        }
        loadClips();
        loadAppSummaries();
      });

      unlistenClipsCleared = await listen('clips-cleared', () => {
        loadClips();
        loadAppSummaries();
      });
    };

    setupListeners();

    return () => {
      if (unlistenClipUpdate) unlistenClipUpdate();
      if (unlistenClipsCleared) unlistenClipsCleared();
    };
  }, [loadClips, loadAppSummaries, selectedFilter, deduplicateClips]);

  // Selected clip
  const selectedClip = clips[selectedIndex] ?? null;

  // Auto-scroll selected item into view
  useEffect(() => {
    if (itemRefs.current[selectedIndex]) {
      itemRefs.current[selectedIndex]?.scrollIntoView({
        block: 'nearest',
        behavior: 'smooth',
      });
    }
  }, [selectedIndex]);

  // Actions
  const handlePaste = async (clip: ClipItem) => {
    try {
      await invoke('paste_clip', { id: clip.id });
    } catch (err) {
      console.error('Failed to paste clip:', err);
    }
  };

  const handleCopyOnly = async (clip: ClipItem) => {
    try {
      await invoke('copy_clip_only', { id: clip.id });
    } catch (err) {
      console.error('Failed to copy clip:', err);
    }
  };

  const handleTogglePin = async (clip: ClipItem) => {
    try {
      await invoke('toggle_pin_clip', { id: clip.id });
      loadClips();
    } catch (err) {
      console.error('Failed to toggle pin:', err);
    }
  };

  const handleDelete = async (clip: ClipItem) => {
    try {
      await invoke('delete_clip', { id: clip.id });
      loadClips();
      loadAppSummaries();
    } catch (err) {
      console.error('Failed to delete clip:', err);
    }
  };

  const handleHideWindow = async () => {
    try {
      await invoke('hide_window');
    } catch (err) {
      console.error('Failed to hide window:', err);
    }
  };

  // Keyboard navigation & shortcuts
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        e.preventDefault();
        handleHideWindow();
        return;
      }

      if (clips.length === 0) return;

      if (e.key === 'ArrowDown') {
        e.preventDefault();
        setSelectedIndex((prev) => Math.min(prev + 1, clips.length - 1));
      } else if (e.key === 'ArrowUp') {
        e.preventDefault();
        setSelectedIndex((prev) => Math.max(prev - 1, 0));
      } else if (e.key === 'Enter') {
        e.preventDefault();
        if (selectedClip) {
          handlePaste(selectedClip);
        }
      } else if (e.key === 'Delete') {
        // Delete item if not actively in an input with text selection
        if (selectedClip && document.activeElement?.tagName !== 'INPUT') {
          e.preventDefault();
          handleDelete(selectedClip);
        }
      } else if (e.ctrlKey && (e.key === 'p' || e.key === 'P')) {
        e.preventDefault();
        if (selectedClip) {
          handleTogglePin(selectedClip);
        }
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [clips, selectedClip, selectedIndex]);

  const totalClipsCount = clips.length;
  const pinnedClipsCount = clips.filter((c) => c.is_pinned).length;

  return (
    <div className="flex items-center justify-center w-screen h-screen p-3 box-border bg-transparent select-none">
      {/* Fluent Spotlight Window Shell */}
      <div className="flex flex-col w-full h-full max-w-[740px] max-h-[480px] rounded-2xl overflow-hidden border border-black/10 dark:border-white/10 shadow-[0_20px_50px_rgba(0,0,0,0.3)] bg-white/75 dark:bg-zinc-900/70 backdrop-blur-2xl transition-all">
        {/* Top Search Bar */}
        <SearchBar
          query={searchQuery}
          onChange={(q) => {
            setSearchQuery(q);
            setSelectedIndex(0);
          }}
          onClear={() => {
            setSearchQuery('');
            setSelectedIndex(0);
          }}
        />

        {/* App Context Filter Bar */}
        <AppFilterBar
          apps={appSummaries}
          selectedFilter={selectedFilter}
          totalCount={totalClipsCount}
          pinnedCount={pinnedClipsCount}
          onSelectFilter={(f) => {
            setSelectedFilter(f);
            setSelectedIndex(0);
          }}
        />

        {/* Split-Pane Body */}
        <div className="flex flex-1 min-h-0 overflow-hidden">
          {/* Left Pane (Clip List, ~40% width) */}
          <div
            ref={listContainerRef}
            className="w-5/12 min-w-[280px] border-r border-black/5 dark:border-white/5 overflow-y-auto no-scrollbar py-1"
          >
            {clips.length === 0 ? (
              <div className="flex flex-col items-center justify-center h-full p-6 text-center text-zinc-400 dark:text-zinc-600">
                <ClipboardList className="w-8 h-8 mb-2 stroke-[1.25] opacity-40" />
                <p className="text-xs font-medium">No clips found</p>
                <p className="text-[11px] text-zinc-400/70 mt-0.5">
                  {searchQuery ? 'Try a different search term' : 'Copied text will automatically appear here'}
                </p>
              </div>
            ) : (
              clips.map((clip, idx) => (
                <div
                  key={clip.id}
                  ref={(el) => {
                    itemRefs.current[idx] = el;
                  }}
                >
                  <ClipListItem
                    clip={clip}
                    isSelected={idx === selectedIndex}
                    onSelect={() => setSelectedIndex(idx)}
                    onTogglePin={(e) => {
                      e.stopPropagation();
                      handleTogglePin(clip);
                    }}
                    onDoubleClick={() => handlePaste(clip)}
                  />
                </div>
              ))
            )}
          </div>

          {/* Right Pane (Inspector Preview, ~60% width) */}
          <div className="flex-1 min-w-0 h-full overflow-hidden">
            <InspectorPreview
              clip={selectedClip}
              onPaste={() => selectedClip && handlePaste(selectedClip)}
              onCopy={() => selectedClip && handleCopyOnly(selectedClip)}
              onTogglePin={() => selectedClip && handleTogglePin(selectedClip)}
              onDelete={() => selectedClip && handleDelete(selectedClip)}
            />
          </div>
        </div>
      </div>
    </div>
  );
}
