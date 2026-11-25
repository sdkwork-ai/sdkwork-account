export interface PageInfo {
  mode: 'offset' | 'cursor';
  page?: number;
  pageSize?: number;
  /** Total rows across all pages. */
  totalItems?: string;
  totalPages?: number;
  hasMore?: boolean;
  nextCursor?: string | null;
}
