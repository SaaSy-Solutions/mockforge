import React, { useState } from 'react';
import { cn } from '../../utils/cn';
import { ChevronIcon, Icons } from './IconSystem';
import { Button } from './DesignSystem';

// Enhanced column definition with responsive options
export interface ResponsiveTableColumn<T = unknown> {
  key: string;
  label: string;
  render?: (value: unknown, row: T) => React.ReactNode;
  sortable?: boolean;
  width?: string;
  minWidth?: string;

  // Responsive options
  hideOnMobile?: boolean;
  showOnHover?: boolean;
  priority?: 'high' | 'medium' | 'low'; // Determines which columns to show first on mobile
  mobileLabel?: string; // Alternative label for mobile view
}

export interface ResponsiveTableProps<T = unknown> {
  columns: ResponsiveTableColumn<T>[];
  data: T[];
  className?: string;
  onRowClick?: (row: T) => void;

  // Responsive options
  stackOnMobile?: boolean; // Stack rows as cards on mobile
  showExpandButton?: boolean; // Show expand/collapse for mobile cards
  sortable?: boolean;
  searchable?: boolean;
  searchPlaceholder?: string;

  // Loading and empty states
  isLoading?: boolean;
  emptyMessage?: string;
}

// Mobile card view for a single row
function MobileCard<T>({
  row,
  columns,
  onRowClick,
  showExpandButton = true
}: {
  row: T;
  columns: ResponsiveTableColumn<T>[];
  onRowClick?: (row: T) => void;
  showExpandButton?: boolean;
}) {
  const [isExpanded, setIsExpanded] = useState(false);

  // Prioritize columns: high priority shown first, then medium, then low
  const prioritizedColumns = columns
    .filter(col => !col.hideOnMobile)
    .sort((a, b) => {
      const priorityOrder = { high: 0, medium: 1, low: 2 };
      const aPriority = priorityOrder[a.priority || 'medium'];
      const bPriority = priorityOrder[b.priority || 'medium'];
      return aPriority - bPriority;
    });

  const highPriorityColumns = prioritizedColumns.filter(col => col.priority === 'high');
  const otherColumns = prioritizedColumns.filter(col => col.priority !== 'high');

  return (
    <div className={cn(
      'bg-card border border-gray-200 dark:border-gray-800 rounded-lg p-4 space-y-3',
      'table-row-hover spring-in animate-fade-in-up',
      onRowClick && 'cursor-pointer'
    )}
    onClick={() => onRowClick?.(row)}>

      {/* High priority info - always visible */}
      <div className="space-y-2">
        {highPriorityColumns.map((column) => {
          const value = (row as Record<string, unknown>)[column.key];
          const displayValue = column.render ? column.render(value, row) : value;

          return (
            <div key={column.key} className="flex items-center justify-between">
              <span className="text-sm font-medium text-gray-600 dark:text-gray-400">
                {column.mobileLabel || column.label}
              </span>
              <span className="text-base text-gray-900 dark:text-gray-100 font-medium">
                {displayValue}
              </span>
            </div>
          );
        })}
      </div>

      {/* Expandable section for additional details */}
      {otherColumns.length > 0 && showExpandButton && (
        <>
          <div className="divider-subtle"></div>

          <button
            className="flex items-center justify-between w-full text-sm text-gray-600 dark:text-gray-400 hover:text-gray-900 dark:hover:text-gray-100 transition-colors"
            onClick={(e) => {
              e.stopPropagation();
              setIsExpanded(!isExpanded);
            }}
          >
            <span>
              {isExpanded ? 'Show less' : `Show ${otherColumns.length} more details`}
            </span>
            <ChevronIcon direction={isExpanded ? 'up' : 'down'} size="sm" />
          </button>

          {isExpanded && (
            <div className="space-y-2 animate-fade-in-up">
              {otherColumns.map((column) => {
                const value = (row as Record<string, unknown>)[column.key];
                const displayValue = column.render ? column.render(value, row) : value;

                return (
                  <div key={column.key} className="flex items-center justify-between">
                    <span className="text-xs font-medium text-gray-600 dark:text-gray-400">
                      {column.mobileLabel || column.label}
                    </span>
                    <span className="text-sm text-gray-600 dark:text-gray-400">
                      {displayValue}
                    </span>
                  </div>
                );
              })}
            </div>
          )}
        </>
      )}
    </div>
  );
}

