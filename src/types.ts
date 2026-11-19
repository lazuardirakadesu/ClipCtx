export type ContentType = 'text' | 'code' | 'json' | 'url' | 'color' | 'image';

export interface ClipItem {
  id: number;
  content: string;
  content_hash: string;
  content_type: ContentType;
  source_app: string;
  window_title: string;
  is_pinned: boolean;
  created_at: number;
  last_used_at: number;
}

export interface AppFilterItem {
  source_app: string;
  count: number;
}