// Desktop table view
function DesktopTable<T>({
  columns,
  data,
  onRowClick,
  sortable = false
}: {
  columns: ResponsiveTableColumn<T>[];
  data: T[];
  onRowClick?: (row: T) => void;
  sortable?: boolean;
}) {
  const [sortColumn, setSortColumn] = useState<string | null>(null);
  const [sortDirection, setSortDirection] = useState<'asc' | 'desc'>('asc');

  const handleSort = (columnKey: string) => {
    if (!sortable) return;

    if (sortColumn === columnKey) {
      setSortDirection(sortDirection === 'asc' ? 'desc' : 'asc');
    } else {
      setSortColumn(columnKey);
      setSortDirection('asc');
    }
  };

  const sortedData = React.useMemo(() => {
    if (!sortColumn) return data;

    return [...data].sort((a, b) => {
      const aValue = (a as Record<string, unknown>)[sortColumn];
      const bValue = (b as Record<string, unknown>)[sortColumn];

      if (aValue < bValue) return sortDirection === 'asc' ? -1 : 1;
      if (aValue > bValue) return sortDirection === 'asc' ? 1 : -1;
      return 0;
    });
  }, [data, sortColumn, sortDirection]);

  return (
    <div className="overflow-x-auto custom-scrollbar">
      <table className="min-w-full divide-y divide-gray-200 dark:divide-gray-700">
        <thead className="bg-gray-50 dark:bg-gray-800">
          <tr>
            {columns.map((column) => (
              <th
                key={column.key}
                className={cn(
                  'px-6 py-3 text-left text-xs font-medium text-tertiary uppercase tracking-wider',
                  column.width && `w-[${column.width}]`,
                  column.minWidth && `min-w-[${column.minWidth}]`,
                  sortable && column.sortable && 'cursor-pointer hover:text-primary transition-colors'
                )}
                style={{
                  width: column.width,
                  minWidth: column.minWidth
                }}
                onClick={() => column.sortable && handleSort(column.key)}
              >
                <div className="flex items-center gap-2">
                  {column.label}
                  {sortable && column.sortable && (
                    <div className="flex flex-col">
                      <ChevronIcon
                        direction="up"
                        size="xs"
                        className={cn(
                          'transition-opacity',
                          sortColumn === column.key && sortDirection === 'asc'
                            ? 'opacity-100' : 'opacity-30'
                        )}
                      />
                      <ChevronIcon
                        direction="down"
                        size="xs"
                        className={cn(
                          'transition-opacity -mt-1',
                          sortColumn === column.key && sortDirection === 'desc'
                            ? 'opacity-100' : 'opacity-30'
                        )}
                      />
                    </div>
                  )}
                </div>
              </th>
            ))}
          </tr>
        </thead>
        <tbody className="bg-background divide-y divide-gray-200 dark:divide-gray-700">
          {sortedData.map((row, index) => (
            <tr
              key={index}
              className={cn(
                'table-row-hover animate-stagger-in',
                onRowClick && 'cursor-pointer'
              )}
              style={{ animationDelay: `${index * 50}ms` }}
              onClick={() => onRowClick?.(row)}
            >
              {columns.map((column) => {
                const value = (row as Record<string, unknown>)[column.key];
                const displayValue = column.render ? column.render(value, row) : value;

                return (
                  <td
                    key={column.key}
                    className="px-6 py-4 whitespace-nowrap text-base text-gray-900 dark:text-gray-100"
                  >
                    {displayValue}
                  </td>
                );
              })}
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

// Main responsive table component
export function ResponsiveTable<T>({
  columns,
  data,
  className,
  onRowClick,
  stackOnMobile = true,
  showExpandButton = true,
  sortable = false,
  searchable = false,
  searchPlaceholder = "Search...",
  isLoading = false,
  emptyMessage = "No data available"
}: ResponsiveTableProps<T>) {
  const [searchQuery, setSearchQuery] = useState('');

  // Filter data based on search
  const filteredData = React.useMemo(() => {
    if (!searchable || !searchQuery.trim()) return data;

    return data.filter(row => {
      return columns.some(column => {
        const value = (row as Record<string, unknown>)[column.key];
        return String(value).toLowerCase().includes(searchQuery.toLowerCase());
      });
    });
  }, [data, searchQuery, columns, searchable]);

  if (isLoading) {
    return (
      <div className="space-y-4">
        {[...Array(3)].map((_, i) => (
          <div key={i} className="animate-pulse">
            <div className="h-16 bg-gray-200 dark:bg-gray-700 rounded-lg"></div>
          </div>
        ))}
      </div>
    );
  }

  if (filteredData.length === 0) {
    return (
      <div className="text-center py-12">
        <div className="text-xl font-bold text-gray-600 dark:text-gray-400 mb-2">
          {searchQuery ? 'No matching results' : emptyMessage}
        </div>
        {searchQuery && (
          <p className="text-base text-gray-600 dark:text-gray-400">
            Try adjusting your search terms
          </p>
        )}
      </div>
    );
  }

  return (
    <div className={cn('space-y-4', className)}>
      {/* Search bar */}
      {searchable && (
        <div className="flex items-center gap-3">
          <div className="relative flex-1 max-w-sm">
            <input
              type="text"
              placeholder={searchPlaceholder}
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              className="w-full pl-10 pr-4 py-2 border border-gray-300 dark:border-gray-600 rounded-lg text-base bg-background focus:ring-2 focus:ring-brand/20 focus:border-brand transition-colors"
            />
            <Icons.Search className="absolute left-3 top-2.5 h-4 w-4 text-gray-600 dark:text-gray-400" />
          </div>

          {searchQuery && (
            <Button
              variant="secondary"
              size="sm"
              onClick={() => setSearchQuery('')}
              className="flex items-center gap-2"
            >
              <Icons.Close className="h-4 w-4" />
              Clear
            </Button>
          )}
        </div>
      )}

      {/* Mobile view: Stack as cards */}
      <div className={cn(
        stackOnMobile ? 'md:hidden' : 'hidden',
        'space-y-3'
      )}>
        {filteredData.map((row, index) => (
          <MobileCard
            key={index}
            row={row}
            columns={columns}
            onRowClick={onRowClick}
            showExpandButton={showExpandButton}
          />
        ))}
      </div>

      {/* Desktop view: Traditional table */}
      <div className={cn(
        stackOnMobile ? 'hidden md:block' : 'block'
      )}>
        <DesktopTable
          columns={columns}
          data={filteredData}
          onRowClick={onRowClick}
          sortable={sortable}
        />
      </div>

      {/* Results summary */}
      <div className="flex items-center justify-between text-sm text-gray-600 dark:text-gray-400">
        <span>
          Showing {filteredData.length} of {data.length} {data.length === 1 ? 'item' : 'items'}
        </span>

        {searchQuery && (
          <span>
            Filtered by "{searchQuery}"
          </span>
        )}
      </div>
    </div>
  );
}
